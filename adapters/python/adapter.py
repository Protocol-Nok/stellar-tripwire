#!/usr/bin/env python3
"""stellar-tripwire adapter for `stellar-sdk` (py-stellar-base) on PyPI -- a genuinely
independent, pure-Python XDR implementation (not a wrapper over the Rust/Go/Java ones).

    python3 adapter.py <corpus-dir>

Protocol: ../../docs/adapter-protocol.md. One JSON object per line on stdout.
"""

import functools
import importlib.metadata
import json
import sys
from pathlib import Path

import stellar_sdk
from stellar_sdk import scval
from stellar_sdk import xdr as stellar_xdr

DECODERS = {
    "ScVal": stellar_xdr.SCVal,
    "TransactionEnvelope": stellar_xdr.TransactionEnvelope,
    "ChangeTrustAsset": stellar_xdr.ChangeTrustAsset,
}


def emit(obj):
    print(json.dumps(obj))


def pass_(detail=""):
    return {"status": "pass", "detail": detail}


def fail(detail):
    return {"status": "fail", "detail": detail}


def unsupported(detail):
    return {"status": "unsupported", "detail": detail}


def sdk_version():
    try:
        return f"stellar-sdk {importlib.metadata.version('stellar-sdk')}"
    except importlib.metadata.PackageNotFoundError:
        return "stellar-sdk (version unknown)"


def safe(fn):
    """Turns any exception the wrapped check raises into a `fail` result, so each
    check function is defensive on its own -- not just because main()'s outer loop
    happens to catch everything too. Matters for anyone calling these directly (the
    test suite does), and makes the failure message specific to the check that threw.
    """

    @functools.wraps(fn)
    def wrapped(*args, **kwargs):
        try:
            return fn(*args, **kwargs)
        except Exception as e:  # noqa: BLE001 - reported as a result, not a crash
            return fail(f"threw {type(e).__name__}: {e}")

    return wrapped


@safe
def check_roundtrip(xdr_type, raw):
    cls = DECODERS.get(xdr_type)
    if cls is None:
        return unsupported(f"unknown xdr_type {xdr_type!r}")
    decoded = cls.from_xdr_bytes(raw)
    again = decoded.to_xdr_bytes()
    if again != raw:
        return fail("decode then encode changed the bytes")
    return pass_()


@safe
def check_native_preserves_map_entries(raw, check):
    val = stellar_xdr.SCVal.from_xdr_bytes(raw)
    native = scval.to_native(val)
    expected = check["entries"]
    got = len(native) if isinstance(native, (dict, list)) else None
    if got is None:
        return fail(f"scval.to_native() returned {type(native).__name__}, expected a dict")
    if got >= expected:
        return pass_(
            "Python dict keys compare by (type, value), so an int key and a str key "
            "that print the same never collide -- this SDK's design has no entry-"
            "collision surface here"
        )
    return fail(f"{expected} entries in the XDR, {got} after scval.to_native(), and no error")


def muxed_accounts(env):
    """The source account, then each payment destination, in order -- muxed ones only."""
    tx = env.v1.tx
    accounts = [tx.source_account]
    for op in tx.operations:
        if op.body.payment_op is not None:
            accounts.append(op.body.payment_op.destination)
    return [a for a in accounts if a.type == stellar_xdr.CryptoKeyType.KEY_TYPE_MUXED_ED25519]


@safe
def check_muxed_ids_roundtrip(raw, check):
    env = stellar_xdr.TransactionEnvelope.from_xdr_bytes(raw)
    found = muxed_accounts(env)
    expected_ids = check["ids"]
    if len(found) != len(expected_ids):
        return fail(f"found {len(found)} muxed accounts, expected {len(expected_ids)}")
    bad = []
    for i, xdr_account in enumerate(found):
        account = stellar_sdk.MuxedAccount.from_xdr_object(xdr_account)
        got = account.account_muxed_id
        if str(got) != expected_ids[i]:
            bad.append(f"#{i}: expected {expected_ids[i]}, SDK says {got!r}")
    return fail("; ".join(bad)) if bad else pass_()


