#!/usr/bin/env bash
# Proves the Go adapter catches a real historical bug: github.com/stellar/go-stellar-sdk
# @v0.7.1's xdr.NewPoolId used Asset.LessThan ordered by strkey text, not raw bytes
# (fixed in v0.7.2, PR #5974 -- the same class of bug as js-stellar-sdk#1766). Mirrors
# scripts/prove-detection.sh for the JS adapter.
set -euo pipefail
cd "$(dirname "$0")/.."

PRE_FIX="${PRE_FIX:-v0.7.1}"   # before xdr.NewPoolId ordered by raw bytes
FIXED="${FIXED:-v0.9.0}"

work="$(mktemp -d)"; trap 'rm -rf "$work"' EXIT
cargo build -q -p tripwire
bin=target/debug/tripwire
"$bin" generate "$work/corpus" >/dev/null

build_adapter() { # build_adapter <sdk-version> <output-dir>
  local ver="$1" dir="$2"
  mkdir -p "$dir"
  cp adapters/go/adapter.go adapters/go/go.mod "$dir"/
  (
    cd "$dir"
    GOFLAGS=-mod=mod go get "github.com/stellar/go-stellar-sdk@$ver" >/dev/null
    GOFLAGS=-mod=mod go mod tidy >/dev/null
    go build -o tripwire-go-adapter .
  )
}

build_adapter "$PRE_FIX" "$work/pre"
pre="$("$bin" run "$work/corpus" --adapter "$work/pre/tripwire-go-adapter")" || true
echo "$pre"
echo "$pre" | grep -q "FAIL  lp-asset-issuer-order / liquidity_pool_asset_order" \
  || { echo "NOT DETECTED on $PRE_FIX"; exit 1; }

build_adapter "$FIXED" "$work/fixed"
fixed="$("$bin" run "$work/corpus" --adapter "$work/fixed/tripwire-go-adapter")" || true
echo "$fixed"
if echo "$fixed" | grep -q "FAIL"; then echo "unexpected failure on $FIXED"; exit 1; fi

echo "detection proven: known bug found on $PRE_FIX, none on $FIXED"
