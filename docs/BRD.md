# BRD — backbone-billing

> Business Requirements & Rules. Tier 2 · Financials. Date: 2026-07-05. Pairs with
> `docs/business-flows/golden-cases.md`.

## Documents
Sales Invoice (+lines; AR) · Purchase Invoice (+lines; A/P) · InvoiceTaxLine (removable tax overlay,
logical ref) · PaymentSchedule (installment due dates, logical ref).

## Business rules
**BR-1 (server-side money).** `net_amount = money(qty·unit_price)`; `net_total = Σ net_amount`. Tax
totals come from the supplied overlay by basis: `output`, `input`, `withholding`. Sales
`grand_total = net_total + Σoutput`; Purchase `grand_total = net_total + Σinput − Σwithholding` (the
A/P owed). 2dp half-up.

**BR-2 (non-empty / non-negative).** ≥1 line; no negative qty/price/tax. → `empty_document` /
`negative_amount`.

**BR-3 (unique numbers).** Invoice numbers unique (soft-delete aware). → `duplicate_number`.

**BR-4 (region-neutral base — localization-standard §1).** The invoice tables carry NO tax columns;
PPN/PPh live only in `InvoiceTaxLine` (logical `invoice_ref`, no DB FK). Drop the overlay and
invoices still compute + post clean (net only). `basis` distinguishes output/input/withholding.

**BR-5 (one balanced post — ADR-001).** On post, billing assembles net lines + overlay tax into ONE
`AccountingPost` and refuses to emit unless `Σdebit = Σcredit` (→ `unbalanced_post`). Sales:
`Dr A/R [customer] · Cr Revenue (per income account) · Cr PPN Output`. Purchase: `Dr Expense (per
account) · Dr PPN Input · Cr A/P [supplier] · Cr PPh Payable`. The receivable/payable line carries
the party (subledger aging).

**BR-6 (idempotent + recoverable posting).** `source_id = invoice id`; a re-post reuses the recorded
ack (no double journal). A rejected post leaves `posting_state=failed`, `status=draft`, no journal —
retryable. Only IDR is supported end-to-end for now (→ `unsupported_currency`).

**BR-7 (A/P seam — ADR-002).** A posted Purchase Invoice emits `PurchaseInvoicePosted{source_po_id,
billed_lines}`; an ACL routes it to buying's `mark_billed`, advancing `billed_qty` and completing the
3-way match. Billing holds no normal Cargo dependency on buying.

**BR-8 (schedules, not settlement).** `PaymentSchedule` records installment due dates + amounts;
`paid_amount`/`status` are advanced by `backbone-payments`. Billing never settles.

## Events
`SalesInvoicePosted`, `PurchaseInvoicePosted`, `InvoiceCancelled`. (Consumed downstream:
`PurchaseInvoicePosted` by buying; both by a faktur-pajak / loyalty overlay.)

## Deferred (with reason)
Settlement/reconciliation (backbone-payments), aging/dunning read-models, deferred revenue &
subscriptions, credit notes / reversal posts, tax computation (backbone-tax), e-Faktur numbering,
multi-currency/FX, POS.
