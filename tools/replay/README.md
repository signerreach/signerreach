# Testnet replay

Replays the `chain` fixtures in [`fixtures/`](../../fixtures/README.md) on Stellar
Testnet and checks that the network did what each fixture says.

This is a development tool. It signs and submits transactions, which the
`signerreach` binary never does, so it lives outside the Rust workspace. It is
not run live in CI because it needs the network. The offline dry run can run in CI.

## What it does

For each fixture whose `expected.json` has `"source": "chain"`:

1. Generates a fresh keypair for every address in the fixture, and funds the
   fixture's account with Friendbot.
2. Sets the account to the fixture's `snapshot.json` (master weight, thresholds,
   signers) and reads it back to confirm it matches.
3. Submits each transaction of `plan.json`, signed by the keys it lists in
   `signed_by`.
4. Compares what happened with `transactions` in `expected.json`, and records the
   ledger and transaction hash of each accepted transaction.
5. If `after.json` exists, runs its ordered replay-only attempts after every
   planned transaction matched. It uses only explicitly listed fixture signers
   declared `available`, reloads account sequence before every attempt, and
   verifies the resulting account signer/threshold state and sequence.

`after.json` has schema `signerreach.replay-after/1`; see
[`fixtures/README.md`](../../fixtures/README.md#replay-only-follow-up-attempts)
for the full format. Its presence does not change any input to `signerreach
check` or the fixture's `expected.json` verdict.

Follow-up expected stages are `accepted`, `rejected_at_submission`, and
`failed_to_apply`. Successful and included failed transactions must be found in
Horizon history with a matching hash, ledger, and XDR result. A submission
rejection is recorded only when Horizon returns structured result codes, a
bounded history lookup does not find the hash, and account sequence and
authorization state remain unchanged. The output labels the number of checks
and explicitly records that bounded absence is not absolute proof of
non-inclusion. An unresolved lookup or inconsistent state is `unconfirmed`,
never a match, and ends the fixture's follow-up sequence. Ordered operation
result codes are compared exactly. New replay output keeps only allowlisted,
sanitized Horizon response fields, with each retained string capped at 24 KiB.

A fixture with no transactions is a state check. For it the tool submits a
change-nothing transaction signed by every signer, and expects it to be accepted
only if the summed weights reach the LOW threshold. That expectation comes from
the stated rule, not from `expected.json`, and the output labels it as a probe.

## Run it

You need Node 22.12 or later.

```bash
cd tools/replay
npm ci --ignore-scripts
node replay.mjs --dry-run
node replay.mjs --out results.json
```

- `--dry-run` builds and signs every transaction but sends nothing. It does not
  construct a Horizon client or call Friendbot. It validates and builds
  `after.json` attempts too; they show `not_submitted` and never count as a
  match against network expectations.
- `--only TEXT` runs the fixtures whose directory name contains `TEXT`.
- Exit code 0: every outcome matched. 1: an outcome differed. 2: the run could
  not be completed or a follow-up outcome was unconfirmed/not run.

Progress goes to standard error. The results are JSON, on standard output or in
the `--out` file.

## What it does not do

- **It never touches mainnet.** The server address is fixed, and the run stops if
  the server's network passphrase is not Testnet's.
- **Live replay requires protocol 29.** The script refuses a different live
  protocol before funding an account or submitting a transaction. The SDK is
  pinned to 17.3.0, the first published JavaScript SDK release with Protocol 30
  XDR, but this does not verify SignerReach's Protocol 30 rules.
- **Secret keys exist only in memory.** They are not written or printed.
- Existing `expected.json` plan outcomes retain their historical `accepted` /
  `rejected` model. The separate follow-up path distinguishes a submission
  rejection from an included `tx_failed` transaction by querying transaction
  history and checking account sequence/state. A bounded missing-history result
  is cautious evidence only, not proof that no inclusion can ever appear.
- Initial follow-up coverage is FX-01 Case 1b only. Do not add controls that
  require signers marked `lost` or `unknown` without a separate decision.

## When to run it

When Testnet moves past protocol 29, first verify the relevant authorization
rules against official protocol sources and new network evidence. Then update
the explicit protocol guard and fixtures before live replay. The historical
evidence stays unchanged. The output records the protocol and core versions.
