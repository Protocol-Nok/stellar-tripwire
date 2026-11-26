#!/usr/bin/env node
// stellar-tripwire adapter for @stellar/stellar-sdk.
//
//   node adapter.mjs <corpus-dir>
//
// Set TRIPWIRE_SDK_DIR to a directory that has @stellar/stellar-sdk in its node_modules to
// test a specific version (for example a pre-fix release) instead of the one installed here.
//
// Protocol: docs/adapter-protocol.md. One JSON object per line on stdout.

import fs from 'node:fs';
import path from 'node:path';
import { createRequire } from 'node:module';
import { fileURLToPath } from 'node:url';

const corpus = process.argv[2];
if (!corpus) {
  console.error('usage: adapter.mjs <corpus-dir>');
  process.exit(2);
}
const here = path.dirname(fileURLToPath(import.meta.url));
const sdkRoot = process.env.TRIPWIRE_SDK_DIR || here;
const sdk = createRequire(path.join(sdkRoot, 'x.js'))('@stellar/stellar-sdk');
const sdkVersion = JSON.parse(
  fs.readFileSync(path.join(sdkRoot, 'node_modules/@stellar/stellar-sdk/package.json'), 'utf8'),
).version;
const { xdr } = sdk;

const TYPES = {
  ScVal: xdr.ScVal,
  TransactionEnvelope: xdr.TransactionEnvelope,
  ChangeTrustAsset: xdr.ChangeTrustAsset,
};

const pass = (detail = '') => ({ status: 'pass', detail });
const fail = (detail) => ({ status: 'fail', detail });

const decode = (entry, bytes) => TYPES[entry.xdr_type].fromXDR(bytes);

/**
 * The adapter uses the high-level APIs developers call, not hand-walked XDR unions: the JS
 * SDK changed its XDR object model between 17.2 and 17.4, and the high-level calls are what
 * downstream code depends on. Decoding failures there are exactly what this corpus is for.
 */
const parseTx = (bytes) => sdk.TransactionBuilder.fromXDR(bytes.toString('base64'), sdk.Networks.PUBLIC);

/** Muxed (M...) addresses in a transaction: the source, then each payment destination. */
function muxedAddresses(bytes) {
  const tx = parseTx(bytes);
  const all = [tx.source];
  for (const op of tx.operations) if (op.type === 'payment') all.push(op.destination);
  return all.filter((a) => a.startsWith('M'));
}

const checks = {
  roundtrip(entry, bytes) {
    const again = decode(entry, bytes).toXDR();
    return Buffer.compare(Buffer.from(again), bytes) === 0
      ? pass()
      : fail('decode then encode changed the bytes');
  },

  native_preserves_map_entries(entry, bytes, check) {
    const value = xdr.ScVal.fromXDR(bytes);
    let native;
    try {
      native = sdk.scValToNative(value);
    } catch (e) {
      return pass(`raised instead of dropping entries: ${e.message}`);
    }
    const n =
      native instanceof Map ? native.size : Array.isArray(native) ? native.length : Object.keys(native).length;
    return n >= check.entries
      ? pass()
      : fail(`${check.entries} entries in the XDR, ${n} after scValToNative, and no error`);
  },

  muxed_ids_roundtrip(entry, bytes, check) {
    const found = muxedAddresses(bytes);
    if (found.length !== check.ids.length) return fail(`found ${found.length} muxed accounts, expected ${check.ids.length}`);
    const bad = [];
    found.forEach((address, i) => {
      const id = sdk.MuxedAccount.fromAddress(address, '0').id();
      if (id !== check.ids[i]) bad.push(`#${i}: expected ${check.ids[i]}, SDK says ${JSON.stringify(id)}`);
    });
    return bad.length ? fail(bad.join('; ')) : pass();
  },

  muxed_id_input(entry, bytes, check) {
    const account = new sdk.Account(
      sdk.MuxedAccount.fromAddress(muxedAddresses(bytes)[0], '0').baseAccount().accountId(),
      '0',
    );
    const bad = [];
    for (const v of check.accept) {
      try {
        const id = new sdk.MuxedAccount(account, v).id();
        if (BigInt(id) !== BigInt(v)) bad.push(`accepted ${JSON.stringify(v)} but id() is ${JSON.stringify(id)}`);
      } catch (e) {
        bad.push(`refused valid ${JSON.stringify(v)}: ${e.message}`);
      }
    }
    for (const v of check.reject) {
      try {
        const m = new sdk.MuxedAccount(account, v);
        bad.push(`accepted ${JSON.stringify(v)} (id() = ${JSON.stringify(m.id())})`);
      } catch {
        /* refusing is what we want */
      }
    }
    return bad.length ? fail(bad.join('; ')) : pass();
  },

  liquidity_pool_asset_order(entry, bytes, check) {
    let pool;
    try {
      pool = sdk.LiquidityPoolAsset.fromOperation(xdr.ChangeTrustAsset.fromXDR(bytes));
    } catch (e) {
      return fail(`refused an asset pair the network accepts: ${e.message}`);
    }
    const id = Buffer.from(pool.getLiquidityPoolId ? pool.getLiquidityPoolId() : sdk.getLiquidityPoolId('constant_product', pool.getLiquidityPoolParameters())).toString('hex');
    return id === check.pool_id ? pass() : fail(`pool id ${id}, expected ${check.pool_id}`);
  },

  signed_payload_strkey(entry, bytes, check) {
    const op = parseTx(bytes).operations[0];
    const text = op.signer?.ed25519SignedPayload;
    if (text !== check.strkey) return fail(`SDK reports signer ${JSON.stringify(text)}, expected ${check.strkey}`);
    // The key decoded from the P... string must be exactly the key bytes in the envelope.
    const decoded = Buffer.from(sdk.SignerKey.decodeAddress(text).toXDR());
    return bytes.includes(decoded)
      ? pass()
      : fail('the P... strkey does not decode to the signer key stored in the envelope');
  },
};

const emit = (o) => process.stdout.write(JSON.stringify(o) + '\n');
emit({ adapter: 'js', version: `@stellar/stellar-sdk ${sdkVersion}` });

const manifest = JSON.parse(fs.readFileSync(path.join(corpus, 'manifest.json'), 'utf8'));
for (const entry of manifest.entries) {
  const bytes = fs.readFileSync(path.join(corpus, entry.file));
  for (const check of entry.checks) {
    const run = checks[check.kind];
    let result;
    if (!run) result = { status: 'unsupported', detail: 'check kind not implemented by this adapter' };
    else {
      try {
        result = run(entry, bytes, check);
      } catch (e) {
        result = fail(`threw ${e.constructor?.name ?? 'Error'}: ${e.message}`);
      }
    }
    emit({ entry: entry.id, check: check.kind, ...result });
  }
}
