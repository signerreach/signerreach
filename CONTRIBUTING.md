# Contributing to SignerReach

Thank you for helping. SignerReach is a small tool with one job: tell someone,
before they change the signers or thresholds of a Stellar classic account,
whether the account can still be administered afterwards. A wrong "safe" answer
can cost someone their account, so this project values being correct over being
broad.

## Before you start

Read these three things first. They are short.

- [`docs/SPEC.md`](docs/SPEC.md): what the tool does, the rules it relies on, and
  the exact meaning of each result.
- [`fixtures/README.md`](fixtures/README.md): the test cases and how they are
  written.
- [`docs/evidence/testnet-2026-10-13/README.md`](docs/evidence/testnet-2026-10-13/README.md):
  the observations on a real network that the rules rest on. A second run,
  [`docs/evidence/replay-2026-10-19`](docs/evidence/replay-2026-10-19/README.md),
  repeated the chain fixtures.

For anything larger than a small fix, open an issue and say what you plan to do
before you write it. That avoids work that cannot be merged.

## Ground rules

These come from the purpose of the tool and are not negotiable.

- **When unsure, say so.** Anything the tool does not model must give
  `INCONCLUSIVE`, never `SAFE WITHIN SCOPE`. A result that is too cautious is a
  small bug. A result that is too confident is a serious one.
- **The Rust core does no input or output.** `signerreach-core` never reads a
  file, opens a connection or reads the clock. In the shipped Rust workspace,
  only `signerreach-cli` touches files or the terminal, and only
  `signerreach-stellar` opens a network connection. That connection only reads.
  Tests check that `check` cannot reach it. The separate replay harness reads
  fixtures and can submit transactions to Testnet.
- **Protect secrets.** The shipped CLI needs no private key, seed or signature.
  The replay harness creates temporary Testnet keys and must never print,
  persist or disclose their secrets. Reports deliberately display validated
  public addresses, signer keys, transaction IDs, scenario names and home
  domains. Errors must not echo unvalidated document values or file names.
  Never put a secret in a field intended for public display.
- **Same input, same output.** Reports are reproducible byte for byte. Do not add
  timestamps, random values or unordered maps to output.
- **No unsafe code.** It is forbidden for the whole workspace.
- **No claims you cannot back.** Do not write that the tool is audited, adopted,
  approved or funded. Do not describe a synthetic test as evidence about the
  network.

## Setting up

You need Rust 1.87 or later. Nothing else: no Docker, no network and no Stellar
account are required to build and test. CI also builds and tests on exactly 1.87,
so do not use a feature that needs a newer compiler without raising
`rust-version` in `Cargo.toml` and the version in `.github/workflows/ci.yml`
together, and say why in the pull request. The tests that fetch use a
fake server on your own machine. Only the optional
[`tools/replay`](tools/replay/README.md) script needs Node and the live Testnet.

```bash
cargo build
```

Before you open a pull request, these three commands must pass. They are the
same ones CI runs, on Linux, Windows and macOS.

```bash
cargo fmt --all -- --check
```

```bash
cargo clippy --workspace --all-targets --locked -- -D warnings
```

```bash
cargo test --workspace --locked
```

## Changing behaviour

Work in this order, so the tests check the tool instead of echoing it.

1. **Spec first.** If the change affects what the tool says, update
   `docs/SPEC.md` in the same pull request.
2. **Then the fixtures.** Add or change a case under `fixtures/`. Write
   `expected.json` by hand from the specification, not by copying what the tool
   printed. Observed parts of a `chain` fixture must never be changed to make the
   tool pass.
3. **Then the code**, with unit tests next to it.

A rule about how Stellar behaves needs evidence from the network or from the
stellar-core source. Say which, and link it.

## Dependencies

Prefer none. A new dependency needs a reason in the pull request: what it does,
why the standard library or an existing dependency is not enough, and how it is
maintained. Check that it builds on all three platforms.

## Commits and pull requests

- **One logical change per commit**, with its tests. Do not split a single change
  into many commits, and do not add commits only to raise activity.
- **Messages follow the conventional style**: `feat(core): ...`, `fix(cli): ...`,
  `docs: ...`, `test: ...`, `ci: ...`, `build: ...` or `chore: ...`. The subject
  says what changed. The body says why.
- **Keep pull requests focused.** One purpose each. Say what you changed, why, and
  how you checked it.
- **Understand and run what you submit.** Contributions that were generated and
  not tested, or that you cannot explain, will not be merged.
- **CI must pass** before a review is useful.

## Reporting problems

- A wrong result, a crash or a confusing message: open an issue with the three
  input files (with no secrets in them), the output, and what you expected.
- A false `SAFE WITHIN SCOPE`, or anything touching secrets: do **not** open a
  public issue. Follow [`SECURITY.md`](SECURITY.md).

## License

By contributing you agree that your contribution is licensed under the
[Apache License 2.0](LICENSE), as in section 5 of that license. There is no
separate agreement to sign.
