#!/usr/bin/env bash
# Proves the replay harness actually catches a dropped observation, not just that it can
# print a report: runs the correct reference indexer and requires an all-PASS result, then
# runs the deliberately broken one -- which reproduces two independent real bugs,
# wallet-backend#716 (CAP-0040 signer truncation) and wallet-backend#731 (CAP-0073
# trustline missed because effects are gated on envelope operation type) -- and requires
# both entries to be caught as DROP. If this fails, the harness has stopped detecting them.
set -euo pipefail
cd "$(dirname "$0")/.."

work="$(mktemp -d)"; trap 'rm -rf "$work"' EXIT
cargo build -q -p tripwire
bin=target/debug/tripwire
"$bin" replay generate "$work/corpus" >/dev/null
(cd replay/reference-js && npm ci --silent)

good="$("$bin" replay run "$work/corpus" -- node replay/reference-js/good-indexer.mjs)"; echo "$good"
if ! echo "$good" | grep -qE "^[0-9]+ passed, 0 dropped, 0 missing$"; then
  echo "correct indexer was not reported as all-PASS"; exit 1
fi

set +e
broken="$("$bin" replay run "$work/corpus" -- node replay/reference-js/broken-indexer.mjs)"
code=$?
set -e
echo "$broken"
if [ "$code" -eq 0 ]; then echo "broken indexer was not caught (exit 0)"; exit 1; fi
echo "$broken" | grep -q "DROP  captured-signed-payload-signer-ledger" || {
  echo "NOT DETECTED: broken indexer's CAP-0040 truncation (wallet-backend#716)"; exit 1
}
echo "$broken" | grep -q "DROP  cap73-sac-trust-ledger" || {
  echo "NOT DETECTED: broken indexer's CAP-0073 envelope-type gating (wallet-backend#731)"; exit 1
}
echo "detection proven: both of the broken indexer's dropped observations were caught, the correct one passed clean"
