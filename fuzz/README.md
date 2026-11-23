# Fuzzing

These targets feed random and mutated bytes to the parts of SignerReach that read
data from outside: the three input documents, the Horizon account record, and the
small text parsers in the network layer. They use
[cargo-fuzz](https://github.com/rust-fuzz/cargo-fuzz) (libFuzzer) and need a
nightly toolchain on Linux or macOS.

| Target | What it reads | What must hold |
|---|---|---|
| `parse_documents` | snapshot, plan and availability text | never panics; a rejection always carries at least one issue |
| `evaluate_report` | three documents, parsed, evaluated and rendered | never panics; the JSON report is valid JSON and states the evaluation's verdict; the same input gives byte-identical reports; a rejection report is valid JSON |
| `horizon_mapping` | a Horizon account record, and whether the snapshot carries `reserve` data | never panics; if the mapping writes a snapshot, `check` can read it (it may still refuse it on purpose for a signer it does not model or a sponsored signer) |
| `network_text` | server address, `Date` header, server root (also read as a ledger record), `Latest-Ledger` header | never panics; an accepted address round-trips to itself; an accepted date is `YYYY-MM-DDTHH:MM:SSZ`; an accepted ledger is at least 1; an accepted ledger record has a sequence and a base reserve of at least 1 |

A target that finds a failing input saves it under `fuzz/artifacts/<target>/`.
That file reproduces the failure on its own.

## Run it

```sh
rustup toolchain install nightly --profile minimal
cargo install cargo-fuzz --version 0.13.2 --locked
bash fuzz/seed.sh                              # build the starting corpus from fixtures/
cargo +nightly fuzz run parse_documents fuzz/corpus/parse_documents -- -max_total_time=60
```

Replace `parse_documents` with any target. To replay one saved failure:

```sh
cargo +nightly fuzz run parse_documents fuzz/artifacts/parse_documents/crash-<hash>
```

## In CI

`.github/workflows/fuzz.yml` runs every target for a short time on pull requests
that change `crates/` or `fuzz/`, and for longer once a week. A failing input is
uploaded as a build artifact.

A short run is a smoke test, not a proof. The targets show that the inputs tried so
far did not break the stated properties. They do not show there is no input that
does.
