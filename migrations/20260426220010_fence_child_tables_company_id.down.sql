-- Down: reverse the company RLS fence + company_id column for the three billing child tables.

-- billing.invoice_tax_lines
DROP POLICY IF EXISTS invoice_tax_lines_company_isolation ON billing.invoice_tax_lines;
ALTER TABLE billing.invoice_tax_lines NO FORCE ROW LEVEL SECURITY;
ALTER TABLE billing.invoice_tax_lines DISABLE ROW LEVEL SECURITY;
DROP INDEX IF EXISTS billing.idx_invoice_tax_lines_company_id;
ALTER TABLE billing.invoice_tax_lines DROP COLUMN IF EXISTS company_id;

-- billing.purchase_invoice_lines
DROP POLICY IF EXISTS purchase_invoice_lines_company_isolation ON billing.purchase_invoice_lines;
ALTER TABLE billing.purchase_invoice_lines NO FORCE ROW LEVEL SECURITY;
ALTER TABLE billing.purchase_invoice_lines DISABLE ROW LEVEL SECURITY;
DROP INDEX IF EXISTS billing.idx_purchase_invoice_lines_company_id;
ALTER TABLE billing.purchase_invoice_lines DROP COLUMN IF EXISTS company_id;

-- billing.sales_invoice_lines
DROP POLICY IF EXISTS sales_invoice_lines_company_isolation ON billing.sales_invoice_lines;
ALTER TABLE billing.sales_invoice_lines NO FORCE ROW LEVEL SECURITY;
ALTER TABLE billing.sales_invoice_lines DISABLE ROW LEVEL SECURITY;
DROP INDEX IF EXISTS billing.idx_sales_invoice_lines_company_id;
ALTER TABLE billing.sales_invoice_lines DROP COLUMN IF EXISTS company_id;
