//! Payment-term shape validation refusals — every `TermInvalid` code the create path can emit.
//! Pure: no DB.

use rust_decimal::Decimal;
use uuid::Uuid;

use backbone_billing::application::service::term_schedule::{validate_term, NewTermLine};

fn d(s: &str) -> Decimal {
    Decimal::from_str_exact(s).unwrap()
}
fn lines(ls: &[(&str, &str, i32, &str)]) -> Vec<NewTermLine> {
    ls.iter()
        .map(|(value, amount, nb_days, delay_type)| NewTermLine {
            value: (*value).into(),
            value_amount: d(amount),
            nb_days: *nb_days,
            day_of_month: None,
            delay_type: (*delay_type).into(),
            anchor: "invoice_date".into(),
            sequence: 10,
        })
        .collect()
}
/// The minimal ACCEPTED term: one balance slice, no discount block. Each refusal case mutates it.
fn ok_term() -> Vec<NewTermLine> {
    lines(&[("balance", "0", 30, "days")])
}
fn check(
    name: &str,
    ls: Vec<NewTermLine>,
    early: bool,
    pct: Decimal,
    days: i32,
    acct: Option<Uuid>,
    basis: &str,
) -> String {
    validate_term(name, &ls, early, pct, days, acct, basis)
        .unwrap_err()
        .code()
}

#[test]
fn name_and_lines_required() {
    assert_eq!(
        check("", ok_term(), false, Decimal::ZERO, 0, None, "included"),
        "term_name_required"
    );
    assert_eq!(
        check("T", vec![], false, Decimal::ZERO, 0, None, "included"),
        "term_lines_required"
    );
}

#[test]
fn reduced_price_basis_refused() {
    assert_eq!(
        check(
            "T",
            ok_term(),
            false,
            Decimal::ZERO,
            0,
            None,
            "reduced_price"
        ),
        "discount_tax_basis_unsupported"
    );
}

#[test]
fn discount_block_must_be_complete() {
    let acct = Some(Uuid::new_v4());
    // zero/negative or >100 percent
    assert_eq!(
        check("T", ok_term(), true, Decimal::ZERO, 10, acct, "included"),
        "discount_percent_out_of_range"
    );
    assert_eq!(
        check("T", ok_term(), true, d("101"), 10, acct, "included"),
        "discount_percent_out_of_range"
    );
    // non-positive window
    assert_eq!(
        check("T", ok_term(), true, d("2"), 0, acct, "included"),
        "discount_days_out_of_range"
    );
    // no expense account
    assert_eq!(
        check("T", ok_term(), true, d("2"), 10, None, "included"),
        "discount_account_required"
    );
}

#[test]
fn slice_value_rules() {
    // unknown value kind
    assert_eq!(
        check(
            "T",
            lines(&[("annuity", "0", 30, "days")]),
            false,
            Decimal::ZERO,
            0,
            None,
            "included"
        ),
        "term_value_invalid"
    );
    // two balance slices — the first is not last, so that refusal fires first (only one
    // balance may exist and it must be the final line)
    assert_eq!(
        check(
            "T",
            lines(&[("balance", "0", 30, "days"), ("balance", "0", 60, "days")]),
            false,
            Decimal::ZERO,
            0,
            None,
            "included"
        ),
        "term_balance_not_last"
    );
    // balance not last
    assert_eq!(
        check(
            "T",
            lines(&[("balance", "0", 30, "days"), ("fixed", "100", 60, "days")]),
            false,
            Decimal::ZERO,
            0,
            None,
            "included"
        ),
        "term_balance_not_last"
    );
    // percent out of range
    assert_eq!(
        check(
            "T",
            lines(&[("percent", "0", 30, "days")]),
            false,
            Decimal::ZERO,
            0,
            None,
            "included"
        ),
        "term_percent_out_of_range"
    );
    // percent sum beyond 100
    assert_eq!(
        check(
            "T",
            lines(&[
                ("percent", "60", 30, "days"),
                ("percent", "60", 60, "days"),
                ("balance", "0", 60, "days")
            ]),
            false,
            Decimal::ZERO,
            0,
            None,
            "included"
        ),
        "term_percent_exceeds_total"
    );
    // fixed must be positive
    assert_eq!(
        check(
            "T",
            lines(&[("fixed", "0", 30, "days")]),
            false,
            Decimal::ZERO,
            0,
            None,
            "included"
        ),
        "term_fixed_out_of_range"
    );
    // negative day offset
    assert_eq!(
        check(
            "T",
            lines(&[("balance", "0", -1, "days")]),
            false,
            Decimal::ZERO,
            0,
            None,
            "included"
        ),
        "term_days_negative"
    );
}

#[test]
fn day_shaped_delays_need_day_of_month() {
    let mut ls = lines(&[("balance", "0", 0, "day_following_month")]);
    assert_eq!(
        validate_term("T", &ls, false, Decimal::ZERO, 0, None, "included")
            .unwrap_err()
            .code(),
        "term_day_of_month_required"
    );
    ls[0].day_of_month = Some(31);
    validate_term("T", &ls, false, Decimal::ZERO, 0, None, "included").unwrap();
}

#[test]
fn accepted_shapes_pass() {
    // multi-slice with discount block
    let acct = Uuid::new_v4();
    validate_term(
        "2/10 Net 30",
        &lines(&[("balance", "0", 30, "days")]),
        true,
        d("2"),
        10,
        Some(acct),
        "included",
    )
    .unwrap();
    // percent + balance
    validate_term(
        "30/70",
        &lines(&[("percent", "30", 15, "days"), ("balance", "0", 45, "days")]),
        false,
        Decimal::ZERO,
        0,
        None,
        "included",
    )
    .unwrap();
}
