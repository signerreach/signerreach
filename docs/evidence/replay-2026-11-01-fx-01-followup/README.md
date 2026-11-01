# FX-01 follow-up Testnet replay — 2026-11-01

**Status:** verified historical Protocol 29 Testnet observation for FX-01 only.
This record does not verify Protocol 30, the other historical follow-up cases,
or the state of any account after this run.

This experiment exercised the approved replay-only `after.json` attempt for
FX-01, not the checker's `snapshot.json`, `plan.json`, or
`availability.json`. It used a fresh disposable Testnet account funded through
the harness's existing Friendbot flow. No seed, secret key, or signing material
is stored in this directory.

## Files

- [`results.json`](results.json) is the exact JSON emitted by the live replay;
  it has not been rewritten. Its allowlisted raw response contains the failed
  submission's HTTP status and result codes.
- [`horizon-root.json`](horizon-root.json) is the public Horizon root response
  captured before the live run.
- [`horizon-ledger-5500424.json`](horizon-ledger-5500424.json) is the root
  response's `history_latest_ledger` record. The root response reported this
  ledger closed at `2026-11-01T09:36:00Z`.
- [`horizon-ledger-5500417.json`](horizon-ledger-5500417.json) is an earlier
  successful public-ledger health read during the same preflight. It is retained
  unchanged as supplementary timing evidence.

## Network preflight

At `2026-11-01T09:36:04Z` the captured Horizon root response returned HTTP 200
for `https://horizon-testnet.stellar.org/` and reported:

| Field | Observed value |
|---|---|
| Network passphrase | `Test SDF Network ; September 2015` |
| Active protocol | `29` |
| Horizon version | `30.0.0-b3ff4119cc99ebcc8048334f5ec64e78c16d1666` |
| Core version | `stellar-core 30.0.0 (0e49751aa9ddf892302cca02cea7f3f5d97df9f1)` |
| `history_latest_ledger` | `5500424` |
| Ledger 5500424 close time / protocol | `2026-11-01T09:36:00Z` / `29` |

The root also advertised support for protocol 30, but its
`current_protocol_version` was 29. The live harness independently performed
its Protocol-29 root guard immediately before it funded or submitted anything;
its replay preflight began at `2026-11-01T09:43:56.423Z`, observed ledger
`5500519`, and recorded the same Core version.

## Observed replay

The live command completed at `2026-11-01T09:44:34.918Z` with exit code 0:

```sh
cd tools/replay
node replay.mjs --only fx-01-raise-high-with-lost-signer --out /tmp/fx-01-followup.json
```

The command above is a live Testnet action and must not be run in CI. It
creates new disposable accounts; the harness refuses to proceed before its
Testnet and Protocol-29 checks pass.

| Step | Expected | Observed | Hash / ledger |
|---|---|---|---|
| Harness setup | accepted | accepted | `3deb1fd5e13241d8993287be547a43f0b0382f5ebf2423af700cc8279c8e6789` / `5500523` |
| FX-01 `raise-high` plan transaction | accepted | accepted | `6e0c973269d2a558b9d80a1a826359b6c104f55a78983d35ee4be700ce083e55` / `5500524` |
| FX-01 `post-master-alone`: `SetOptions(high=0)`, signed only by the available master | `rejected_at_submission`, `tx_failed`, `op_bad_auth` | `rejected_at_submission`, `tx_failed`, `op_bad_auth` | `b7952b310286ecf301de92239fe57ea3597057edc57208f3670e44bd1b5f91c2` / no ledger asserted |

The final row is a **submission rejection**, not an included transaction that
failed during application. Horizon returned HTTP 400 with the structured
transaction result code `tx_failed` and ordered operation code `op_bad_auth`.
The result therefore does not claim a transaction-inclusion ledger for that
hash.

## Reconciliation

The replay's post-plan precondition matched the expected account state and
sequence `23624557805961218`. Before and after the rejected follow-up, it
observed LOW/MEDIUM/HIGH thresholds `1/1/2` and exactly two weight-1 ed25519
signers: the disposable account's master address and
`GDUGHLYZGMEHZTFNESQ6VQSGKBJMS3PRZUFEPVR6VOBYQTNNRA6JROIO`.

At `2026-11-01T09:46:38.269Z`, separate read-only Horizon requests confirmed
the accepted plan hash as successful in ledger `5500524` (created at
`2026-11-01T09:44:20Z`) and returned the same account sequence, thresholds,
and signer set. A direct lookup of the rejected follow-up hash returned HTTP
404. The replay itself recorded five bounded history lookups, unchanged
sequence and authorization state, and `non_inclusion_is_absolute_proof: false`.
Those bounded checks support the submission-rejection interpretation but do
not turn a missing history record into absolute non-inclusion proof.

The successful transaction and the disposable account can be independently
read from Horizon with the exact URLs below. The rejected-hash request is
expected to return HTTP 404 at the time of this evidence, so it is deliberately
not used as a Markdown link check target.

```sh
curl --fail --silent --show-error \
  https://horizon-testnet.stellar.org/transactions/6e0c973269d2a558b9d80a1a826359b6c104f55a78983d35ee4be700ce083e55
curl --fail --silent --show-error \
  https://horizon-testnet.stellar.org/accounts/GBPC4G6LJDBXU6CA4APP2DZZXIBULI6ULXKAGE2YTLHS2BFCGIZ67N6O
curl --include --silent --show-error \
  https://horizon-testnet.stellar.org/transactions/b7952b310286ecf301de92239fe57ea3597057edc57208f3670e44bd1b5f91c2
```

## Scope and reproduction limits

Run the offline checks first:

```sh
cd tools/replay
npm test
node replay.mjs --dry-run --only fx-01-raise-high-with-lost-signer --out /tmp/fx-01-followup-dry-run.json
```

The dry run signs in memory and makes no network requests; it reports
`not_submitted` rather than a network match. A live reproduction must again
verify the current Testnet root, network passphrase, active protocol, health,
and Core version before the guarded command above is allowed to submit.

This is a single fresh-account replay of FX-01 Case 1b. It does not repeat the
other original follow-up attempts, prove key custody, prove account state at a
later time, or change the Protocol-29-only support boundary. Issue
[#3](https://github.com/signerreach/signerreach/issues/3) remains open for
maintainer review of this evidence and any decision about additional cases.
