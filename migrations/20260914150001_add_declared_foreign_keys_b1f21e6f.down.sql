-- Down: drop the declared foreign keys for billing module
ALTER TABLE IF EXISTS billing.sales_invoice_lines DROP CONSTRAINT IF EXISTS fk_sales_invoice_lines_invoice_id;
ALTER TABLE IF EXISTS billing.purchase_invoice_lines DROP CONSTRAINT IF EXISTS fk_purchase_invoice_lines_invoice_id;
ALTER TABLE IF EXISTS billing.payment_term_lines DROP CONSTRAINT IF EXISTS fk_payment_term_lines_term_id;
