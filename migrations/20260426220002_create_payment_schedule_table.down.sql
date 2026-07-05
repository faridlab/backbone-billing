-- Down: drop billing.payment_schedules table
DROP TABLE IF EXISTS billing.payment_schedules CASCADE;
DROP FUNCTION IF EXISTS billing.payment_schedules_audit_timestamp() CASCADE;
