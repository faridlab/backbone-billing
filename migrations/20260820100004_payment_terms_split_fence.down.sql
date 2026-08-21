-- Reverse the payment-terms split fence (drops the policies, disables RLS).

DROP POLICY IF EXISTS payment_term_lines_company_isolation ON billing.payment_term_lines;
ALTER TABLE billing.payment_term_lines FORCE  ROW LEVEL SECURITY;
ALTER TABLE billing.payment_term_lines DISABLE ROW LEVEL SECURITY;

DROP POLICY IF EXISTS payment_terms_company_isolation ON billing.payment_terms;
ALTER TABLE billing.payment_terms FORCE  ROW LEVEL SECURITY;
ALTER TABLE billing.payment_terms DISABLE ROW LEVEL SECURITY;
