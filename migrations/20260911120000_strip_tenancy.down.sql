-- Hand-authored (user-owned). Not regenerated.
--
-- Best-effort restore sketch for the tenancy strip (ADR-0029). This is a breaking module
-- release against dev-stage databases: the down re-adds the company_id column as nullable
-- with its plain index and the company isolation policy shape, but restores NO data —
-- rows written after the strip (or after the decorator re-keyed them) carry org_unit_id
-- only. The composing service's tenancy decorator remains the live fence; treat this
-- down as a schema-shape sketch for archaeology, not a usable rollback.
--
-- The former NULL-company global-template rows of payment_terms / payment_term_lines
-- are NOT re-created as NULL here either — under the sketch they read as ordinary
-- unscoped rows until data is restored by hand.

ALTER TABLE billing.sales_invoices       ADD COLUMN IF NOT EXISTS company_id uuid;
ALTER TABLE billing.sales_invoice_lines  ADD COLUMN IF NOT EXISTS company_id uuid;
ALTER TABLE billing.purchase_invoices    ADD COLUMN IF NOT EXISTS company_id uuid;
ALTER TABLE billing.purchase_invoice_lines ADD COLUMN IF NOT EXISTS company_id uuid;
ALTER TABLE billing.invoice_tax_lines    ADD COLUMN IF NOT EXISTS company_id uuid;
ALTER TABLE billing.payment_schedules    ADD COLUMN IF NOT EXISTS company_id uuid;
ALTER TABLE billing.payment_terms        ADD COLUMN IF NOT EXISTS company_id uuid;
ALTER TABLE billing.payment_term_lines   ADD COLUMN IF NOT EXISTS company_id uuid;

CREATE INDEX IF NOT EXISTS idx_sales_invoices_company_id_customer_id_status
    ON billing.sales_invoices (company_id, customer_id, status);
CREATE INDEX IF NOT EXISTS idx_sales_invoice_lines_company_id
    ON billing.sales_invoice_lines (company_id);
CREATE INDEX IF NOT EXISTS idx_purchase_invoices_company_id_supplier_id_status
    ON billing.purchase_invoices (company_id, supplier_id, status);
CREATE INDEX IF NOT EXISTS idx_purchase_invoice_lines_company_id
    ON billing.purchase_invoice_lines (company_id);
CREATE INDEX IF NOT EXISTS idx_invoice_tax_lines_company_id
    ON billing.invoice_tax_lines (company_id);
CREATE INDEX IF NOT EXISTS idx_payment_schedules_company_id_due_date_status
    ON billing.payment_schedules (company_id, due_date, status);
CREATE INDEX IF NOT EXISTS idx_payment_terms_company_id_status
    ON billing.payment_terms (company_id, status);

-- The split-fence shape of the payment-terms policies (NULL rows readable by every
-- tenant) is not reconstructed — the plain own-rows-only policy sketch stands in.
CREATE POLICY sales_invoices_company_isolation ON billing.sales_invoices
    FOR ALL USING (company_id = NULLIF(current_setting('app.company_id', true), '')::uuid)
    WITH CHECK (company_id = NULLIF(current_setting('app.company_id', true), '')::uuid);
CREATE POLICY sales_invoice_lines_company_isolation ON billing.sales_invoice_lines
    FOR ALL USING (company_id = NULLIF(current_setting('app.company_id', true), '')::uuid)
    WITH CHECK (company_id = NULLIF(current_setting('app.company_id', true), '')::uuid);
CREATE POLICY purchase_invoices_company_isolation ON billing.purchase_invoices
    FOR ALL USING (company_id = NULLIF(current_setting('app.company_id', true), '')::uuid)
    WITH CHECK (company_id = NULLIF(current_setting('app.company_id', true), '')::uuid);
CREATE POLICY purchase_invoice_lines_company_isolation ON billing.purchase_invoice_lines
    FOR ALL USING (company_id = NULLIF(current_setting('app.company_id', true), '')::uuid)
    WITH CHECK (company_id = NULLIF(current_setting('app.company_id', true), '')::uuid);
CREATE POLICY invoice_tax_lines_company_isolation ON billing.invoice_tax_lines
    FOR ALL USING (company_id = NULLIF(current_setting('app.company_id', true), '')::uuid)
    WITH CHECK (company_id = NULLIF(current_setting('app.company_id', true), '')::uuid);
CREATE POLICY payment_schedules_company_isolation ON billing.payment_schedules
    FOR ALL USING (company_id = NULLIF(current_setting('app.company_id', true), '')::uuid)
    WITH CHECK (company_id = NULLIF(current_setting('app.company_id', true), '')::uuid);
CREATE POLICY payment_terms_company_isolation ON billing.payment_terms
    FOR ALL USING (company_id = NULLIF(current_setting('app.company_id', true), '')::uuid)
    WITH CHECK (company_id = NULLIF(current_setting('app.company_id', true), '')::uuid);
CREATE POLICY payment_term_lines_company_isolation ON billing.payment_term_lines
    FOR ALL USING (company_id = NULLIF(current_setting('app.company_id', true), '')::uuid)
    WITH CHECK (company_id = NULLIF(current_setting('app.company_id', true), '')::uuid);
