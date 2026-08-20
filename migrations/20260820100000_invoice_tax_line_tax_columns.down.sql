-- Reverse the template-driven tax overlay columns.
ALTER TABLE billing.invoice_tax_lines
    DROP COLUMN exigibility,
    DROP COLUMN real_account_id,
    DROP COLUMN repartition_line_id,
    DROP COLUMN tax_template_id;

DROP TYPE billing.tax_exigibility;
