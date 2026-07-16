-- Down: remove the company RLS fence for billing module

-- Reverse the company RLS fence for billing.payment_schedules
DROP POLICY IF EXISTS payment_schedules_company_isolation ON billing.payment_schedules;
ALTER TABLE billing.payment_schedules NO FORCE ROW LEVEL SECURITY;
ALTER TABLE billing.payment_schedules DISABLE ROW LEVEL SECURITY;

-- Reverse the company RLS fence for billing.purchase_invoices
DROP POLICY IF EXISTS purchase_invoices_company_isolation ON billing.purchase_invoices;
ALTER TABLE billing.purchase_invoices NO FORCE ROW LEVEL SECURITY;
ALTER TABLE billing.purchase_invoices DISABLE ROW LEVEL SECURITY;

-- Reverse the company RLS fence for billing.sales_invoices
DROP POLICY IF EXISTS sales_invoices_company_isolation ON billing.sales_invoices;
ALTER TABLE billing.sales_invoices NO FORCE ROW LEVEL SECURITY;
ALTER TABLE billing.sales_invoices DISABLE ROW LEVEL SECURITY;

