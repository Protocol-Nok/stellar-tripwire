#!/usr/bin/env bash
# Unlike prove-detection.sh (JS) and prove-detection-go.sh, this doesn't compare a pre-fix
# release against a fixed one -- there is no fixed release yet. network.lightsail:stellar-sdk
# @6.0.0's Asset.compareTo orders issuers by strkey text instead of raw bytes, the same bug
# class as js-stellar-sdk#1766 and go-stellar-sdk's pre-0.7.2 Asset.LessThan (both already
# fixed upstream). Reported: https://github.com/lightsail-network/java-stellar-sdk/issues/849
#
# This script asserts the CURRENT known-bad state. If it starts failing because the known
# failure became a pass, that is good news, not a broken test: it means upstream shipped a
# fix. Update adapters/java/README.md and this script's SDK_VERSION/expectations when that
# happens, rather than just re-pinning to make CI green again.
set -euo pipefail
cd "$(dirname "$0")/.."

work="$(mktemp -d)"; trap 'rm -rf "$work"' EXIT
cargo build -q -p tripwire
bin=target/debug/tripwire
"$bin" generate "$work/corpus" >/dev/null

( cd adapters/java && mvn -q -DskipTests package )
jar=adapters/java/target/tripwire-adapter-java-0.1.0.jar

out="$("$bin" run "$work/corpus" -- java -jar "$jar")" || true
echo "$out"

echo "$out" | grep -q "FAIL  lp-asset-issuer-order / liquidity_pool_asset_order" || {
  echo "EXPECTED FAILURE NOT FOUND: either the adapter broke, or upstream fixed"
  echo "lightsail-network/java-stellar-sdk#849 -- if the latter, update the README and this script."
  exit 1
}

# Nothing else should be failing.
other_failures="$(echo "$out" | grep "^  FAIL" | grep -v "lp-asset-issuer-order / liquidity_pool_asset_order" || true)"
if [ -n "$other_failures" ]; then
  echo "unexpected additional failure(s):"
  echo "$other_failures"
  exit 1
fi

echo "tracked: lightsail-network/java-stellar-sdk#849 is still open (the known failure reproduced)"
