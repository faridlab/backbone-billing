-- Down: drop billing.sales_invoice_lines table
DROP TABLE IF EXISTS billing.sales_invoice_lines CASCADE;
DROP FUNCTION IF EXISTS billing.sales_invoice_lines_audit_timestamp() CASCADE;
