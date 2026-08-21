-- PaymentTerm Seed Data
-- Hand-authored: the GLOBAL payment-term templates (company_id IS NULL).
--
-- These rows are the shared defaults every tenant's picker lists alongside its own terms. They
-- are born NULL-company on purpose and only this owner-side path (seeders run as the table
-- owner and bypass RLS) can create them — the split fence's WITH CHECK refuses a scoped
-- connection forging `company_id = NULL`, so a tenant can read these templates but never
-- mutate or impersonate them.
--
-- Idempotent: keyed on (company_id, name) so re-seeding never duplicates.

INSERT INTO billing.payment_terms
    (id, company_id, name, note, sequence, status, early_discount,
     discount_percent, discount_days, discount_account_id, discount_tax_basis)
VALUES
    -- Immediate — balance on the invoice date.
    ('00000000-0000-0000-0000-0000000000a1', NULL, 'Immediate',
     'Due on the invoice date', 5, 'active', false,
     0, 0, NULL, 'included'),
    -- 30 days net — the workhorse.
    ('00000000-0000-0000-0000-0000000000a2', NULL, '30 Days Net',
     'Balance due 30 days after the invoice date', 10, 'active', false,
     0, 0, NULL, 'included'),
    -- 2/10 net 30 — the classic early-pay discount: 2% if paid within 10 days, balance at 30.
    -- `discount_account_id` stays NULL in the template: the discount expense account is
    -- chart-specific, so a company adopting this template copies it with its own account.
    ('00000000-0000-0000-0000-0000000000a3', NULL, '2/10 Net 30',
     '2% discount within 10 days; balance due at 30', 15, 'active', true,
     2.0000, 10, NULL, 'included'),
    -- End of following month (B2B staple): anchor end-of-invoice-month, 1 month on, day 10.
    ('00000000-0000-0000-0000-0000000000a4', NULL, '10th Following Month',
     'Due on the 10th of the month following the invoice month', 20, 'active', false,
     0, 0, NULL, 'included')
ON CONFLICT DO NOTHING;

INSERT INTO billing.payment_term_lines
    (id, term_id, company_id, value, value_amount, nb_days, day_of_month,
     delay_type, anchor, sequence)
VALUES
    ('00000000-0000-0000-0000-0000000000b1',
     '00000000-0000-0000-0000-0000000000a1', NULL,
     'balance', 0, 0, NULL, 'days', 'invoice_date', 10),
    ('00000000-0000-0000-0000-0000000000b2',
     '00000000-0000-0000-0000-0000000000a2', NULL,
     'balance', 0, 30, NULL, 'days', 'invoice_date', 10),
    ('00000000-0000-0000-0000-0000000000b3',
     '00000000-0000-0000-0000-0000000000a3', NULL,
     'balance', 0, 30, NULL, 'days', 'invoice_date', 10),
    ('00000000-0000-0000-0000-0000000000b4',
     '00000000-0000-0000-0000-0000000000a4', NULL,
     'balance', 0, 0, 10, 'day_following_month', 'end_of_invoice_month', 10)
ON CONFLICT DO NOTHING;
