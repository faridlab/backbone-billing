//! Global payment-term fence suite: the SPLIT policy (read admits global templates, write stays
//! own-only) proven at BOTH layers — raw SQL as a restricted (non-BYPASSRLS) role, and the service
//! running on a restricted pool (the regression that catches a service-level read that quietly
//! bypasses the bind and sees nothing, or worse, someone else's rows).
//! Requires DATABASE_URL (:5433/backbone_billing) reachable as a superuser (to mint the probe role
//! and seed fixtures) — the app-role probes connect as that restricted role.

use rust_decimal::Decimal;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use backbone_billing::application::service::billing_write_service::BillingWriteService;
use backbone_billing::application::service::term_schedule::NewTermLine;

fn d(s: &str) -> Decimal {
    Decimal::from_str_exact(s).unwrap()
}
fn uq(p: &str) -> String {
    format!("{p}-{}", &Uuid::new_v4().simple().to_string()[..8])
}
async fn pool() -> PgPool {
    let url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgresql://postgres:postgres@localhost:5433/backbone_billing".to_string()
    });
    PgPool::connect(&url).await.expect("connect DB")
}

/// The restricted probe role: NOSUPERUSER NOBYPASSRLS — the only posture under which RLS policies
/// actually bind. Minted idempotently by the superuser test pool.
const PROBE_ROLE: &str = "billing_fence_probe";
const PROBE_PASSWORD: &str = "probe";

async fn restricted_pool(admin: &PgPool, db: &str) -> PgPool {
    // Serialize mint + grants across the parallel tests (shared-catalog DDL does not tolerate
    // concurrent GRANTs), then tolerate losing the race — the winner made the same role.
    sqlx::query("SELECT pg_advisory_lock(hashtext('billing_fence_probe'))")
        .execute(admin)
        .await
        .expect("take probe mint lock");
    let _ = sqlx::query(&format!(
        "CREATE ROLE {PROBE_ROLE} LOGIN PASSWORD '{PROBE_PASSWORD}' \
           NOSUPERUSER NOBYPASSRLS NOCREATEDB NOCREATEROLE"
    ))
    .execute(admin)
    .await;
    // One statement per execute (a multi-command string is not a legal prepared statement).
    for grant in [
        format!("GRANT CONNECT ON DATABASE {db} TO {PROBE_ROLE}"),
        format!("GRANT USAGE ON SCHEMA billing TO {PROBE_ROLE}"),
        format!("GRANT SELECT, INSERT, UPDATE ON TABLE billing.payment_terms TO {PROBE_ROLE}"),
        format!("GRANT SELECT, INSERT, UPDATE ON TABLE billing.payment_term_lines TO {PROBE_ROLE}"),
    ] {
        sqlx::query(&grant)
            .execute(admin)
            .await
            .expect("grant probe role");
    }
    sqlx::query("SELECT pg_advisory_unlock(hashtext('billing_fence_probe'))")
        .execute(admin)
        .await
        .expect("release probe mint lock");
    let url = format!("postgresql://{PROBE_ROLE}:{PROBE_PASSWORD}@localhost:5433/{db}");
    PgPool::connect(&url)
        .await
        .expect("connect as restricted probe")
}

/// A global template (header + one balance line) as the owner path would seed it (superuser
/// only — the whole point of the fence).
async fn seed_global_term(admin: &PgPool, name: &str) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO billing.payment_terms \
             (id, company_id, name, sequence, status, early_discount, discount_percent, \
              discount_days, discount_account_id, discount_tax_basis) \
         VALUES ($1, NULL, $2, 99, 'active', false, 0, 0, NULL, 'included')",
    )
    .bind(id)
    .bind(name)
    .execute(admin)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO billing.payment_term_lines \
             (id, term_id, company_id, value, value_amount, nb_days, day_of_month, \
              delay_type, anchor, sequence) \
         VALUES ($1, $2, NULL, 'balance', 0, 30, NULL, 'days', 'invoice_date', 10)",
    )
    .bind(Uuid::new_v4())
    .bind(id)
    .execute(admin)
    .await
    .unwrap();
    id
}

