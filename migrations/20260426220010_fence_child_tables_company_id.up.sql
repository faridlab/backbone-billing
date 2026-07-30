-- Migration: add company_id + ENABLE/FORCE RLS fence to the unfenced billing child tables
--            (sales_invoice_lines, purchase_invoice_lines, invoice_tax_lines)
-- Hand-written (ADR-0010 Decision A). Follows the ADR-0008 invariant #1: the policy uses
-- NULLIF(current_setting('app.company_id', true), '')::uuid so an unset var fails CLOSED
-- (zero rows) instead of erroring on a NULL cast.
--
-- Why no hard FK to organization.companies: every other company_id column in this module
-- (sales_invoices, purchase_invoices, payment_schedules) is a LOGICAL FK only — declared
-- @exclude_from_foreign_key_check in the schema YAML and emitted as a bare `UUID` column
-- with no DB constraint. We keep that convention here for consistency; cross-module
-- hard-FKs are explicitly avoided (modules own their own schema).
--
-- REVERSIBILITY (council 2026-07-26): this migration has NO .down.sql — the company_id RLS fence
-- is intentionally one-way (rolling it back would drop the tenant column + policy child rows now
-- depend on). The create migrations (...20001/20004/20006) were folded to emit company_id at birth,
-- so on a FRESH DB this .up is a redundant-but-idempotent guard; on a DB that ran the older creates
-- (no company_id), this .up back-fills it. sqlx tracks migrations by version, not content, so
-- editing these files does NOT re-run them — to reconcile a DB that ran a prior revision, re-apply
-- the fence by hand or recreate the schema from the current migrations.

-- =============================================================================
-- 1) billing.sales_invoice_lines  (parent: billing.sales_invoices via invoice_id)
-- =============================================================================
ALTER TABLE billing.sales_invoice_lines ADD COLUMN IF NOT EXISTS company_id UUID;

UPDATE billing.sales_invoice_lines l
SET company_id = (SELECT s.company_id FROM billing.sales_invoices s WHERE s.id = l.invoice_id);

-- Fail loud if any line is orphaned (header missing) — the FK to sales_invoices is the
-- invariant that makes this backfill total.
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM billing.sales_invoice_lines WHERE company_id IS NULL) THEN
        RAISE EXCEPTION 'sales_invoice_lines rows with unresolved company_id (orphaned invoice_id?) — fix the data before re-running';
    END IF;
END $$;

ALTER TABLE billing.sales_invoice_lines ALTER COLUMN company_id SET NOT NULL;
CREATE INDEX IF NOT EXISTS idx_sales_invoice_lines_company_id ON billing.sales_invoice_lines (company_id);

ALTER TABLE billing.sales_invoice_lines ENABLE ROW LEVEL SECURITY;
ALTER TABLE billing.sales_invoice_lines FORCE  ROW LEVEL SECURITY;
DROP POLICY IF EXISTS sales_invoice_lines_company_isolation ON billing.sales_invoice_lines;
CREATE POLICY sales_invoice_lines_company_isolation ON billing.sales_invoice_lines
    FOR ALL
    USING      (company_id = NULLIF(current_setting('app.company_id', true), '')::uuid)
    WITH CHECK (company_id = NULLIF(current_setting('app.company_id', true), '')::uuid);

-- =============================================================================
-- 2) billing.purchase_invoice_lines  (parent: billing.purchase_invoices via invoice_id)
-- =============================================================================
ALTER TABLE billing.purchase_invoice_lines ADD COLUMN IF NOT EXISTS company_id UUID;

UPDATE billing.purchase_invoice_lines l
SET company_id = (SELECT p.company_id FROM billing.purchase_invoices p WHERE p.id = l.invoice_id);

DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM billing.purchase_invoice_lines WHERE company_id IS NULL) THEN
        RAISE EXCEPTION 'purchase_invoice_lines rows with unresolved company_id (orphaned invoice_id?) — fix the data before re-running';
    END IF;
END $$;

ALTER TABLE billing.purchase_invoice_lines ALTER COLUMN company_id SET NOT NULL;
CREATE INDEX IF NOT EXISTS idx_purchase_invoice_lines_company_id ON billing.purchase_invoice_lines (company_id);

ALTER TABLE billing.purchase_invoice_lines ENABLE ROW LEVEL SECURITY;
ALTER TABLE billing.purchase_invoice_lines FORCE  ROW LEVEL SECURITY;
DROP POLICY IF EXISTS purchase_invoice_lines_company_isolation ON billing.purchase_invoice_lines;
CREATE POLICY purchase_invoice_lines_company_isolation ON billing.purchase_invoice_lines
    FOR ALL
    USING      (company_id = NULLIF(current_setting('app.company_id', true), '')::uuid)
    WITH CHECK (company_id = NULLIF(current_setting('app.company_id', true), '')::uuid);

-- =============================================================================
-- 3) billing.invoice_tax_lines  (TRICKY CASE)
--    Parent is polymorphic: invoice_ref points at sales_invoices (when invoice_kind='sales')
--    OR purchase_invoices (when invoice_kind='purchase'). Backfill resolves the right
--    parent using the disambiguating invoice_kind column on the row itself.
-- =============================================================================
ALTER TABLE billing.invoice_tax_lines ADD COLUMN IF NOT EXISTS company_id UUID;

UPDATE billing.invoice_tax_lines t
SET company_id = CASE t.invoice_kind
    WHEN 'sales'    THEN (SELECT s.company_id FROM billing.sales_invoices    s WHERE s.id = t.invoice_ref)
    WHEN 'purchase' THEN (SELECT p.company_id FROM billing.purchase_invoices p WHERE p.id = t.invoice_ref)
END;

DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM billing.invoice_tax_lines WHERE company_id IS NULL) THEN
        RAISE EXCEPTION 'invoice_tax_lines rows with unresolved company_id (unknown invoice_kind or orphaned invoice_ref?) — fix the data before re-running';
    END IF;
END $$;

ALTER TABLE billing.invoice_tax_lines ALTER COLUMN company_id SET NOT NULL;
CREATE INDEX IF NOT EXISTS idx_invoice_tax_lines_company_id ON billing.invoice_tax_lines (company_id);

ALTER TABLE billing.invoice_tax_lines ENABLE ROW LEVEL SECURITY;
ALTER TABLE billing.invoice_tax_lines FORCE  ROW LEVEL SECURITY;
DROP POLICY IF EXISTS invoice_tax_lines_company_isolation ON billing.invoice_tax_lines;
CREATE POLICY invoice_tax_lines_company_isolation ON billing.invoice_tax_lines
    FOR ALL
    USING      (company_id = NULLIF(current_setting('app.company_id', true), '')::uuid)
    WITH CHECK (company_id = NULLIF(current_setting('app.company_id', true), '')::uuid);
