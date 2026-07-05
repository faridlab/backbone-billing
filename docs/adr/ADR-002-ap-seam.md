# ADR-002: The billing↔buying A/P seam (procure-to-pay billing, end-to-end)

**Status**: Accepted — Applied 2026-07-05 (proven end-to-end; retires buying's simulated billing leg)
**Deciders**: Farid (owner), build session 2026-07-05
**Related**: buying ADR-001/002 (receipt seam), inventory ADR-002, `docs/erp/extension-contract.md` §5,
`docs/erp/gl-posting-contract.md`

## Context

Buying proved procure-to-pay through goods receipt but *simulated* its billing leg (`mark_billed`
called directly in the test). This ADR records the symmetric **A/P seam**: a real Purchase Invoice in
`backbone-billing` posts A/P to the ledger AND advances the source PO's `billed_qty` — closing the
3-way match with a real document. It is the AR/AP counterpart of inventory's receipt asset post.

## Decision

1. **Every cross-module hop is a serialized envelope mapped by an ACL — zero normal Cargo edges.**
   - Billing posts the AR/AP journal by emitting `AccountingPostEnvelope` through a `GlPostSink`; a
     composition ACL maps it into accounting's `PostingRequest` → real Journal + Ledger.
   - A posted Purchase Invoice emits `PurchaseInvoicePosted { source_po_id, billed_lines[],
     grand_total }`; the composition routes it → buying `mark_billed(po, lines)` → advances
     `billed_qty` → PO recomputes to `completed`.
   The shipped billing library has **no normal dependency** on buying or accounting
   (`cargo tree -e normal -i backbone-buying`/`-i backbone-accounting` are empty; both are
   dev-dependencies for the seam test only).
2. **The event carries the invoice grand_total, not the post's debit sum.** For a purchase invoice the
   posting's debit total (`net + input`) differs from the A/P owed (`net + input − withholding`);
   `PurchaseInvoicePosted.grand_total` is the invoice's persisted `grand_total` (the A/P), so a
   downstream consumer sees the amount actually owed. (Caught in build; fixed before landing.)
3. **Physical/financial decoupling carries over.** The invoice + lines commit first; the GL post is
   emitted after, is idempotent (`source_id = invoice id`), and is repost-recoverable on rejection.
4. **The seam event publishes exactly once — gated on the state transition, not the ack** (council
   2026-07-05). The GL post is idempotent, but the *event* is a distinct side-effect: under a
   concurrent double-post both callers clear `short_circuit_posted` (both see `pending`) and both get
   a deduped ack from accounting. `post_sales_invoice`/`post_purchase_invoice` therefore bind the
   `pending→posted` UPDATE result and **only publish when `rows_affected() == 1`**; the loser
   reconciles from the persisted row and returns idempotently without re-emitting. Without this, a
   double `PurchaseInvoicePosted` would double-advance the PO's `billed_qty` via `mark_billed` — the
   GL stays balanced while procurement state is silently corrupted. Proven by `IP-4` (2 emissions
   without the gate, 1 with).

## Consequences

- **Proven, not asserted:** `tests/ap_seam.rs` runs the full round-trip — buying confirms a PO
  (10 @ 90,000) and records receipt (→ `to_bill`); billing raises a Purchase Invoice (net 900,000,
  PPN Input 99,000, PPh 23 18,000 → A/P 981,000), posts `Dr Expense 900,000 · Dr PPN Input 99,000 ·
  Cr A/P 981,000 · Cr PPh 18,000` into the real ledger; `PurchaseInvoicePosted` → `mark_billed` →
  PO `completed`, `billed_qty = 10`.
- **Extension-contract §5 discharged for the seam:** `scripts/ap_seam_roundtrip.sh` regenerates
  **both** modules and asserts every ACL/consumer file is byte-identical and the seam stays green.
- This is the **third proven cross-module GL seam** (after selling→revenue and inventory→asset), and
  the seam that finally makes buying's 3-way match close on a real invoice.
- Residual / parking lot: a real event bus + billing service to own the ACL in production; credit
  notes / reversal posts; the inbound `OrderInvoiced` (selling) and `TaxResult` (backbone-tax) seams;
  settlement + aging (backbone-payments).
