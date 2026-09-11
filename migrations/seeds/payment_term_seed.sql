-- PaymentTerm Seed Data
-- Hand-authored: the SHARED payment-term templates.
--
-- These rows are the shared defaults every tenant's picker lists alongside its own terms.
-- The module itself is tenant-agnostic (ADR-0029): a template is just a row here. A
-- composing service that wants the old global-template behavior declares these tables
-- ROOT-ANCHORED SHARED (allow_root) in its tenancy.yaml, so the rows it seeds at the root
-- org unit are readable by every tenant while staying writable only on the owner side —
-- seeding runs as the table owner and bypasses the decorator's row-level fence.
--
-- Idempotent: keyed on the fixed row ids (the primary key), so re-seeding never duplicates.

INSERT INTO billing.payment_terms
    (id, name, note, sequence, status, early_discount,
     discount_percent, discount_days, discount_account_id, discount_tax_basis)
VALUES
    -- Immediate — balance on the invoice date.
    ('00000000-0000-0000-0000-0000000000a1', 'Immediate',
     'Due on the invoice date', 5, 'active', false,
     0, 0, NULL, 'included'),
    -- 30 days net — the workhorse.
    ('00000000-0000-0000-0000-0000000000a2', '30 Days Net',
     'Balance due 30 days after the invoice date', 10, 'active', false,
     0, 0, NULL, 'included'),
    -- 2/10 net 30 — the classic early-pay discount: 2% if paid within 10 days, balance at 30.
    -- `discount_account_id` stays NULL in the template: the discount expense account is
    -- chart-specific, so a tenant adopting this template copies it with its own account.
    ('00000000-0000-0000-0000-0000000000a3', '2/10 Net 30',
     '2% discount within 10 days; balance due at 30', 15, 'active', true,
     2.0000, 10, NULL, 'included'),
    -- End of following month (B2B staple): anchor end-of-invoice-month, 1 month on, day 10.
    ('00000000-0000-0000-0000-0000000000a4', '10th Following Month',
     'Due on the 10th of the month following the invoice month', 20, 'active', false,
     0, 0, NULL, 'included')
ON CONFLICT DO NOTHING;

INSERT INTO billing.payment_term_lines
    (id, term_id, value, value_amount, nb_days, day_of_month,
     delay_type, anchor, sequence)
VALUES
    ('00000000-0000-0000-0000-0000000000b1',
     '00000000-0000-0000-0000-0000000000a1',
     'balance', 0, 0, NULL, 'days', 'invoice_date', 10),
    ('00000000-0000-0000-0000-0000000000b2',
     '00000000-0000-0000-0000-0000000000a2',
     'balance', 0, 30, NULL, 'days', 'invoice_date', 10),
    ('00000000-0000-0000-0000-0000000000b3',
     '00000000-0000-0000-0000-0000000000a3',
     'balance', 0, 30, NULL, 'days', 'invoice_date', 10),
    ('00000000-0000-0000-0000-0000000000b4',
     '00000000-0000-0000-0000-0000000000a4',
     'balance', 0, 0, 10, 'day_following_month', 'end_of_invoice_month', 10)
ON CONFLICT DO NOTHING;
