-- Down: drop billing.invoice_tax_lines table
DROP TABLE IF EXISTS billing.invoice_tax_lines CASCADE;
DROP FUNCTION IF EXISTS billing.invoice_tax_lines_audit_timestamp() CASCADE;
