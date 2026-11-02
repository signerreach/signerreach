# Get started

SignerReach is a Rust command-line program. The released binary is not yet published. Build it from the source repository with Rust **1.87 or newer**.

## Build

```sh
git clone https://github.com/signerreach/signerreach.git
cd signerreach
cargo build --release --locked
```

The executable is `target/release/signerreach` on Linux and macOS, or `target/release/signerreach.exe` on Windows.

## Run an unsafe example

The repository contains a fixture that describes an account whose HIGH threshold will become unreachable if a signer is lost:

```sh
cargo run --locked -p signerreach-cli -- check \
  --snapshot fixtures/fx-01-raise-high-with-lost-signer/snapshot.json \
  --plan fixtures/fx-01-raise-high-with-lost-signer/plan.json \
  --availability fixtures/fx-01-raise-high-with-lost-signer/availability.json
```

This example reports `VERDICT: UNSAFE` because the still-available signer weight is 1 while HIGH is 2. The report also explains each transaction and the declared scenarios.

## Compare the corrected plan

```sh
cargo run --locked -p signerreach-cli -- check \
  --snapshot fixtures/fx-01-raise-high-with-lost-signer/snapshot.json \
  --plan docs/walkthrough/corrected-plan.json \
  --availability fixtures/fx-01-raise-high-with-lost-signer/availability.json
```

The repository's [walkthrough](https://github.com/signerreach/signerreach/blob/main/docs/walkthrough/README.md) explains the corrected case and its assumptions. This corrected plan was tested locally, but has not itself been submitted to Testnet.

## Use a real account snapshot

`fetch` reads Horizon account data and produces a snapshot. It **does not submit** a transaction:

```sh
target/release/signerreach fetch --network testnet --account G... > snapshot.json
```

Before trusting the result, check the network, provenance ledger, supported protocol and signer availability. `check` itself stays offline. See the [CLI reference](cli.md) and [limitations](security.md).
