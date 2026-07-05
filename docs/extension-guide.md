# Extension Guide — backbone-billing

> Public contract per `docs/erp/extension-contract.md`. Stable path:
> `backbone_billing::application::service::*` (the generated `exports/` tree is unwired scaffolding).

## Public surface
**A. Domain events** (`billing_events`, the 3-variant `BillingEvent`): `SalesInvoicePosted`
{invoice_id, company_id, journal_id, post_id, source_so_id, grand_total}, `PurchaseInvoicePosted`
{…, source_po_id, billed_lines[], grand_total}, `InvoiceCancelled` {invoice_id, kind}.

**B. The GL-posting port** (`billing_gl`) — `AccountingPostEnvelope` is the serialized wire contract
into `backbone-accounting`; a consumer implements `GlPostSink` (async `post(&envelope)`) over
accounting's `PostingService`. Billing never imports accounting in the shipped library.

**C. The tax overlay** — `InvoiceTaxLine` (logical `invoice_ref`, `basis ∈ output|input|withholding`)
is populated by supplying `NewTaxLine`s at create time; a `backbone-tax` adapter fills them from a
`TaxResult`. Remove the overlay and invoices post net-only.

## How a consumer extends
1. **Post to the GL** — implement `GlPostSink` in your composition crate, mapping
   `AccountingPostEnvelope` → accounting's `PostingRequest`; pass it to `post_sales_invoice` /
   `post_purchase_invoice`. (Reference ACL: `tests/ap_seam.rs`.)
2. **React to a posted invoice** — implement `BillingEventSink` in your crate / a `*_custom.rs` and
   pass it to `BillingWriteService::with_sink` (e.g. faktur-pajak numbering, loyalty accrual).
3. **Wire the A/P seam** — route `PurchaseInvoicePosted{source_po_id, billed_lines}` → buying's
   `mark_billed`, retiring buying's simulated billing leg.
4. **Supply tax** — feed `NewTaxLine`s from a `backbone-tax` computation; the base stays neutral.
5. Keep logic in `user_owned`/`*_custom.rs` — survives regen (proven by
   `scripts/ap_seam_roundtrip.sh`).

## Not a contract
Generated CRUD events; internal repositories/services; `// <<< CUSTOM` blocks (own edits only).

## Deferred surfaces
Settlement/reconciliation (backbone-payments), aging/dunning, deferred revenue & subscriptions, credit
notes / reversal posts, e-Faktur numbering, multi-currency — additive when built.
