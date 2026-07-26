---
date: 2026-07-26
repo_type: module
unit: backbone-billing
focus_lens: bounded-context-cleanliness
roster:
  standing: [chair, skeptic, steelman, yagni-business]
  context: [ddd-bounded-context, contract-seat]
  invited: [domain-expert]
notes: >
  Skeptic's load-bearing claim independently verified against
  backbone-buying/src/application/service/buying_write_service.rs:596-598.
  Chair's "no live consumer import" fact verified: backbone-buying/src/ has
  zero `use billing::` imports (downstream-free consolidation).
---

# Council — module:backbone-billing — focus: bounded-context-cleanliness

## Best call
Consolidate the seam contract into `exports::` and disambiguate the dual `BillingEvent`. Move `application/service/billing_events.rs` (the 3 seam event structs + `BilledLine` + the real 3-variant `BillingEvent` enum + `BillingEventSink` trait + `LoggingSink`) into a regen-safe home under `exports/` (sibling `exports/seam_events.rs` re-exported from `exports/mod.rs`, or a `// <<< CUSTOM` block in `exports/events.rs`), and rename the dead 18-variant generated ghost at `exports/events.rs:182` from `BillingEvent` to `GeneratedCrudEvents` (or delete it — its variants are verified to appear only in their own enum definition; no emitter, no sink wired to it). One move, smallest residual downside.

- Residual negative value: ~2h work (move + rename + rewire the single `BillingEventSink` binding in `module.rs`). Zero new Cargo edges; the seam stays a serialized envelope behind a port, and the boundary strictly strengthens (consumers depend only on `exports::`, as `exports/mod.rs:6` already demands). Risk surface is near-zero: `backbone-buying/src/` has zero `use billing::` imports today (grep-verified), so no coordinated downstream release is required. The only residual is a regen-overwrite caveat — the seam must land where `metaphor-schema` cannot clobber it.
- Reversibility: easy. It is a file move plus a rename of dead code; no live downstream `use` to coordinate, no schema change, no migration. One-way door only if a consumer has already pinned the `application::service::billing_events` path on a branch heading to release — grep says no.
- What would flip this: (a) a live consumer (buying/selling) already importing `PurchaseInvoicePosted` from `application::service::billing_events` on a merging branch — flips reversibility to costly and elevates a deprecation-shim approach; (b) the codegen pipeline proving unable to preserve a `// <<< CUSTOM` block in `exports/events.rs` — forces the ghost rename through schema-YAML config, raising the cost from ~2h toward half a day.

## Disagreement map
- **Structural seam defect vs. record-accuracy defect** — ddd-bounded-context + contract-seat want the seam physically moved into `exports::` and the ghost renamed (structural, every consumer hits it). The skeptic, with the chair-runner's independent verification of `buying_write_service.rs:596-598`, wants ADR-002 §4 rewritten and IP-4 re-aimed at `mark_billed` under `emitted==2`, because the stated threat ("double emit double-advances `billed_qty`") is wrong — buying's own `total_cap = Σ(received_qty − billed_qty)` cap rejects `qty > total_cap` and publishes `ThreeWayMatchFailed{over_billing}`, so the real worst case is misleading operational signal, not corruption. Crux: which defect does more damage to the next engineer under THIS lens — a ubiquitous-language collision in the wrong layer that every consumer meets, or an overclaiming ADR whose code already defends in depth. Under bounded-context-cleanliness the structural defect wins; the record defect is real but its blast radius is the ADR reader and buying's cap already defends. Record-accuracy is ranked #4, not merged into the Best call.
- **Type-level scope vs. ambient runtime scope** — contract-seat wants `apply_settlement(company_id, invoice_ref, kind, amount)` so the tenant precondition is a compile-time fact. The current code at `billing_settlement.rs:60-67` deliberately binds an ambient `with_company_scope` set by an ACL because the seam is "driven by payment's `PaymentSettled` via an ACL" and trusts that caller. Crux: is the ACL the trusted scope-setter, or is every seam entry point self-securing. Under cleanliness a boundary that leans on an ambient runtime for correctness is leaking its composition assumptions into its contract — type-level wins, ranked #2.
- **Honest remainder contract vs. owning on-account** — domain-expert notes billing offloads the overpayment remainder to "the caller" (`billing_settlement.rs:54-59`) but has no customer-credit/on-account concept, so unapplied cash cannot be re-applied within billing. contract-seat wants the remainder surfaced in the return type so the caller's obligation is visible. yagni-business notes the whole module is over-polished for a context whose ADR-002 parking lot admits no production consumer. Crux: is the on-account gap a billing responsibility or a composition-layer responsibility. The settlement docstring is deliberate that billing owns the invoice subledger and the composition layer owns party credit — so the fix is an honest contract (type-level remainder), not a new aggregate. Modeling-on-account-in-billing is parked.

