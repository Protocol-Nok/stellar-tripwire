# Python adapter

stellar-tripwire adapter for [`stellar-sdk`](https://pypi.org/project/stellar-sdk/)
(py-stellar-base) — a genuinely independent, pure-Python XDR implementation,
not a wrapper over the Rust, Go or Java SDKs.

```sh
python3 -m venv .venv
.venv/bin/pip install -r requirements.txt
tripwire run <corpus-dir> -- .venv/bin/python adapter.py
```

## Coverage

| Check | Status | Notes |
|---|---|---|
| `roundtrip` | implemented | `from_xdr_bytes` / `to_xdr_bytes` on the generated types |
| `native_preserves_map_entries` | implemented, passes | Python dict keys compare by `(type, value)`, so an `int` key and a `str` key that print the same never collide — this SDK's design has no entry-collision surface here |
| `muxed_ids_roundtrip` | implemented | `MuxedAccount.from_xdr_object(...).account_muxed_id` |
| `muxed_id_input` | **unsupported** | `account_muxed_id` is type-hinted `int`, but Python doesn't enforce that at runtime — a string is accepted at construction and only fails later with an unrelated `TypeError`, for every string including canonical ones. Not the string-parsing boundary this check targets. |
| `liquidity_pool_asset_order` | implemented, **passes** | `LiquidityPoolAsset`'s own source comments "Compare asset issuers by raw key bytes, as stellar-core does" — and it does; verified against this corpus |
| `signed_payload_strkey` | implemented | `SignerKey.from_xdr_object(...).encoded_signer_key` / `.from_encoded_signer_key(...)` |

Unlike the JS, Go and Java adapters, this one currently finds **no bug**: every
check that has a real equivalent in this SDK passes, including the two bug
classes the other three SDKs have each gotten wrong in some version. That's a
genuine result worth stating plainly, not a reason to look for something to
report.

## Development

```sh
python3 -m venv .venv
.venv/bin/pip install -r requirements-dev.txt
.venv/bin/python -m pytest
.venv/bin/ruff check .
```

## A note on this environment

The development sandbox this was built in shipped Python without `pip` or
`venv` (Debian/Ubuntu split those into separate, not-preinstalled packages).
`pip` was bootstrapped by downloading the official `python3-pip` `.deb` from
Ubuntu's archive, verifying its SHA-256 against the archive's own published
`Packages` index, and extracting (not installing via `dpkg` — no maintainer
scripts ran) its `pip` module into a venv. A normal Python install, and
GitHub Actions' `ubuntu-latest` runners, have `pip` and `venv` already; this
note exists only so the unusual setup commit in this repo's history makes
sense, not because anyone else needs to repeat it.
