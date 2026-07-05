-- Down: drop billing.purchase_invoices table
DROP TABLE IF EXISTS billing.purchase_invoices CASCADE;
DROP FUNCTION IF EXISTS billing.purchase_invoices_audit_timestamp() CASCADE;