## Recommendations (ranked by leverage)
| # | Move | Leverage | Residual negative | Reversibility | Evidence to flip |
|---|------|----------|-------------------|---------------|------------------|
| 1 | Consolidate seam into `exports::` + rename/delete the dead `BillingEvent` ghost at `exports/events.rs:182` | high — kills the loudest BC smell (two public types same name, real one in the wrong layer); unblocks #2/#3 | ~2h; near-zero risk; only caveat is regen-safe placement | easy | a merging branch with a live `use ...::application::service::billing_events` import, or codegen can't preserve CUSTOM in `exports/events.rs` |
| 2 | Lift `company_id` into the settlement seam signature: `apply_settlement(company_id, ..)` and `apply_settlements_once(company_id, ..)` | med-high — turns invisible runtime precondition (`billing_settlement.rs:60-67`) into a compile-time check | one signature change + one ACL call-site update; small | easy | evidence that the ACL is the sole legitimate scope-setter and will never be bypassed by a non-ACL caller |
| 3 | Make the overpayment remainder explicit: `apply_settlement` returns `SettlementOutcome { applied, remainder }` instead of bare `Decimal` | med — documents the caller's booking obligation at the type level (`billing_settlement.rs:54-59`) | small; touches settlement call-sites only | easy | evidence that every caller already books the remainder correctly without the type nudge |
| 4 | Correct ADR-002 §4's threat model (worst case = misleading `ThreeWayMatchFailed{over_billing}`, not `billed_qty` corruption) and rewrite IP-4 to drive `mark_billed` under `emitted==2` against buying's real cap | med — record honesty for the seam's threat model; stops the tautological proof | low; doc + test rewrite; no code behavior change | easy | evidence buying's `allocate` cap (`buying_write_service.rs:596-598`) is ever removed — then double-emit becomes real corruption and this becomes #1 |

## Parking lot
- C4 fence-child migration half-staged (`.down.sql` deleted, no paired down) — raised by steelman, scope: migration/RLS deploy (off-lens, but a deploy blocker independent of this council — must be fixed before any release).
- C2 "zero normal Cargo edges" not enforced in CI — raised by steelman, scope: CI/tooling (a `cargo tree`/edge-lint gate; future council).
- gRPC/proto toolchain carried as dead weight (`tonic`/`prost`/`tonic-build` + `grpc` feature, generators disabled, no `build.rs`) — raised by yagni-business, scope: Cargo feature hygiene (off-lens; cut or gate behind a real consumer).
- Asymmetric reversal vocabulary (`reverse_sales_invoice` exists, no `reverse_purchase_invoice`) — raised by domain-expert, scope: domain vocabulary (off-lens; model decision, revisit when A/P returns/credit-note story lands).
- Billing owning customer-credit/on-account aggregate — raised by domain-expert, scope: domain modeling (off-lens; deliberate boundary today, revisit only if composition-layer booking proves insufficient).
- C5 `build_*_post` as the sole emit path invariant — raised by steelman, scope: invariant enforcement (off-lens; worth a guard test in a future council).
