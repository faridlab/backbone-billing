-- Revert the ADR-0014 strict fence re-statement for billing module.
-- The fence predates this migration (ADR-0008-era), so the honest reverse is to
-- re-state the same live policy, not to disarm the tables: a down that disabled RLS
-- would leave company data unfenced — a posture this module never had.

-- Re-state the pre-existing fence for billing.invoice_tax_lines (identical policy; see header).
DROP POLICY IF EXISTS invoice_tax_lines_company_isolation ON billing.invoice_tax_lines;
CREATE POLICY invoice_tax_lines_company_isolation ON billing.invoice_tax_lines
    FOR ALL
    USING      (company_id = NULLIF(current_setting('app.company_id', true), '')::uuid)
    WITH CHECK (company_id = NULLIF(current_setting('app.company_id', true), '')::uuid);

-- Re-state the pre-existing fence for billing.payment_schedules (identical policy; see header).
DROP POLICY IF EXISTS payment_schedules_company_isolation ON billing.payment_schedules;
CREATE POLICY payment_schedules_company_isolation ON billing.payment_schedules
    FOR ALL
    USING      (company_id = NULLIF(current_setting('app.company_id', true), '')::uuid)
    WITH CHECK (company_id = NULLIF(current_setting('app.company_id', true), '')::uuid);

-- Re-state the pre-existing fence for billing.purchase_invoice_lines (identical policy; see header).
DROP POLICY IF EXISTS purchase_invoice_lines_company_isolation ON billing.purchase_invoice_lines;
CREATE POLICY purchase_invoice_lines_company_isolation ON billing.purchase_invoice_lines
    FOR ALL
    USING      (company_id = NULLIF(current_setting('app.company_id', true), '')::uuid)
    WITH CHECK (company_id = NULLIF(current_setting('app.company_id', true), '')::uuid);

-- Re-state the pre-existing fence for billing.purchase_invoices (identical policy; see header).
DROP POLICY IF EXISTS purchase_invoices_company_isolation ON billing.purchase_invoices;
CREATE POLICY purchase_invoices_company_isolation ON billing.purchase_invoices
    FOR ALL
    USING      (company_id = NULLIF(current_setting('app.company_id', true), '')::uuid)
    WITH CHECK (company_id = NULLIF(current_setting('app.company_id', true), '')::uuid);

-- Re-state the pre-existing fence for billing.sales_invoice_lines (identical policy; see header).
DROP POLICY IF EXISTS sales_invoice_lines_company_isolation ON billing.sales_invoice_lines;
CREATE POLICY sales_invoice_lines_company_isolation ON billing.sales_invoice_lines
    FOR ALL
    USING      (company_id = NULLIF(current_setting('app.company_id', true), '')::uuid)
    WITH CHECK (company_id = NULLIF(current_setting('app.company_id', true), '')::uuid);

-- Re-state the pre-existing fence for billing.sales_invoices (identical policy; see header).
DROP POLICY IF EXISTS sales_invoices_company_isolation ON billing.sales_invoices;
CREATE POLICY sales_invoices_company_isolation ON billing.sales_invoices
    FOR ALL
    USING      (company_id = NULLIF(current_setting('app.company_id', true), '')::uuid)
    WITH CHECK (company_id = NULLIF(current_setting('app.company_id', true), '')::uuid);

