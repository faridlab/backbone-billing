#!/usr/bin/env bash
# Extension-contract §5 for the billing↔buying A/P seam: prove the cross-module ACL/consumer wiring
# survives a regeneration of BOTH modules. Snapshots the seam files, regenerates billing AND buying
# with --force, asserts byte-identical, and re-runs the end-to-end seam test green.
# Usage: DATABASE_URL=... bash scripts/ap_seam_roundtrip.sh
set -euo pipefail
cd "$(dirname "$0")/.."

BILL_FILES=(
  src/application/service/billing_write_service.rs
  src/application/service/billing_events.rs
  src/application/service/billing_gl.rs
  src/presentation/http/guarded_routes.rs
  tests/ap_seam.rs
)
BUY_FILES=(
  ../backbone-buying/src/application/service/buying_write_service.rs
  ../backbone-buying/src/application/service/buying_events.rs
)

echo "→ snapshot seam consumer/ACL files (both modules)"
before=$(shasum -a 256 "${BILL_FILES[@]}" "${BUY_FILES[@]}")

echo "→ regenerate BOTH modules (§5) — buying then billing"
( cd ../backbone-buying && metaphor schema schema generate --force >/dev/null )
metaphor schema schema generate --force >/dev/null

echo "→ verify every seam file is byte-identical after regen"
after=$(shasum -a 256 "${BILL_FILES[@]}" "${BUY_FILES[@]}")
if [ "$before" != "$after" ]; then
  echo "✗ FAIL: a seam file changed during regen"; diff <(echo "$before") <(echo "$after") || true; exit 1
fi
echo "  ✓ all ${#BILL_FILES[@]}+${#BUY_FILES[@]} seam files unchanged"

echo "→ re-run the end-to-end A/P seam post-regen"
cargo test --test ap_seam -- --test-threads=1 >/dev/null
echo "  ✓ buying→billing→accounting→buying seam still green after regenerating both modules"
echo "✓ §5 round-trip proven for the A/P billing seam."
