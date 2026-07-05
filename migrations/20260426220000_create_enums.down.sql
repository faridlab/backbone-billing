-- Down: drop enum types for billing module
DROP TYPE IF EXISTS gl_posting_state CASCADE;
DROP TYPE IF EXISTS invoice_status CASCADE;
DROP TYPE IF EXISTS payment_schedule_status CASCADE;
DROP TYPE IF EXISTS tax_basis CASCADE;
DROP TYPE IF EXISTS invoice_kind CASCADE;
