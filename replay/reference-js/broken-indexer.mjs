#!/usr/bin/env node
// A deliberately broken reference indexer, reproducing two real, independent bugs:
//
// - wallet-backend#716 (CAP-0040): for a signed-payload signer, it stores/reports only
//   the 32-byte ed25519 portion in the shape a plain Ed25519 signer needs, silently
//   discarding the payload.
// - wallet-backend#731 (CAP-0073): it only looks for a trustline change under a
//   ChangeTrust operation's own code path -- dispatching on the envelope's operation
//   type, exactly the mistake docs/indexer-hardening-guide.md section 3 warns against.
//   A trustline created by a Soroban InvokeHostFunction call into the Stellar Asset
//   Contract's trust() is real on the ledger but has no matching code path here, so it's
//   silently missed.
//
// Both bugs leave the indexer processing the ledger fine -- it just reports the wrong
// fact, which the replay harness should catch as a DROP, not a crash.
//
//   node broken-indexer.mjs <corpus-dir>
//
// See docs/replay-protocol.md and docs/indexer-hardening-guide.md sections 3 and 4.

import fs from 'node:fs';
import path from 'node:path';
import { createRequire } from 'node:module';
import { fileURLToPath } from 'node:url';

const require = createRequire(path.join(path.dirname(fileURLToPath(import.meta.url)), 'x.js'));
const { xdr, StrKey, SignerKey } = require('@stellar/stellar-sdk');
const corpus = process.argv[2];
if (!corpus) {
  console.error('usage: broken-indexer.mjs <corpus-dir>');
  process.exit(2);
}

function canonicalAsset(trustLineAsset) {
  if (trustLineAsset.type === 'assetTypeNative') return 'native';
  const alphaNum = trustLineAsset.alphaNum4 ?? trustLineAsset.alphaNum12;
  const code = Buffer.from(alphaNum.assetCode.value).toString('ascii').replace(/\0+$/, '');
  const issuer = StrKey.encodeEd25519PublicKey(Buffer.from(alphaNum.issuer.ed25519.value));
  return `${code}:${issuer}`;
}

const emit = (o) => process.stdout.write(JSON.stringify(o) + '\n');
emit({ indexer: 'reference-js-broken-cap40-truncation-and-cap73-envelope-gating', version: '0.1.0' });

// THE BUG (CAP-0073 / wallet-backend#731): only this one operation type's own code path
// ever looks for a trustline change. A trustline created by any other operation type --
// InvokeHostFunction included -- has no matching branch here and is silently missed.
const OPERATION_TYPE_WITH_TRUSTLINE_HANDLING = 'changeTrust';

const manifest = JSON.parse(fs.readFileSync(path.join(corpus, 'manifest.json'), 'utf8'));
for (const entry of manifest.entries) {
  const bytes = fs.readFileSync(path.join(corpus, entry.file));
  const observations = [];
  try {
    const ledger = xdr.LedgerCloseMeta.fromXDR(bytes).v1;
    const envelopeOps = ledger.txSet.v1TxSet.phases[0].v0Components[0].txsMaybeDiscountedFee
      .txs[0].v1.tx.operations;
    for (const trm of ledger.txProcessing) {
      if (trm.txApplyProcessing.type !== 'v3') continue;
      for (const [opIndex, opMeta] of trm.txApplyProcessing.v3.operations.entries()) {
        const opType = envelopeOps[opIndex].body.type;
        for (const change of opMeta.changes) {
          const entryData = change.updated ?? change.created;
          if (!entryData) continue;

          if (entryData.data.type === 'account') {
            const account = entryData.data.account;
            const accountId = StrKey.encodeEd25519PublicKey(Buffer.from(account.accountId.ed25519.value));
            for (const signer of account.signers) {
              // THE BUG (CAP-0040 / wallet-backend#716): a signed-payload signer has a
              // 32-byte ed25519 pubkey plus a separate payload. Report only the pubkey,
              // re-encoded as a plain Ed25519 signer -- the payload is silently dropped.
              const signerKey =
                signer.key.type === 'signerKeyTypeEd25519SignedPayload'
                  ? StrKey.encodeEd25519PublicKey(
                      Buffer.from(signer.key.ed25519SignedPayload.ed25519.value),
                    )
                  : SignerKey.encodeSignerKey(signer.key);
              observations.push({
                kind: 'signer_added',
                account_id: accountId,
                signer_key: signerKey,
              });
            }
          } else if (
            entryData.data.type === 'trustline' &&
            change.type === 'ledgerEntryCreated' &&
            opType === OPERATION_TYPE_WITH_TRUSTLINE_HANDLING
          ) {
            const trustLine = entryData.data.trustLine;
            observations.push({
              kind: 'trustline_created',
              account_id: StrKey.encodeEd25519PublicKey(Buffer.from(trustLine.accountId.ed25519.value)),
              asset: canonicalAsset(trustLine.asset),
            });
          }
        }
      }
    }
  } catch {
    // report nothing for a ledger this (broken) indexer can't process at all
  }
  emit({ ledger: entry.id, observations });
}