fn balance_line() -> NewTermLine {
    NewTermLine {
        value: "balance".into(),
        value_amount: Decimal::ZERO,
        nb_days: 30,
        day_of_month: None,
        delay_type: "days".into(),
        anchor: "invoice_date".into(),
        sequence: 10,
    }
}

// FN-1: an UNSCOPED restricted session reads the global templates but no tenant rows — the shared
// master stays visible; company rows fail closed without `app.company_id`.
#[tokio::test]
async fn unscoped_restricted_sees_globals_only() {
    let admin = pool().await;
    let company = Uuid::new_v4();
    let global = seed_global_term(&admin, &uq("GLOBAL")).await;
    let w = BillingWriteService::new(admin.clone());
    w.create_payment_term(
        company,
        &uq("OWN"),
        None,
        10,
        &[balance_line()],
        false,
        Decimal::ZERO,
        0,
        None,
        "included",
    )
    .await
    .unwrap();

    let probe = restricted_pool(&admin, "backbone_billing").await;
    // Pollution-immune shape: other tests may seed their own globals in parallel, so assert on
    // the invariant — every row an unscoped session sees is a global (NULL company), ours among
    // them, and no tenant row (ours or anyone's) leaks.
    let seen: Vec<(Uuid, Option<Uuid>)> =
        sqlx::query_as("SELECT id, company_id FROM billing.payment_terms")
            .fetch_all(&probe)
            .await
            .unwrap();
    assert!(
        seen.iter().any(|(id, c)| *id == global && c.is_none()),
        "global template stays readable unscoped"
    );
    assert!(
        seen.iter().all(|(_, c)| c.is_none()),
        "no tenant row leaks to an unscoped session"
    );
}

// FN-2: a scoped restricted session sees its own rows AND the globals — never another company's.
#[tokio::test]
async fn scoped_session_sees_own_and_globals() {
    let admin = pool().await;
    let a = Uuid::new_v4();
    let b = Uuid::new_v4();
    let w = BillingWriteService::new(admin.clone());
    let term_a = w
        .create_payment_term(
            a,
            &uq("A"),
            None,
            10,
            &[balance_line()],
            false,
            Decimal::ZERO,
            0,
            None,
            "included",
        )
        .await
        .unwrap();
    let term_b = w
        .create_payment_term(
            b,
            &uq("B"),
            None,
            10,
            &[balance_line()],
            false,
            Decimal::ZERO,
            0,
            None,
            "included",
        )
        .await
        .unwrap();
    let global = seed_global_term(&admin, &uq("GLOBAL")).await;

    let probe = restricted_pool(&admin, "backbone_billing").await;
    let mut tx = probe.begin().await.unwrap();
    sqlx::query("SELECT set_config('app.company_id', $1, true)")
        .bind(a.to_string())
        .execute(&mut *tx)
        .await
        .unwrap();
    let seen: Vec<Uuid> = sqlx::query_scalar("SELECT id FROM billing.payment_terms")
        .fetch_all(&mut *tx)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    assert!(seen.contains(&term_a), "own row visible");
    assert!(seen.contains(&global), "global template visible");
    assert!(!seen.contains(&term_b), "company B's row is invisible to A");
}

