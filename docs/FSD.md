# FSD — backbone-billing

> Functional Spec. Tier 2 · Financials. Date: 2026-07-05.

## Entities (schema/models/*.model.yaml — SSoT)
SalesInvoice(+lines) · PurchaseInvoice(+lines) · InvoiceTaxLine (removable overlay) · PaymentSchedule.
Invoice headers carry `net_total`/`tax_total`/`grand_total`/`outstanding_amount`, a logical
`receivable_account_id`/`payable_account_id`, `posting_state`, and `journal_id`/`accounting_post_id`
(reconciled from the ack). Cross-module ids are logical FKs (`@exclude_from_foreign_key_check`):
customer/supplier→party, item→catalog, company/branch→organization, account→accounting,
`source_so_id`→selling, `source_po_id`→buying. **No tax columns on the invoice tables** (§1).

## Services (application/service — hand-authored, user_owned)
- `BillingWriteService` — validated creates (`create_sales_invoice` / `create_purchase_invoice`,
  pricing net server-side + inserting the supplied tax overlay by basis); `build_ar_post` /
  `build_ap_post` (assemble ONE balanced `AccountingPostEnvelope`); `post_sales_invoice` /
  `post_purchase_invoice` (short-circuit if already posted → emit envelope through a `GlPostSink` →
  reconcile `journal_id`/`post_id` from the ack → publish the domain event); `add_payment_schedule`.
- `billing_gl` — the outbound GL port: `GlPostLine`, `AccountingPostEnvelope` (idempotency_key,
  company/branch, source_type/id/reference, posting_date, currency, posting_type, lines[]),
  `GlPostAck`, `GlPostRejected`, `GlPostSink` (async trait). The wire contract; zero normal edge.
- `billing_events` — `BillingEvent` {`SalesInvoicePosted`, `PurchaseInvoicePosted` (carries
  `source_po_id` + `billed_lines`), `InvoiceCancelled`} + `BillingEventSink` + `LoggingSink`.

## HTTP surface (presentation/http/guarded_routes.rs)
`create_guarded_billing_routes(&BillingModule, pool)` — read documents + validated `POST
/sales-invoices` + `POST /purchase-invoices`. No generic mutation. Posting needs a `GlPostSink`
composition layer, so it is service/job-driven, not an HTTP route.

## State machines
- Invoice (`InvoiceStatus`): `draft → submitted` on a successful post; `partially_paid`/`paid` are
  advanced by payments against `outstanding_amount`; `cancelled` (reversal deferred).
- Posting (`GlPostingState`): `pending → posted` (ack) / `failed` (rejected, retryable).
- PaymentSchedule (`PaymentScheduleStatus`): `unpaid → partially_paid → paid` / `overdue` (payments).

## Integration seams
- **A/P seam (proven):** posted Purchase Invoice → `PurchaseInvoicePosted{source_po_id, billed_lines}`
  → ACL → buying `mark_billed` → `billed_qty` advances → PO `completed`. The AR/AP post routes through
  a `GlPostSink` ACL into accounting's `PostingService` (real ledger). Zero normal Cargo edge.
  ADR-002, `tests/ap_seam.rs`, `scripts/ap_seam_roundtrip.sh`.
- **Inbound (future):** `OrderInvoiced` (selling) → draft Sales Invoice; `PurchaseOrderRef` (buying) →
  draft Purchase Invoice header; `TaxResult` (backbone-tax) → the overlay; settlement (payments) →
  `outstanding_amount`/schedule.

## Test oracle
`billing_golden_cases` (5: AR + A/P math, idempotency, validation gates, tax-free posts net-only),
`integrity_probes` (4: rejected-post recovery, non-IDR refusal, balanced-with-tax, **concurrent
double-post emits the seam event exactly once** — IP-4, council 2026-07-05),
`ap_seam` (1, real ledger + §5), `schedules_and_events` (2, installments + event surface).
**12 tests** (of the hand-authored suite).
