# SignerReach submission pack

Prepared for maintainer review. Current state updated 2026-11-02 after v0.1.0 publication and final read-only audit.
This is a draft information pack, not a submission, release announcement,
approval request or funding claim.
The [readiness audit](../docs/readiness-audit.md) records the remaining gates.

## Current state after publication (2026-11-02)

PR [#17](https://github.com/signerreach/signerreach/pull/17) merged to
upstream `main` at `e1a37cea9101bbf2cb68372f111c6e4a5ecd1ba4`. SignerReach
v0.1.0 was published, not as a draft or prerelease, on November 2, 2026.
Annotated tag `v0.1.0` (tag object
`2a01d2eedb4800fa7b443f344eb931a7f1eadb5f`) points to that exact source
commit, and the [publication workflow](https://github.com/signerreach/signerreach/actions/runs/41451872483)
succeeded.

The [GitHub Release](https://github.com/signerreach/signerreach/releases/tag/v0.1.0)
contains nine verified assets: archives, checksums and
manifests for Linux `x86_64-unknown-linux-gnu`, macOS `aarch64-apple-darwin`,
and Windows `x86_64-pc-windows-msvc`. A download verification confirmed archive
SHA-256 values and manifest source commits. [Final-main CI](https://github.com/signerreach/signerreach/actions/runs/41447399997)
and the [documentation build](https://github.com/signerreach/signerreach/actions/runs/41447399730)
passed. The published [mdBook](https://signerreach.github.io/signerreach/)
returns HTTP 200, and the repository Homepage field points to that URL.

Release publication does not establish program eligibility. No program
application has been submitted, accepted or approved. The final audit below
records that eligibility and program-specific requirements remain unverified.
Detailed branch-protection verification remains the last operational task.
Issues [#1](https://github.com/signerreach/signerreach/issues/1),
[#2](https://github.com/signerreach/signerreach/issues/2) and
[#7](https://github.com/signerreach/signerreach/issues/7) remain open.

## Final read-only audit (2026-11-02)

The latest verified `main` is
`1c4c92ee4e26aba2b08ca50dec905e9097108ef1`; PR #19 merged the README cleanup.
Current-main [CI](https://github.com/signerreach/signerreach/actions/runs/41459984154)
and [documentation build](https://github.com/signerreach/signerreach/actions/runs/41459984009)
passed. The push-triggered Pages deploy was skipped, so the existing manual
[Documentation workflow](https://github.com/signerreach/signerreach/actions/runs/41464392358)
was run on that commit and deployed successfully. The live book and its
release-status page return HTTP 200 and show the publication details from PR
#18. The v0.1.0 release remains from source commit
`e1a37cea9101bbf2cb68372f111c6e4a5ecd1ba4`; its tag and nine verified assets
were not changed.

The code sets `SUPPORTED_PROTOCOL_VERSION` to 29 and rejects other snapshot
versions. Its Horizon regression test confirms a Protocol 30 response cannot
produce a safe verdict. The supported release scope remains Protocol 29, one
classic G-address account, ed25519 signers and the implemented `SetOptions`
subset.

The unsafe FX-01 example is backed by historical Protocol 29 Testnet evidence.
The corrected plan at `docs/walkthrough/corrected-plan.json` returns
`SAFE WITHIN SCOPE` locally, but that exact plan was not submitted to Testnet;
the mechanism is supported by separate Protocol 29 cases. Historical evidence
does not prove current network behavior. `check` is offline and the shipped CLI
does not hold keys, sign or submit transactions. `fetch` is constrained and
read-only. The separate development replay harness can submit disposable
Testnet transactions when explicitly run; this audit made no Testnet
submission.

The [security policy](../SECURITY.md) and [security review](../docs/security-review.md)
disclose that there has been no independent security audit. Signer availability
is declared, snapshots can become stale, and reserve sufficiency and other
external conditions are not guaranteed.

On this audit date, the Stellar Development Foundation's [Bell announcement](https://stellar.org/blog/developers/introducing-bell-protocol-30-on-stellar)
listed the Protocol 30 Testnet upgrade vote for November 3, 2026 at 17:00 UTC;
its proposed dates may change. If the vote passes and Testnet moves to Protocol
30, `fetch` may record version 30, but the checker will reject that snapshot as
unsupported and return `INCONCLUSIVE`. Verify the current network protocol
before any future live demonstration. After a Testnet upgrade, do not present
the shipped release as a supported live checker; Protocol 30 requires separate
implementation and verification. The existing Protocol 29 fixtures remain
historical offline demonstrations.

Technical evidence and program eligibility are separate. Eligibility,
program-specific requirements, adoption evidence and an application plan are
unverified. No award, invitation, budget, application, acceptance or approval
is claimed. **NOT READY FOR PROGRAM SUBMISSION** until those matters and the
submission contents are verified. Detailed branch-protection verification
remains the final operational task after the other readiness work.

## Historical pre-publication release-preparation record (2026-11-02)

This section records the release-preparation state before publication and is
kept as historical context. The current release status is recorded above.

The Rust workspace is now versioned `0.1.0`, with `publish = false` retained
and all internal crate requirements and `Cargo.lock` aligned. This branch
prepares a release candidate; it has not created a tag or GitHub Release and
does not authorize crates.io publication. Draft notes are in
[`RELEASE_NOTES_v0.1.0.md`](../RELEASE_NOTES_v0.1.0.md).

The candidate scope is the implemented `check` and `fetch` CLI, text and JSON
reports, Protocol 29 rules, fixtures, evidence and the development-only replay
harness. Native Linux, macOS and Windows packages are prepared through a
review-only CI workflow. [Run 41431206670](https://github.com/signerreach/signerreach/actions/runs/41431206670)
completed all three native jobs from source commit
`9d7985f6dfec05f92db472de6304212f16ffcc6c`; its manifests record the exact
target triples, toolchains and checksums in the [draft release notes](../RELEASE_NOTES_v0.1.0.md#candidate-artifacts).
These are review artifacts, not release assets, and must be rebuilt from the
final approved main commit. No live Testnet submission is part of this
preparation.

## Pre-publication repository status snapshot (2026-11-02)

This snapshot predates the v0.1.0 publication. Its release and main-commit
statements are preserved as historical findings and superseded by the current
state above.

PRs [#12](https://github.com/signerreach/signerreach/pull/12), [#13](https://github.com/signerreach/signerreach/pull/13) and the documentation PR [#15](https://github.com/signerreach/signerreach/pull/15) are merged. The current main before this preparation branch is `15105788b60f8d3d05188cbe0d23d6ad51dbafe6`; its Pages documentation is published at [signerreach.github.io/signerreach](https://signerreach.github.io/signerreach/). The latest evidence PR reported 278 Rust tests and 51 Node replay tests. Initial follow-up scope issue [#3](https://github.com/signerreach/signerreach/issues/3) is closed. [The November 1 FX-01 replay](../docs/evidence/replay-2026-11-01-fx-01-followup/README.md) observed a new Protocol 29 submission rejection with `tx_failed` / `op_bad_auth`; it does not verify Protocol 30. The older October tests and CI details below remain historical observations, not current-head claims. The v0.1.0 release and its tag remain preparatory, not published.

## Project

**SignerReach** is a published Rust command-line tool, currently at v0.1.0,
that reviews planned changes to a Stellar classic account's signers and
thresholds before the user submits those changes. Repository:
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
There is no hosted app, smart contract or API server. The published release is
v0.1.0 from commit `e1a37cea9101bbf2cb68372f111c6e4a5ecd1ba4`; crates.io
publication remains disabled (`publish = false`).

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
sources and limits. [PR #6](https://github.com/signerreach/signerreach/pull/6)
from Hollujay's fork merged into upstream `main` as `be8b082e9c6e5dad4be2ab4f9ddd5d353e39e702` on
2026-10-31. The [post-merge main CI](https://github.com/signerreach/signerreach/actions/runs/41182773470) passed all six
jobs, including 277 Rust tests, minimum Rust verification, the offline replay
dry run, formatting/linting, cross-platform tests and the Ubuntu release
build. The earlier 269-test run on the pre-merge commit remains historical
evidence, not the current main-branch result. Current final-main CI is linked
in the post-publication status above.

Real Testnet observations from [2026-10-13](../docs/evidence/testnet-2026-10-13/README.md)
include accepted changes and rejected follow-up lockout attempts at protocol
29. A [2026-10-19 replay](../docs/evidence/replay-2026-10-19/README.md)
repeated seven chain fixture plans at protocol 29 and matched their expected
outcomes, but did not repeat the follow-up attempts. Both records are
historical and do not establish current network or Protocol 30 compatibility.
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

## Security and open work

The [security policy](../SECURITY.md) uses GitHub private vulnerability
reporting. The [security review](../docs/security-review.md) records fixed
findings, tests and outstanding limits. The CLI has no key custody. The replay
harness generates temporary Testnet keypairs in memory and must not print or
persist their secrets. This is not an independent security audit.

Contributor work remains open: [#1 JSON Schema command](https://github.com/signerreach/signerreach/issues/1), [#2 Markdown report](https://github.com/signerreach/signerreach/issues/2), and [#7 Protocol 30 verification](https://github.com/signerreach/signerreach/issues/7). The initial follow-up replay scope in [#3](https://github.com/signerreach/signerreach/issues/3) is closed.

## Historical pre-publication release decision

A coherent first release contains the implemented `check` and `fetch`
commands, text and JSON reports, protocol 29 support, fixtures, source
documentation and the stated limitations. It requires reviewed native release
artifacts, current CI, release notes and maintainer authorization. At the time
of that assessment, no tag or GitHub Release had yet been created. The current
[SCF Build Award information](https://communityfund.stellar.org/awards)
lists tracks, but this project's eligibility and any application choice remain
unverified. No application has been submitted and no approval or funding is
claimed.
