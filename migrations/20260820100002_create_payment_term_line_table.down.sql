-- Down: drop billing.payment_term_lines table
DROP TABLE IF EXISTS billing.payment_term_lines CASCADE;
DROP FUNCTION IF EXISTS billing.payment_term_lines_audit_timestamp() CASCADE;
