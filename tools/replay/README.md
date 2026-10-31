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
  construct a Horizon client or call Friendbot.
- `--only TEXT` runs the fixtures whose directory name contains `TEXT`.
- Exit code 0: every outcome matched. 1: an outcome differed. 2: the run could
  not be completed.

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
- **It does not tell a rejected transaction from one that failed to apply.**
  Both come back from Horizon as an error with result codes, and the tool
  records them as `rejected`. The chain fixtures expect only `accepted` and
  `rejected`; a fixture that expects `failed_to_apply` is reported as differing.
- **It does not replay the follow-up attempts** of the Phase 1 evidence (for
  example, trying to change a threshold after a plan that locked the account).
  It checks what the plan's own transactions did.

## When to run it

When Testnet moves past protocol 29, first verify the relevant authorization
rules against official protocol sources and new network evidence. Then update
the explicit protocol guard and fixtures before live replay. The historical
evidence stays unchanged. The output records the protocol and core versions.
