#!/usr/bin/env bash
# Proves the corpus catches the real bugs it was built from: runs the JS adapter against a
# pre-fix @stellar/stellar-sdk and requires exactly the three known failures, then against
# the fixed release and requires none. If this fails, a check has stopped detecting its bug.
set -euo pipefail
cd "$(dirname "$0")/.."

PRE_FIX="${PRE_FIX:-17.2.0}"      # before js-stellar-sdk #1766, #1785, #1789 were fixed
FIXED="${FIXED:-17.4.0}"

work="$(mktemp -d)"; trap 'rm -rf "$work"' EXIT
cargo build -q -p tripwire
bin=target/debug/tripwire
"$bin" generate "$work/corpus" >/dev/null

run() { # run <sdk-version>
  mkdir -p "$work/sdk-$1"
  (cd "$work/sdk-$1" && npm init -y >/dev/null && npm i --silent "@stellar/stellar-sdk@$1")
  TRIPWIRE_SDK_DIR="$work/sdk-$1" "$bin" run "$work/corpus" --adapter node adapters/js/adapter.mjs || true
}

pre="$(run "$PRE_FIX")"; echo "$pre"
for want in "scval-map-key-collision / native_preserves_map_entries" \
            "muxed-ids-boundary / muxed_id_input" \
            "lp-asset-issuer-order / liquidity_pool_asset_order"; do
  echo "$pre" | grep -q "FAIL  $want" || { echo "NOT DETECTED on $PRE_FIX: $want"; exit 1; }
done

fixed="$(run "$FIXED")"; echo "$fixed"
if echo "$fixed" | grep -q "FAIL"; then echo "unexpected failure on $FIXED"; exit 1; fi
echo "detection proven: 3 known bugs found on $PRE_FIX, none on $FIXED"