// FN-3: the write fence — a scoped session cannot forge a global row, cannot write another
// company's id, and cannot mutate a global template. All three are WITH CHECK refusals.
#[tokio::test]
async fn scoped_writes_cannot_forge_or_touch_globals() {
    let admin = pool().await;
    let a = Uuid::new_v4();
    let b = Uuid::new_v4();
    let global = seed_global_term(&admin, &uq("GLOBAL")).await;
    let probe = restricted_pool(&admin, "backbone_billing").await;

    let forge_global = sqlx::query(
        "INSERT INTO billing.payment_terms \
             (id, company_id, name, sequence, status, early_discount, discount_percent, \
              discount_days, discount_account_id, discount_tax_basis) \
         VALUES ($1, NULL, 'forged', 1, 'active', false, 0, 0, NULL, 'included')",
    )
    .bind(Uuid::new_v4());
    let forge_other = sqlx::query(
        "INSERT INTO billing.payment_terms \
             (id, company_id, name, sequence, status, early_discount, discount_percent, \
              discount_days, discount_account_id, discount_tax_basis) \
         VALUES ($1, $2, 'forged', 1, 'active', false, 0, 0, NULL, 'included')",
    )
    .bind(Uuid::new_v4())
    .bind(b);
    let touch_global =
        sqlx::query("UPDATE billing.payment_terms SET name='hijacked' WHERE id=$1").bind(global);

    for q in [forge_global, forge_other, touch_global] {
        let mut tx = probe.begin().await.unwrap();
        sqlx::query("SELECT set_config('app.company_id', $1, true)")
            .bind(a.to_string())
            .execute(&mut *tx)
            .await
            .unwrap();
        let res = q.execute(&mut *tx).await;
        assert!(
            res.is_err(),
            "the fence must refuse this write (got {res:?})"
        );
        tx.rollback().await.unwrap();
    }
}

// FN-4 (the service regression): the SERVICE running on the restricted pool — the exact posture
// production runs under — must still list own+global, and must treat another company's term and a
// global template as unusable (preview/list semantics) or unmodifiable (status flip).
#[tokio::test]
async fn service_on_restricted_pool_list_and_status() {
    let admin = pool().await;
    let a = Uuid::new_v4();
    let b = Uuid::new_v4();
    let global = seed_global_term(&admin, &uq("GLOBAL")).await;
    let w_admin = BillingWriteService::new(admin.clone());
    let term_a = w_admin
        .create_payment_term(
            a,
            &uq("A"),
            None,
            10,
            &[balance_line()],
            false,
            Decimal::ZERO,
            0,
            None,
            "included",
        )
        .await
        .unwrap();
    let term_b = w_admin
        .create_payment_term(
            b,
            &uq("B"),
            None,
            10,
            &[balance_line()],
            false,
            Decimal::ZERO,
            0,
            None,
            "included",
        )
        .await
        .unwrap();

    let probe = restricted_pool(&admin, "backbone_billing").await;
    let w = BillingWriteService::new(probe.clone());

    // list: own + global, NOT B's.
    let listed = w.list_payment_terms(a).await.unwrap();
    let ids: Vec<Uuid> = listed.iter().map(|t| t.id).collect();
    assert!(ids.contains(&term_a) && ids.contains(&global) && !ids.contains(&term_b));

    // preview: own term derives; the global template derives; B's term is TermNotFound.
    w.preview_payment_term(
        a,
        term_a,
        chrono::NaiveDate::from_ymd_opt(2026, 7, 5).unwrap(),
        d("1000"),
    )
    .await
    .unwrap();
    w.preview_payment_term(
        a,
        global,
        chrono::NaiveDate::from_ymd_opt(2026, 7, 5).unwrap(),
        d("1000"),
    )
    .await
    .unwrap();
    let err = w
        .preview_payment_term(
            a,
            term_b,
            chrono::NaiveDate::from_ymd_opt(2026, 7, 5).unwrap(),
            d("1000"),
        )
        .await
        .unwrap_err();
    assert_eq!(err.code(), "term_not_found");

    // status flip: own works; a global template is not modifiable by a tenant (the WITH CHECK
    // refuses the UPDATE — surfaced as an error, and the template provably stays active).
    w.set_payment_term_status(a, term_a, "inactive")
        .await
        .unwrap();
    assert!(w
        .set_payment_term_status(a, global, "inactive")
        .await
        .is_err());
    let (status,): (String,) =
        sqlx::query_as("SELECT status::text FROM billing.payment_terms WHERE id=$1")
            .bind(global)
            .fetch_one(&admin)
            .await
            .unwrap();
    assert_eq!(status, "active");

    // create: the service path stamps the caller's company — the restricted pool confirms the
    // write fence holds under the real role.
    w.create_payment_term(
        a,
        &uq("A2"),
        None,
        10,
        &[balance_line()],
        false,
        Decimal::ZERO,
        0,
        None,
        "included",
    )
    .await
    .unwrap();
}
