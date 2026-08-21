-- Reverse the invoice-side payment-term columns.

DROP INDEX IF EXISTS billing.idx_purchase_invoices_payment_term;
DROP INDEX IF EXISTS billing.idx_sales_invoices_payment_term;

ALTER TABLE billing.purchase_invoices
    DROP COLUMN IF EXISTS early_pay_discount_account_id,
    DROP COLUMN IF EXISTS early_pay_discount_deadline,
    DROP COLUMN IF EXISTS early_pay_discount_percent,
    DROP COLUMN IF EXISTS payment_term_id;

ALTER TABLE billing.sales_invoices
    DROP COLUMN IF EXISTS early_pay_discount_account_id,
    DROP COLUMN IF EXISTS early_pay_discount_deadline,
    DROP COLUMN IF EXISTS early_pay_discount_percent,
    DROP COLUMN IF EXISTS payment_term_id;
