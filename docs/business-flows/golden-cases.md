# Billing — Golden Cases (the numeric oracle)

Mirrors `tests/billing_golden_cases.rs`, `tests/integrity_probes.rs`, `tests/schedules_and_events.rs`,
and the cross-module A/P seam in `tests/ap_seam.rs`. Money is exact IDR (2dp, half-up).

## Write path + posting (`tests/billing_golden_cases.rs`)

| Case | Input | Expected |
|------|-------|----------|
| **SIGC-1** | Sales Invoice: 10 × 100,000 net, PPN Output 11% (110,000) supplied | net `1,000,000`, tax `110,000`, grand `1,110,000`; post `Dr A/R 1,110,000 [customer] · Cr Revenue 1,000,000 · Cr PPN Output 110,000` (balanced); `posted`, outstanding `1,110,000`. |
| **SIGC-2** | Purchase Invoice: 10 × 90,000 net, PPN Input 11% (99,000), PPh 23 2% (18,000) | net `900,000`, tax `99,000`, withholding `18,000`, grand `981,000`; post `Dr Expense 900,000 · Dr PPN Input 99,000 · Cr A/P 981,000 [supplier] · Cr PPh 18,000` (999,000 each side). |
| **SIGC-3** | post the same sales invoice twice | second post `idempotent_reuse=true`, same journal, the sink hit exactly once. |
| **SIGC-4** | empty / negative / duplicate-number | `empty_document` / `negative_amount` / `duplicate_number`. |
| **SIGC-5** | Sales Invoice with NO tax lines (2 × 75,000) | grand == net `150,000`; post is `Dr A/R + Cr Revenue` only (overlay removable). |

## Integrity probes (`tests/integrity_probes.rs`)

| Case | Input | Expected |
|------|-------|----------|
| **IP-1** | GL sink rejects the post, then a good sink retries | first: `posting_state=failed`, `status=draft`, no journal (recoverable); retry → `posted`/`submitted`. |
| **IP-2** | post a non-IDR (USD) invoice | `unsupported_currency`; the sink is never reached (no mis-valued post). |
| **IP-3** | `build_ar_post` with a supplied output tax line | balanced; A/R debit `= net + Σoutput`; grand persisted = `net + output`. |
| **IP-4** | two `tokio::join!`ed `post_purchase_invoice` calls racing on a barrier inside a blocking sink (council 2026-07-05) | the seam event fires **exactly once** — the `pending→posted` gate stops a double-`mark_billed` that would over-advance the PO's `billed_qty` (2 without the gate, 1 with). |

## Schedules + event surface (`tests/schedules_and_events.rs`)

| Case | Input | Expected |
|------|-------|----------|
| **SE-1** | 3 installments on an invoice | rows numbered 1..3, all `unpaid`, summing the grand (300,000). |
| **SE-2** | post a sales invoice sourced from an SO | `SalesInvoicePosted` emitted carrying `source_so_id` + grand `300,000` + company. |

## A/P seam — buying ↔ billing ↔ accounting (`tests/ap_seam.rs` + `scripts/ap_seam_roundtrip.sh`)

| Case | Input | Expected |
|------|-------|----------|
| **APSEAM-1** | buying PO (10 @ 90,000) confirmed + received (`to_bill`); billing raises a Purchase Invoice (PPN Input 99,000, PPh 18,000 → A/P 981,000), posts; `PurchaseInvoicePosted` → `mark_billed` | real journal balances `999,000` each side; PO `completed`, `billed_qty`=10; invoice `posted`, `journal_id` reconciled. Zero normal Cargo edge. |
| **§5 round-trip** | regen BOTH billing + buying, re-run | all seam ACL/consumer files byte-identical; APSEAM-1 still green — survives regen of both modules. |

## Conventions
- The invoice tables carry **no tax columns**; PPN/PPh live in the removable `InvoiceTaxLine` overlay
  (localization-standard §1). Empty overlay → clean net-only post.
- One balanced `AccountingPost` per invoice; the receivable/payable line carries the party (subledger).
- Posting is idempotent (`source_id = invoice id`) + repost-recoverable on rejection; IDR-only for now.
- Billing records payment **schedules**, never settlement (that is `backbone-payments`).
