-- Invoice-side payment-term columns: the link + the materialized early-pay-discount decision.
--
-- An invoice may carry a payment term instead of a manual due date; at post time the term's
-- derivation stamps `due_date` (already present) plus this block. The EPD columns are
-- MATERIALIZED at post — a snapshot of the term header's discount decision on the posting date,
-- never recomputed, so a later term edit cannot rewrite history (the same reason the schedule
-- rows are seeded instead of referenced).
--
-- `payment_term_id` and `early_pay_discount_account_id` are logical references (no FK): the
-- account lives in the accounting schema (cross-schema FK refused) and the term is deliberately
-- soft-referenced — retiring a term must not block invoice reads.
-- Idempotent per-column guards: re-running against a partially-migrated DB is safe.

ALTER TABLE billing.sales_invoices
    ADD COLUMN IF NOT EXISTS payment_term_id                    UUID,
    ADD COLUMN IF NOT EXISTS early_pay_discount_percent         NUMERIC(7,4) NOT NULL DEFAULT 0
        CHECK (early_pay_discount_percent >= 0),
    ADD COLUMN IF NOT EXISTS early_pay_discount_deadline        DATE,
    ADD COLUMN IF NOT EXISTS early_pay_discount_account_id     UUID;

ALTER TABLE billing.purchase_invoices
    ADD COLUMN IF NOT EXISTS payment_term_id                    UUID,
    ADD COLUMN IF NOT EXISTS early_pay_discount_percent         NUMERIC(7,4) NOT NULL DEFAULT 0
        CHECK (early_pay_discount_percent >= 0),
    ADD COLUMN IF NOT EXISTS early_pay_discount_deadline        DATE,
    ADD COLUMN IF NOT EXISTS early_pay_discount_account_id     UUID;

CREATE INDEX IF NOT EXISTS idx_sales_invoices_payment_term
    ON billing.sales_invoices (payment_term_id) WHERE payment_term_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_purchase_invoices_payment_term
    ON billing.purchase_invoices (payment_term_id) WHERE payment_term_id IS NOT NULL;
