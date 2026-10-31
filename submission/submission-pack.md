# SignerReach submission pack

Prepared for maintainer review on 2026-10-31. This is a draft information pack,
not a submission, approval request, release announcement or funding claim.
The [readiness audit](../docs/readiness-audit.md) records the remaining gates.

## Project

**SignerReach** is an unreleased Rust command-line tool that reviews planned
changes to a Stellar classic account's signers and thresholds before the user
submits those changes. Repository:
[github.com/signerreach/signerreach](https://github.com/signerreach/signerreach).

The intended users are people and teams administering multisig Stellar
accounts or rotating keys. An accepted change can leave HIGH unreachable with
the keys still available, preventing later administrative changes. The
[problem record](../research/problem-validation.md) explains the demonstrated
technical failure and the unverified adoption assumptions.

Stellar is essential to the mechanism. The checker models Stellar's LOW
envelope authorization, HIGH authorization for signer and threshold changes,
signature use, and the rule that every operation in one transaction is
authorized against its pre-state. The [specification](../docs/SPEC.md),
[architecture](../docs/architecture.md) and [ecosystem research](../research/ecosystem-recon.md)
link the protocol sources and observations.

## What exists

- `signerreach check` reads a snapshot, ordered transaction plan and signer
  availability declaration. It produces deterministic text or JSON with an
  authorization trace, diagnostics, coverage and one of `SAFE WITHIN SCOPE`,
  `UNSAFE` or `INCONCLUSIVE`.
- `signerreach fetch` reads an account from Horizon into a snapshot. It uses
  read-only GET requests and validates the network and account response.
- Rust unit and integration tests cover parsing, authorization, state changes,
  scenarios, reports, CLI exit codes and the Horizon adapter. Chain-backed and
  synthetic fixtures are identified separately.
- A separate Node replay harness can reconstruct and submit chain fixture
  plans on Testnet. Its dry run builds and signs without network requests.

The shipped binary never holds private signing keys or submits transactions.
There is no hosted app, smart contract, API server, database or release.
Workspace version remains `0.0.0`.

## Supported scope and evidence

Only protocol 29 is verified for the checker's rules. It handles one classic
G-address account, ed25519 signers, and the modeled `SetOptions` subset.
Recognized unsupported inputs and later protocol versions give
`INCONCLUSIVE`. Account flag changes also give `INCONCLUSIVE` because the
snapshot does not hold the existing flags. See the [limitations](../docs/architecture.md#inputs-and-trust-boundaries)
and [security review](../docs/security-review.md).

Local verification on 2026-10-31 ran `cargo test --workspace --locked` with
277 passing tests, and the same workspace tests on Rust 1.87.0. The replay
dry run built and signed seven chain fixtures with no submissions; a network
system-call trace observed no network calls. `cargo audit` and
`npm audit --omit=dev` reported no known advisories after narrow lockfile
corrections. The [evidence index](../evidence/index.md) records the exact
sources and limits. The base `main` commit's [historical CI run](https://github.com/signerreach/signerreach/actions/runs/39461300547)
passed on 2026-10-20 with 269 Ubuntu tests and its cross-platform jobs. It is
not evidence that this branch has passed hosted CI. [PR #5](https://github.com/signerreach/signerreach/pull/5)
is waiting for maintainer approval to run its workflow; its new checks have
not executed on GitHub-hosted CI.

Real Testnet observations from [2026-10-13](../docs/evidence/testnet-2026-10-13/README.md)
include accepted changes and rejected follow-up lockout attempts at protocol
29. A [2026-10-19 replay](../docs/evidence/replay-2026-10-19/README.md)
repeated seven chain fixture plans at protocol 29 and matched their expected
outcomes, but did not repeat the follow-up attempts. Both records are
historical and do not establish current live or Protocol 30 compatibility.
The [documentation index](../docs/README.md) gives a short path through the
CLI, schemas, examples, security model and reproduction steps.

## Reproducible demonstration

Run from the repository root after `cargo build --locked`:

```bash
target/debug/signerreach check \
  --snapshot fixtures/fx-01-raise-high-with-lost-signer/snapshot.json \
  --plan fixtures/fx-01-raise-high-with-lost-signer/plan.json \
  --availability fixtures/fx-01-raise-high-with-lost-signer/availability.json
```

The 2026-10-31 local run exited 1 and included:

```text
VERDICT: UNSAFE
Reason: signers declared available weigh 1, below the HIGH threshold 2
    result: accepted
  SR-LOSS-01 [UNSAFE] (baseline) signers declared available weigh 1, below the HIGH threshold 2
```

Then run the corrected plan:

```bash
target/debug/signerreach check \
  --snapshot fixtures/fx-01-raise-high-with-lost-signer/snapshot.json \
  --plan docs/walkthrough/corrected-plan.json \
  --availability fixtures/fx-01-raise-high-with-lost-signer/availability.json
```

The local run exited 0 and included:

```text
VERDICT: SAFE WITHIN SCOPE
    check operation 0 [HIGH]: needs 1, signatures give 1: met
    check operation 1 [HIGH]: needs 1, signatures give 1: met
    HIGH   threshold   2  total 3  available 2  unknown 0: reachable
```

This corrected plan was evaluated locally but was not itself submitted to
Testnet. Its authorization mechanism is backed by separate protocol 29 cases.
The verdict assumes an accurate snapshot, obtainable signatures and declared
key availability. Reserves and live state changes remain outside the model.

## Security, open work and release decision

The [security policy](../SECURITY.md) uses GitHub private vulnerability
reporting. The [security review](../docs/security-review.md) records fixed
findings, tests and outstanding limits. The CLI has no key custody. The replay
harness generates temporary Testnet keypairs in memory and must not print or
persist their secrets. This is not an independent security audit.

Contributor work remains open: [#1 JSON Schema command](https://github.com/signerreach/signerreach/issues/1),
[#2 Markdown report](https://github.com/signerreach/signerreach/issues/2),
[#3 follow-up replay format discussion](https://github.com/signerreach/signerreach/issues/3),
and [#4 Protocol 30 verification](https://github.com/signerreach/signerreach/issues/4).
No issue has been closed to improve appearances.

A coherent first release could contain the implemented `check` and `fetch`
commands, text and JSON reports, protocol 29 support, fixtures, source
documentation and the stated limitations. It would require an intentional
version, release build, current CI, release notes and maintainer authorization.
No tag or release has been created. The current
[SCF Build Award information](https://communityfund.stellar.org/awards)
lists tracks, but this project's eligibility and any application choice remain
unverified. No application has been submitted and no approval or funding is
claimed.
