# PRD — backbone-billing

> Tier 2 · Financials · Indonesia-first ERP. Status: built. Date: 2026-07-05.

## Problem & intent
A business bills its customers (AR) and is billed by its suppliers (A/P). `backbone-billing` owns
that invoicing pipeline as an independent module: it turns a Sales/Purchase Invoice into ONE balanced
posting to the ledger of record (`backbone-accounting`) and retires buying's *simulated* billing leg
with a real Purchase Invoice — mirroring how selling/inventory drive revenue and stock posts.

## Goals
- Own **Sales Invoice** (AR) and **Purchase Invoice** (A/P), each with line children.
- Compute money **server-side**; guarded surface (no generic mutation of totals).
- Post **one balanced `AccountingPost`** per invoice through the GL-posting contract:
  - Sales: `Dr A/R (grand) [customer] · Cr Revenue (net) · Cr PPN Output (Σoutput)`.
  - Purchase: `Dr Expense (net) · Dr PPN Input (Σinput) · Cr A/P (grand) [supplier] · Cr PPh (Σwht)`.
- Keep the base **region-neutral**: invoices carry NO tax columns; PPN/PPh live in a **removable**
  `InvoiceTaxLine` overlay (supplied now; `backbone-tax` computes later).
- Drive the **A/P seam** billing→buying (`PurchaseInvoicePosted` → `mark_billed`), zero normal edge.
- Record **payment schedules** (installment due dates); settlement is `backbone-payments`' concern.

## Non-goals (this phase / deferred)
Settlement/reconciliation (backbone-payments), dunning/aging read-models, deferred revenue &
subscriptions, credit notes / reversal posts, tax *computation* (backbone-tax; overlay supplied),
e-Faktur/faktur-pajak numbering, multi-currency/FX, POS invoicing.

## Personas
Finance/AR clerk (raises sales invoices, tracks outstanding), AP clerk (books supplier invoices
against POs), Integrating engineer (subscribes to invoice events, wires the A/P seam + tax overlay).

## Success criteria
- AR/AP math + posting locked by a numeric oracle (5 golden cases) + integrity probes (4).
- The A/P seam proven end-to-end against the real ledger (APSEAM-1) + survives regen of both modules
  (§5, `scripts/ap_seam_roundtrip.sh`).
- Indonesia-ready: PPN Output/Input + PPh withholding flow through the removable overlay; the base
  posts clean net-only when the overlay is empty.
