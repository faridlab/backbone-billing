# ADR-001: Billing owns AR/AP invoicing; it posts ONE balanced journal per invoice, tax as a removable overlay

**Status**: Accepted — Applied 2026-07-05
**Deciders**: Farid (owner), build session 2026-07-05
**Related**: `docs/erp/financials.md`, `docs/erp/gl-posting-contract.md`,
`docs/erp/localization-standard.md` (§1), accounting ADRs, ADR-002 (A/P seam)

## Context

`backbone-billing` is the AR/AP invoicing context of the Financials pillar. It turns a Sales Invoice
(customer owes us) and a Purchase Invoice (we owe a supplier) into postings on the ledger of record
owned by `backbone-accounting`. It holds no masters — customer/supplier/item/company/account are
logical FKs — and it is the module that retires buying's *simulated* billing leg with a real invoice.
Indonesia tax (PPN/PPh) must not be baked into the base (localization-standard §1).

## Decision

1. **Two documents, one posting shape.** `SalesInvoice(+lines)` and `PurchaseInvoice(+lines)`. Money
   is computed server-side (`net = money(qty·price)`, 2dp half-up); generic CRUD is not mounted on
   the guarded surface. On post, billing assembles net lines + overlay tax into **ONE** balanced
   `AccountingPost` and refuses to emit unless `Σdebit = Σcredit`:
   - **Sales:** `Dr A/R (grand) [customer] · Cr Revenue (net per income account) · Cr PPN Output`.
   - **Purchase:** `Dr Expense (net per account) · Dr PPN Input · Cr A/P (grand) [supplier] · Cr PPh`.
   `grand = net + Σoutput` (sales); `grand = net + Σinput − Σwithholding` (purchase, the A/P owed).
   The receivable/payable line carries the party so accounting can age the subledger.
2. **Tax is a removable overlay (localization-standard §1).** The invoice tables carry **no tax
   columns**. PPN/PPh live only in `InvoiceTaxLine` (logical `invoice_ref`, `basis ∈
   output|input|withholding`), supplied at create time (by `backbone-tax` later). Drop the overlay and
   invoices still compute + post clean (net only) — proven by `tax_free_invoice_posts_net_only`.
3. **Posting is idempotent + recoverable.** `source_id = invoice id` (accounting dedupes on it); a
   re-post reuses the recorded ack. A rejected post leaves `posting_state=failed`, `status=draft`, no
   journal — retryable. Only IDR is supported end-to-end for now (`unsupported_currency` otherwise).
4. **Schedules, not settlement.** `PaymentSchedule` records installment due dates + amounts;
   `paid_amount`/`status` belong to `backbone-payments`. Billing never settles or reconciles.

## Consequences

- The AR/AP math + posting are locked by `tests/billing_golden_cases.rs` (5) and the guarded/failure
  surface by `tests/integrity_probes.rs` (3, incl. rejected-post recovery + non-IDR refusal).
- Billing is independently composable: it needs only a Postgres pool, a `GlPostSink`, and a
  `BillingEventSink`.
- Deferred (per the brief): settlement/reconciliation (payments), aging/dunning, deferred revenue &
  subscriptions, credit notes / reversal posts, tax *computation* (backbone-tax), e-Faktur numbering,
  multi-currency/FX, POS invoicing.
