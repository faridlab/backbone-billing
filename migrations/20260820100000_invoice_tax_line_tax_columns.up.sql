-- Tax overlay columns for template-driven computation (backbone-tax integration).
--
-- When an invoice line carries a tax template, billing computes the document's tax through
-- backbone-tax's document engine and stores the routing the engine resolved alongside each
-- overlay row: which template produced it, which repartition split, and — for cash-basis
-- (on_payment) templates — the REAL account the amount flips to as payments reconcile
-- (`account_id` holds the transition account meanwhile, so the journal legs at post time need
-- no translation). Explicitly supplied tax lines (the pre-existing path) keep
-- exigibility 'on_invoice' and NULL ids — behavior unchanged.

CREATE TYPE billing.tax_exigibility AS ENUM ('on_invoice', 'on_payment');

ALTER TABLE billing.invoice_tax_lines
    ADD COLUMN tax_template_id     uuid,
    ADD COLUMN repartition_line_id uuid,
    ADD COLUMN real_account_id     uuid,
    ADD COLUMN exigibility         billing.tax_exigibility NOT NULL DEFAULT 'on_invoice';

-- Additive fence restatement (ADR-0014 strict posture, idempotent refresh — no semantic
-- change): re-declare the company isolation policy on the widened table so the posture
-- travels with every schema touch, not only with the migration that created it.
ALTER TABLE billing.invoice_tax_lines ENABLE ROW LEVEL SECURITY;
ALTER TABLE billing.invoice_tax_lines FORCE  ROW LEVEL SECURITY;
DROP POLICY IF EXISTS invoice_tax_lines_company_isolation ON billing.invoice_tax_lines;
CREATE POLICY invoice_tax_lines_company_isolation ON billing.invoice_tax_lines
    FOR ALL
    USING      (company_id = NULLIF(current_setting('app.company_id', true), '')::uuid)
    WITH CHECK (company_id = NULLIF(current_setting('app.company_id', true), '')::uuid);
