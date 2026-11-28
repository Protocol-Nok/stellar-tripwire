#!/usr/bin/env bash
# docs/cap-coverage.md's "Covered" table names specific corpus/replay entry IDs as
# evidence a CAP is tested. This checks those IDs still exist in what the generators
# actually produce today, so a renamed or removed entry leaves the doc quietly wrong
# only until the next CI run, not forever.
set -euo pipefail
cd "$(dirname "$0")/.."

work="$(mktemp -d)"; trap 'rm -rf "$work"' EXIT
cargo build -q -p tripwire
bin=target/debug/tripwire
"$bin" generate "$work/corpus" >/dev/null
"$bin" replay generate "$work/replay-corpus" >/dev/null

have_ids="$(jq -r '.entries[].id' "$work/corpus/manifest.json" "$work/replay-corpus/manifest.json")"

claimed_ids="$(
  awk '/^## Covered/{flag=1; next} /^## /{if(flag) exit} flag' docs/cap-coverage.md \
    | awk -F'|' 'NF>=7 && $6 !~ /^[- ]*$/ {print $6}' \
    | grep -oE '`[a-z0-9][a-z0-9-]*`' | tr -d '`' \
    | grep -vE '^(tripwire-corpus|tripwire-replay)$' \
    | sort -u
)"

if [ -z "$claimed_ids" ]; then
  echo "found no entry IDs in docs/cap-coverage.md's Covered table -- check the table format, or this script's parsing of it"
  exit 1
fi

missing=0
while IFS= read -r id; do
  if ! grep -qxF "$id" <<<"$have_ids"; then
    echo "docs/cap-coverage.md claims coverage from entry '$id', but no corpus or replay entry has that ID today"
    missing=1
  fi
done <<<"$claimed_ids"

if [ "$missing" -ne 0 ]; then
  exit 1
fi
echo "ok: every entry ID docs/cap-coverage.md's Covered table names still exists"
