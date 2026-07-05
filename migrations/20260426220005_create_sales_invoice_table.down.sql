-- Down: drop billing.sales_invoices table
DROP TABLE IF EXISTS billing.sales_invoices CASCADE;
DROP FUNCTION IF EXISTS billing.sales_invoices_audit_timestamp() CASCADE;
