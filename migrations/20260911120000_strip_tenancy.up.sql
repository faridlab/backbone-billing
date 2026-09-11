-- Hand-authored (user-owned). Not regenerated.
--
-- Strip every company-fence artifact from the billing tables (ADR-0029): the module is
-- tenant-agnostic; org scoping is installed by the COMPOSING service's tenancy decorator,
-- never by the module. Dropped here, per table: the company-leading indexes, the
-- <table>_company_isolation RLS policy, and the company_id column itself.
--
-- Ordering guard (the decorator must run FIRST on any database with data): the module
-- never moves tenancy data. A table is safe to strip when EITHER
--   a) it carries org_unit_id with no NULLs — the decorator backfilled it from company_id —
--      or b) it is empty (a fresh database: the earlier chain files created it empty).
-- Otherwise the strip RAISEs, naming the decorator step, rather than dropping a column
-- that still holds the only tenancy key. The file is re-runnable (every drop is IF EXISTS
-- and the tracker has no checksums), so a failed run retries cleanly after the decorator
-- lands.
--
-- RLS enable/force flags are deliberately NOT touched: the decorator owns those now.
--
-- Shared-row note: payment_terms / payment_term_lines formerly carried NULL-company rows
-- under a split fence (NULL = global template every tenant reads, none mutates). The
-- column goes here like any other; a composing service that wants the global-template
-- posture declares these tables ROOT-ANCHORED SHARED (allow_root) in its tenancy.yaml
-- and backfills the former NULL rows onto its root org unit BEFORE this file runs (the
-- guard below accepts a backfilled org_unit_id exactly as for any other table).

DO $$
DECLARE
    t text;
    has_org boolean;
    org_nulls bigint;
    total bigint;
    offenders text := '';
BEGIN
    FOREACH t IN ARRAY ARRAY[
        'sales_invoices', 'sales_invoice_lines',
        'purchase_invoices', 'purchase_invoice_lines',
        'invoice_tax_lines', 'payment_schedules',
        'payment_terms', 'payment_term_lines'
    ]
    LOOP
        IF to_regclass(format('billing.%I', t)) IS NULL THEN
            CONTINUE; -- chain not fully applied on this database; nothing to strip
        END IF;

        SELECT EXISTS (
                   SELECT 1 FROM information_schema.columns
                   WHERE table_schema = 'billing' AND table_name = t AND column_name = 'org_unit_id'
               )
        INTO has_org;

        EXECUTE format('SELECT count(*) FROM billing.%I', t) INTO total;

        IF has_org THEN
            EXECUTE format(
                'SELECT count(*) FROM billing.%I WHERE org_unit_id IS NULL', t)
            INTO org_nulls;
        ELSE
            org_nulls := total; -- no org column: every row's only tenancy key is company_id
        END IF;

        IF has_org AND org_nulls = 0 THEN
            CONTINUE; -- decorator backfilled: safe
        END IF;
        IF total = 0 THEN
            CONTINUE; -- empty table (fresh database): safe
        END IF;
        offenders := offenders || format(' billing.%s (%s rows, %s rows not covered by org_unit_id);', t, total, org_nulls);
    END LOOP;

    IF offenders <> '' THEN
        RAISE EXCEPTION 'refusing to strip company_id — these tables are not yet covered by the tenancy decorator:%. Apply the composing service''s tenancy decorator (it backfills org_unit_id from company_id — NULL-template payment terms map onto the root org unit) and re-run; it is the only step that moves tenancy data.', offenders;
    END IF;
END $$;

-- ── sales_invoices ─────────────────────────────────────────────────────────────
DROP INDEX IF EXISTS billing.idx_sales_invoices_company_id_customer_id_status;
DROP POLICY IF EXISTS sales_invoices_company_isolation ON billing.sales_invoices;
ALTER TABLE billing.sales_invoices DROP COLUMN IF EXISTS company_id;

-- ── sales_invoice_lines ────────────────────────────────────────────────────────
DROP INDEX IF EXISTS billing.idx_sales_invoice_lines_company_id;
DROP POLICY IF EXISTS sales_invoice_lines_company_isolation ON billing.sales_invoice_lines;
ALTER TABLE billing.sales_invoice_lines DROP COLUMN IF EXISTS company_id;

-- ── purchase_invoices ──────────────────────────────────────────────────────────
DROP INDEX IF EXISTS billing.idx_purchase_invoices_company_id_supplier_id_status;
DROP POLICY IF EXISTS purchase_invoices_company_isolation ON billing.purchase_invoices;
ALTER TABLE billing.purchase_invoices DROP COLUMN IF EXISTS company_id;

-- ── purchase_invoice_lines ─────────────────────────────────────────────────────
DROP INDEX IF EXISTS billing.idx_purchase_invoice_lines_company_id;
DROP POLICY IF EXISTS purchase_invoice_lines_company_isolation ON billing.purchase_invoice_lines;
ALTER TABLE billing.purchase_invoice_lines DROP COLUMN IF EXISTS company_id;

-- ── invoice_tax_lines ──────────────────────────────────────────────────────────
DROP INDEX IF EXISTS billing.idx_invoice_tax_lines_company_id;
DROP POLICY IF EXISTS invoice_tax_lines_company_isolation ON billing.invoice_tax_lines;
ALTER TABLE billing.invoice_tax_lines DROP COLUMN IF EXISTS company_id;

-- ── payment_schedules ──────────────────────────────────────────────────────────
DROP INDEX IF EXISTS billing.idx_payment_schedules_company_id_due_date_status;
DROP POLICY IF EXISTS payment_schedules_company_isolation ON billing.payment_schedules;
ALTER TABLE billing.payment_schedules DROP COLUMN IF EXISTS company_id;

-- ── payment_terms ──────────────────────────────────────────────────────────────
DROP INDEX IF EXISTS billing.idx_payment_terms_company_id_status;
DROP POLICY IF EXISTS payment_terms_company_isolation ON billing.payment_terms;
ALTER TABLE billing.payment_terms DROP COLUMN IF EXISTS company_id;

-- ── payment_term_lines ─────────────────────────────────────────────────────────
DROP POLICY IF EXISTS payment_term_lines_company_isolation ON billing.payment_term_lines;
ALTER TABLE billing.payment_term_lines DROP COLUMN IF EXISTS company_id;
