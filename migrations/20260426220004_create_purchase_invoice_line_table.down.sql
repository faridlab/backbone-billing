-- Down: drop billing.purchase_invoice_lines table
DROP TABLE IF EXISTS billing.purchase_invoice_lines CASCADE;
DROP FUNCTION IF EXISTS billing.purchase_invoice_lines_audit_timestamp() CASCADE;
