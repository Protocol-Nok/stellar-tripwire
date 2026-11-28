"""Tests for adapter.py. Run with: .venv/bin/python -m pytest"""

import adapter


def test_unknown_xdr_type_is_unsupported():
    r = adapter.check_roundtrip("SomethingNew", b"\x00\x00\x00\x00")
    assert r["status"] == "unsupported"


def test_truncated_bytes_fail_rather_than_raise_uncaught():
    r = adapter.check_roundtrip("ScVal", b"\x00\x00")
    assert r["status"] == "fail"


def test_unknown_check_kind_is_unsupported():
    r = adapter.run_check({"xdr_type": "ScVal"}, b"\x00\x00\x00\x00", {"kind": "nonsense"})
    assert r["status"] == "unsupported"


def test_muxed_id_input_is_honestly_unsupported():
    # No clean string-parsing boundary exists for this; the adapter must say so,
    # not silently report a pass it never checked.
    r = adapter.check_muxed_id_input({"accept": ["16"], "reject": ["-1"]})
    assert r["status"] == "unsupported"


def test_signed_payload_strkey_rejects_entry_without_signer():
    # A truncated/empty-looking envelope has no v1/operations to find a signer in;
    # this must fail cleanly (via run_check's try/except), not crash the adapter.
    r = adapter.run_check({}, b"\x00\x00\x00\x00", {"kind": "signed_payload_strkey"})
    assert r["status"] == "fail"


def test_liquidity_pool_asset_order_rejects_non_pool_entry():
    r = adapter.run_check({}, b"\x00\x00\x00\x00", {"kind": "liquidity_pool_asset_order", "fee": 30, "pool_id": "00"})
    assert r["status"] == "fail"
