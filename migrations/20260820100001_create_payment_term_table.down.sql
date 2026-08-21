-- Down: drop billing.payment_terms table
DROP TABLE IF EXISTS billing.payment_terms CASCADE;
DROP FUNCTION IF EXISTS billing.payment_terms_audit_timestamp() CASCADE;
