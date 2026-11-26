# Check kinds

The set is closed on purpose. An adapter implements a kind or reports it `unsupported`.
Adding a kind is a schema change, discussed in an issue first.

| kind | asserts |
|---|---|
| `roundtrip` | decoding the entry's bytes and encoding the result reproduces the bytes exactly |
| `native_preserves_map_entries` | converting an `ScVal::Map` to the SDK's native value keeps every entry or raises; silently returning fewer is a defect |
| `muxed_ids_roundtrip` | each muxed account in the transaction yields exactly the expected decimal ID, in order of appearance |
| `muxed_id_input` | muxed-ID strings in `accept` are taken as the same number; every string in `reject` is refused |
| `liquidity_pool_asset_order` | a pool asset built from the two assets, in the order the network requires, is accepted and has the expected pool ID |
| `signed_payload_strkey` | the CAP-40 signer encodes to the expected `P...` strkey, and decodes back to the key stored in the envelope |

## Deliberately not tested

Leading zeros in muxed-ID strings (`"016"`). Implementations differ, the network does not
decide it, and a corpus that picked a side would report a disagreement that is not a bug.