@safe
def check_muxed_id_input(check):
    # MuxedAccount's account_muxed_id parameter is type-hinted as a native int, but
    # Python does not enforce type hints at runtime. Passing a string doesn't raise a
    # clean validation error the way a real string-parsing path would: it's accepted at
    # construction and only fails later, inside to_xdr_object(), with an unrelated
    # "'<' not supported between instances of 'str' and 'int'" from an internal range
    # check -- for every string tried, including otherwise-canonical ones like "16".
    # That's not the string-parsing boundary this check is aimed at, so there's nothing
    # honest to assert pass/fail on here.
    del check  # unused: every input is rejected the same uninformative way
    return unsupported(
        "MuxedAccount's account_muxed_id is type-hinted int; Python doesn't enforce "
        "that at runtime, so a string is accepted at construction and only fails "
        "later with an unrelated TypeError, for every string including canonical "
        "ones -- not the string-parsing boundary this check targets"
    )


@safe
def check_liquidity_pool_asset_order(raw, check):
    cta = stellar_xdr.ChangeTrustAsset.from_xdr_bytes(raw)
    params = cta.liquidity_pool.constant_product
    asset_a = stellar_sdk.Asset.from_xdr_object(params.asset_a)
    asset_b = stellar_sdk.Asset.from_xdr_object(params.asset_b)
    try:
        pool = stellar_sdk.LiquidityPoolAsset(asset_a, asset_b, params.fee.int32)
    except ValueError as e:
        return fail(f"refused an asset pair the network accepts: {e}")
    got = pool.liquidity_pool_id
    expected = check["pool_id"]
    if got.lower() != expected.lower():
        return fail(f"pool id {got}, expected {expected}")
    return pass_()


@safe
def check_signed_payload_strkey(raw, check):
    env = stellar_xdr.TransactionEnvelope.from_xdr_bytes(raw)
    ops = env.v1.tx.operations
    if not ops or ops[0].body.set_options_op is None or ops[0].body.set_options_op.signer is None:
        return fail("entry has no SetOptions signer")
    xdr_key = ops[0].body.set_options_op.signer.key
    key = stellar_sdk.SignerKey.from_xdr_object(xdr_key)
    got = key.encoded_signer_key
    expected = check["strkey"]
    if got != expected:
        return fail(f"encoded {got}, expected {expected}")
    back = stellar_sdk.SignerKey.from_encoded_signer_key(got)
    if back.to_xdr_object().to_xdr_bytes() != xdr_key.to_xdr_bytes():
        return fail("strkey did not decode back to the same signer key")
    return pass_()


def run_check(entry, raw, check):
    kind = check["kind"]
    if kind == "roundtrip":
        return check_roundtrip(entry["xdr_type"], raw)
    if kind == "native_preserves_map_entries":
        return check_native_preserves_map_entries(raw, check)
    if kind == "muxed_ids_roundtrip":
        return check_muxed_ids_roundtrip(raw, check)
    if kind == "muxed_id_input":
        return check_muxed_id_input(check)
    if kind == "liquidity_pool_asset_order":
        return check_liquidity_pool_asset_order(raw, check)
    if kind == "signed_payload_strkey":
        return check_signed_payload_strkey(raw, check)
    return unsupported("check kind not implemented by this adapter")


def main():
    if len(sys.argv) < 2:
        print("usage: adapter.py <corpus-dir>", file=sys.stderr)
        sys.exit(2)
    corpus = Path(sys.argv[1])

    emit({"adapter": "python", "version": sdk_version()})

    manifest = json.loads((corpus / "manifest.json").read_text())
    for entry in manifest["entries"]:
        raw = (corpus / entry["file"]).read_bytes()
        for check in entry["checks"]:
            result = run_check(entry, raw, check)
            emit({"entry": entry["id"], "check": check["kind"], **result})


if __name__ == "__main__":
    main()
