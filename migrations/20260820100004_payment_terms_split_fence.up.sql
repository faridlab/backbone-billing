-- Company fence posture for the payment-terms master (ADR-0014: split).
--
-- `payment_terms` / `payment_term_lines` carry company_id NULL = a GLOBAL template (the seeded
-- 30-net, 2/10-etc. defaults every tenant picks from). A plain strict fence — the predicate the
-- declaration would naively derive — makes those NULL-company rows invisible to every scoped
-- connection, so a tenant's term list comes back empty and the picker shows nothing. The split
-- policy instead admits global rows on READ (USING) while keeping writes own-only (WITH CHECK):
-- a global template can be created or retired only via the owner/admin path (migrations,
-- seeders, a platform caller on a role that bypasses RLS), never by a tenant forging
-- `company_id = NULL` on a scoped connection. This is the exact shape corporate's
-- `currency_exchanges` landed (20260819100000) for the same reason.
--
-- The lines table denormalizes company_id from its header so the child fence holds on its own
-- (a scoped session can never read or write another tenant's slice of a term).
--
-- A future `metaphor schema generate` re-deriving a plain-strict stanza for either table would
-- be a REGRESSION: if the generator ever emits for them, restore the split shape.
-- Requires the app to connect as a non-superuser role; migrations/seeders run as
-- the owner and bypass.

ALTER TABLE billing.payment_terms ENABLE ROW LEVEL SECURITY;
ALTER TABLE billing.payment_terms FORCE  ROW LEVEL SECURITY;
DROP POLICY IF EXISTS payment_terms_company_isolation ON billing.payment_terms;
CREATE POLICY payment_terms_company_isolation ON billing.payment_terms
    FOR ALL
    USING      (company_id IS NULL
                OR company_id = NULLIF(current_setting('app.company_id', true), '')::uuid)
    WITH CHECK (company_id = NULLIF(current_setting('app.company_id', true), '')::uuid);

ALTER TABLE billing.payment_term_lines ENABLE ROW LEVEL SECURITY;
ALTER TABLE billing.payment_term_lines FORCE  ROW LEVEL SECURITY;
DROP POLICY IF EXISTS payment_term_lines_company_isolation ON billing.payment_term_lines;
CREATE POLICY payment_term_lines_company_isolation ON billing.payment_term_lines
    FOR ALL
    USING      (company_id IS NULL
                OR company_id = NULLIF(current_setting('app.company_id', true), '')::uuid)
    WITH CHECK (company_id = NULLIF(current_setting('app.company_id', true), '')::uuid);
