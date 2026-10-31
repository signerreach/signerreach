# SignerReach

An offline checker for changes to Stellar classic-account signers and
thresholds. Before you submit a change, it tells you whether the account can
still be administered afterwards, under the assumptions you state.

[![CI](https://github.com/signerreach/signerreach/actions/workflows/ci.yml/badge.svg)](https://github.com/signerreach/signerreach/actions/workflows/ci.yml)

> **Status: early and unreleased.** The tool runs end to end and is tested, but
> it has not been audited and there is no release yet. It cannot know whether
> anyone holds a key. Treat a result as an aid to your own review, not as a
> guarantee. SignerReach is an independent project, not affiliated with or
> endorsed by the Stellar Development Foundation.

## The problem

On a Stellar classic account, changing signers or thresholds needs the account's
**high** threshold. A change can leave behind a configuration that is valid but
can never be changed again.

Example: an account has two signers of weight 1 each, and every threshold is 1.
Raising the high threshold to 2 looks routine and succeeds. If one of the two
keys is later lost, no combination of remaining keys reaches 2, so nobody can
ever change the signers again.

SignerReach looks for this kind of outcome before you submit the change.

## What it does

You give it three files:

| File | Content |
|---|---|
| `snapshot.json` | The account: signers, weights, thresholds, and the ledger it was read at |
| `plan.json` | The transactions you intend to submit, in order, and who will sign each |
| `availability.json` | Which signers you can use, which are lost, and which are unknown, with optional "what if" scenarios |

It answers one of three things, with the reasoning shown:

- **`SAFE WITHIN SCOPE`**: a path to administer the account remains, in every
  scenario you declared.
- **`UNSAFE`**: the plan would be rejected, or administrative access would be
  lost.
- **`INCONCLUSIVE`**: the result cannot be established from what is declared and
  modelled. This includes any input it does not understand.

The shipped `signerreach` binary never holds private signing keys, signs, or
submits anything. The `check` command makes no network connection; only the
optional `fetch` command does, and it only reads. The separate development
replay harness creates temporary Testnet keys to reproduce fixtures.

## Example

The files for this case are in
[`fixtures/fx-01-raise-high-with-lost-signer`](fixtures/fx-01-raise-high-with-lost-signer).

```bash
cargo run -p signerreach-cli -- check \
  --snapshot fixtures/fx-01-raise-high-with-lost-signer/snapshot.json \
  --plan fixtures/fx-01-raise-high-with-lost-signer/plan.json \
  --availability fixtures/fx-01-raise-high-with-lost-signer/availability.json
```

The report (shortened) begins:

```
VERDICT: UNSAFE
Reason: signers declared available weigh 1, below the HIGH threshold 2
...
Diagnostics
-----------
  SR-LOSS-01 [UNSAFE] (baseline) signers declared available weigh 1, below the HIGH threshold 2
```

The full report also shows each transaction's signers, weights and thresholds,
and whether each threshold can be reached under every scenario. Add
`--format json` for a machine-readable report. A longer example, with the
network evidence behind it and a corrected plan, is in
[`docs/walkthrough`](docs/walkthrough/README.md).

### Exit codes

| Code | Meaning |
|---|---|
| 0 | `SAFE WITHIN SCOPE` |
| 1 | `UNSAFE` |
| 2 | `INCONCLUSIVE`, including input that was rejected |
| 64 | Usage error, or a file that cannot be read |

## Building

There is no release yet, so build from source. You need Rust 1.87 or later and
nothing else.

```bash
cargo build --release
```

The binary is `target/release/signerreach`.

## Making a snapshot

`fetch` reads an account from a Horizon server and prints a snapshot. It only
reads, and it writes no file, so save the output yourself:

```bash
signerreach fetch --network testnet --account G... > snapshot.json
```

Use `--horizon-url` to read from a server of your own (https, or http on your own
machine). It exits 0 with a snapshot, 64 for bad arguments, and 69 if the account
could not be read. The snapshot records the ledger and protocol version the server
reported, and `check` reports `INCONCLUSIVE` for any protocol other than 29.

Or write the file yourself, from an account record such as Horizon's:

| In the snapshot | From the account record |
|---|---|
| `account` | the account address (`G...`) |
| `master_weight` | the weight of the signer entry whose key is the account itself |
| `signers` | every other signer entry, with `"type": "ed25519"` |
| `thresholds` | `low_threshold`, `med_threshold`, `high_threshold` |
| `provenance.as_of_ledger` | the ledger you read it at, taken from the same fetch |

Signers of any other type (pre-authorized transactions, hash(x), signed payloads)
are not supported, and the tool reports `INCONCLUSIVE` if it sees one. The exact
format is in section 5 of [`docs/SPEC.md`](docs/SPEC.md).

## What it learned from the network

The rules were not assumed. They were read in stellar-core and then observed on
Testnet at protocol 29: [`docs/evidence/testnet-2026-10-13`](docs/evidence/testnet-2026-10-13/README.md).
The chain fixtures were replayed on a fresh Testnet on 2026-10-19, and every
outcome matched: [`docs/evidence/replay-2026-10-19`](docs/evidence/replay-2026-10-19/README.md).
One finding matters most:

> All operations in a transaction are authorized against the account **as it was
> before that transaction**. The same two changes can be accepted as one
> transaction and rejected as two.

A tool that simply adds up final weights gets this wrong. SignerReach models
transactions as the unit, and keeps both cases as test fixtures
(`FX-05a` and `FX-05b`).

## How it differs from other tools

Read on 2026-10-13, from their public source. The source files that were
read had not changed when checked again on 2026-10-20. Corrections are welcome.

- **Stellar Laboratory** shows a fixed warning that a change can lock an account.
  It does not analyse the change.
- **Stellar Stratum** ([`tupui/stellar-stratum`](https://github.com/tupui/stellar-stratum))
  is an app for building and signing multisig transactions. It checks the
  resulting configuration and blocks one whose thresholds exceed the total signer
  weight.
- **stellar-toml-lint** ([`anchor-tools/stellar-toml-lint`](https://github.com/anchor-tools/stellar-toml-lint),
  pull request 266) audits an account's current signers and thresholds, and
  reports thresholds that the total weight cannot reach. It looks at the current
  state, not at a planned change.
- **js-stellar-sdk** has an open issue,
  [#1606](https://github.com/stellar/js-stellar-sdk/issues/1606), proposing a
  function that checks whether a transaction's signatures are sufficient. That
  is about current signatures, not planned changes.

Closed-source wallets and dashboards were not examined.

SignerReach does not repeat the final-state check as its headline. What it adds is
**availability scenarios** ("what if this key is lost"), **ordered plans** of
several transactions evaluated one after another, and use **offline and in
scripts**, with a reproducible trace.

## Limits

- Availability is what you declare. Nothing here can check that a key exists.
- A snapshot goes stale. The result applies to the ledger it was read at.
- Only protocol 29 rules are verified, only `SetOptions` operations, and only
  ed25519 signers on a single classic account.
- Not modelled, and so reported `INCONCLUSIVE`: fee-bump transactions,
  preconditions, other source accounts, sponsorship and reserves, and Soroban
  contract accounts.

Every report ends with a coverage list stating what was and was not modelled.

## Project layout

| Path | Content |
|---|---|
| `crates/signerreach-core` | The evaluation: pure, with no file, clock or network access |
| `crates/signerreach-stellar` | Turns Horizon data into snapshots; the only code that opens a network connection, and it only reads |
| `crates/signerreach-cli` | The `signerreach` command; the only code that touches files or the terminal |
| `tools/replay` | A Node script that replays the chain fixtures on Testnet; a development tool, not part of the binary |
| `docs/SPEC.md` | The specification: inputs, rules, verdicts, diagnostic codes |
| `docs/evidence` | What was observed on a real network |
| `fixtures` | The test cases, each with its expected result |

## Contributing and security

See [`CONTRIBUTING.md`](CONTRIBUTING.md). To report a problem privately,
especially a result that says `SAFE WITHIN SCOPE` when it should not, see
[`SECURITY.md`](SECURITY.md).

## License

[Apache License 2.0](LICENSE).
