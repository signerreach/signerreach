# Evidence and claim ledger

Checked 2026-10-31. This is the central index for what the repository can
support. Historical Testnet observations remain in their original directories.
`TESTED LOCALLY` means this review ran the named command; it does not imply
hosted CI or independent audit. `VERIFIED` means a source or observation directly
supports the narrow claim shown. `LOGICALLY COVERED` means code or tests support
the stated mechanism without a new live network observation. `UNVERIFIED` and
`KNOWN LIMITATION` keep those boundaries explicit. `BLOCKED` marks an external
gate that has not passed.

## Evidence index

| Claim | Evidence type | Exact source | Date checked | Status |
|---|---|---|---|---|
| Protocol 29 `SetOptions` authorization uses LOW for the envelope and the required operation threshold | Stellar Core source and historical Testnet observation | [Specification rules](../docs/SPEC.md#4-verified-rules-this-specification-relies-on), [Stellar Core TransactionFrame](https://github.com/stellar/stellar-core/blob/master/src/transactions/TransactionFrame.cpp), [SetOptionsOpFrame](https://github.com/stellar/stellar-core/blob/master/src/transactions/SetOptionsOpFrame.cpp), [2026-10-13 evidence](../docs/evidence/testnet-2026-10-13/README.md) | 2026-10-31 | VERIFIED for protocol 29 |
| Operations in one transaction are authorized against pre-transaction state | Protocol source, Testnet cases and fixture tests | [CAP-0002](https://github.com/stellar/stellar-protocol/blob/master/core/cap-0002.md), [Testnet cases 5a and 5b](../docs/evidence/testnet-2026-10-13/README.md#what-the-cases-show), [FX-05a](../fixtures/fx-05a-two-changes-in-one-transaction/expected.json), [FX-05b](../fixtures/fx-05b-two-changes-in-two-transactions/expected.json), [fixture test](../crates/signerreach-cli/tests/fixtures.rs) | 2026-10-31 | VERIFIED for observed protocol 29 cases |
| Original follow-up lockout attempts were rejected | Historical Testnet observation | [2026-10-13 cases 1b and 3b](../docs/evidence/testnet-2026-10-13/README.md#results), [raw results](../docs/evidence/testnet-2026-10-13/results.json) | 2026-10-31 | VERIFIED historical observation |
| Seven chain fixture plans matched a second Testnet replay | Historical Testnet replay | [2026-10-19 replay](../docs/evidence/replay-2026-10-19/README.md), [raw results](../docs/evidence/replay-2026-10-19/results.json) | 2026-10-31 | VERIFIED historical observation |
| Second replay did not repeat follow-up lockout attempts | Replay scope and issue | [Replay notes](../docs/evidence/replay-2026-10-19/README.md#notes), [issue #3](https://github.com/signerreach/signerreach/issues/3) | 2026-10-31 | KNOWN LIMITATION |
| `check` evaluates files offline, returns documented verdict exit codes and stable reports | Integration and unit tests; local workspace run | [CLI tests](../crates/signerreach-cli/tests/check.rs), [offline tests](../crates/signerreach-cli/tests/offline.rs), [report tests](../crates/signerreach-core/src/report.rs), `cargo test --workspace --locked` passed 277 tests | 2026-10-31 | TESTED LOCALLY |
| `fetch` makes constrained read-only Horizon requests and rejects contradictory responses | Fake-server tests and source review | [fetch tests](../crates/signerreach-stellar/tests/fetch.rs), [client](../crates/signerreach-stellar/src/client.rs), [mapping](../crates/signerreach-stellar/src/horizon.rs) | 2026-10-31 | TESTED LOCALLY |
| The Rust workspace builds on Rust 1.87.0 | Local and main CI minimum-toolchain tests | `cargo +1.87.0 test --workspace --locked`; [merged-main CI](https://github.com/signerreach/signerreach/actions/runs/41182773470) | 2026-10-31 | VERIFIED on main at `be8b082` |
| Base main CI passed 269 tests on Ubuntu plus cross-platform jobs | GitHub Actions run | [CI run 39461300547](https://github.com/signerreach/signerreach/actions/runs/39461300547) | 2026-10-31 | VERIFIED for commit `698bafe` only |
| Merged readiness work passed all six hosted CI jobs on main, including 277 Rust tests and an Ubuntu release build | GitHub merge and main-branch CI | [PR #6](https://github.com/signerreach/signerreach/pull/6), [main CI](https://github.com/signerreach/signerreach/actions/runs/41182773470), merge commit `be8b082e9c6e5dad4be2ab4f9ddd5d353e39e702` | 2026-10-31 | VERIFIED on merged main |
| Replay dry run submits no transaction and made no network system calls in this review | Source inspection and local trace | [replay script](../tools/replay/replay.mjs), `node replay.mjs --dry-run`, `strace -f -e trace=network ...`; seven fixtures reported `not_submitted` and trace showed no network calls | 2026-10-31 | TESTED LOCALLY |
| Protocol 30 is scheduled; SignerReach has not verified its rules | Official schedule and code guard | [Bell upgrade guide](https://stellar.org/blog/developers/bell-protocol-30-upgrade-guide), [protocol gate](../crates/signerreach-core/src/input/snapshot.rs), [replay guard](../tools/replay/replay.mjs) | 2026-10-31 | KNOWN LIMITATION |
| Protocol 30 pre-upgrade source review traced Core v29/v30 classic authorization call paths but did not establish full compatibility; Horizon v30 live behavior is unobserved | Tagged Stellar Core/Horizon source review and offline fake-server regression | [Protocol 30 pre-upgrade review](../research/protocol-30-preupgrade-review.md), [fetch tests](../crates/signerreach-stellar/tests/fetch.rs) | 2026-10-31 | PRE-UPGRADE / NOT VERIFIED FOR PROTOCOL 30 |
| Current dependency scanners found no known advisory after narrow updates | RustSec and npm scanners | `cargo audit` on `Cargo.lock`; `npm audit --omit=dev` on [replay lockfile](../tools/replay/package-lock.json); findings and fixes in [security review](../docs/security-review.md) | 2026-10-31 | TESTED LOCALLY; scanner scope only |
| Private vulnerability reporting is enabled | GitHub repository API | `gh api repos/signerreach/signerreach/private-vulnerability-reporting` returned `{"enabled":true}` | 2026-10-31 | VERIFIED |
| No release or tag exists now | GitHub releases and tags API | `gh release list --repo signerreach/signerreach`; `gh api repos/signerreach/signerreach/tags` returned empty lists | 2026-10-31 | VERIFIED at check time |
| Current project eligibility for a Stellar funding track | Official program information | [SCF Build Award](https://communityfund.stellar.org/awards), [research record](../research/ecosystem-recon.md#program-status-and-open-questions) | 2026-10-31 | UNVERIFIED for this project |

## Public claim ledger

| Claim and documentation location | Source of truth and supporting observation | Strength | Last validated |
|---|---|---|---|
| [README](../README.md#what-it-does) says `check` is an offline decision aid with three verdicts | [CLI implementation](../crates/signerreach-cli/src/check.rs), [CLI tests](../crates/signerreach-cli/tests/check.rs), [core evaluator](../crates/signerreach-core/src/evaluate.rs) | TESTED LOCALLY | 2026-10-31 |
| [README](../README.md#making-a-snapshot) says `fetch` only reads and validates network data | [client](../crates/signerreach-stellar/src/client.rs), [URL validation](../crates/signerreach-stellar/src/url.rs), [fake-server tests](../crates/signerreach-stellar/tests/fetch.rs) | TESTED LOCALLY | 2026-10-31 |
| [SPEC](../docs/SPEC.md#3-scope) limits support to protocol 29, classic G accounts and ed25519 signers | [snapshot parser](../crates/signerreach-core/src/input/snapshot.rs), [unsupported fixtures](../fixtures/README.md), historical Testnet records | TESTED LOCALLY for rejection; VERIFIED historically for protocol 29 | 2026-10-31 |
| [SPEC](../docs/SPEC.md#9-command-line) lists text and JSON as implemented, Markdown and schema as planned | [CLI arguments](../crates/signerreach-cli/src/main.rs), [open issues #1](https://github.com/signerreach/signerreach/issues/1) and [#2](https://github.com/signerreach/signerreach/issues/2) | VERIFIED source and issue state | 2026-10-31 |
| [SECURITY.md](../SECURITY.md) and [CONTRIBUTING.md](../CONTRIBUTING.md) distinguish the shipped no-key CLI from the Testnet replay harness | [Rust CLI](../crates/signerreach-cli/src/main.rs), [replay script](../tools/replay/replay.mjs), source review and offline trace | LOGICALLY COVERED; no independent audit | 2026-10-31 |
| [Architecture](../docs/architecture.md) describes pre-state authorization, scenario evaluation and deterministic output | [authorize](../crates/signerreach-core/src/authorize.rs), [apply](../crates/signerreach-core/src/apply.rs), [evaluate](../crates/signerreach-core/src/evaluate.rs), report tests | TESTED LOCALLY | 2026-10-31 |
| [Walkthrough](../docs/walkthrough/README.md) shows unsafe and corrected plans | [FX-01 inputs](../fixtures/fx-01-raise-high-with-lost-signer/expected.json), [corrected plan](../docs/walkthrough/corrected-plan.json), real CLI runs in this review; corrected plan itself was not submitted on-chain | TESTED LOCALLY; corrected live behavior UNVERIFIED | 2026-10-31 |
| [Testnet summaries](../docs/evidence/testnet-2026-10-13/README.md) and [replay summary](../docs/evidence/replay-2026-10-19/README.md) describe observed transactions and their limits | Preserved raw `results.json` files and case scripts in those directories | VERIFIED historical observation; current live compatibility UNVERIFIED | 2026-10-31 |
| [Problem record](../research/problem-validation.md) identifies intended operators and risks | Protocol mechanism and historic cases; no interviews or adoption measurements | LOGICALLY COVERED mechanism; adoption UNVERIFIED | 2026-10-31 |
| [Submission pack](../submission/submission-pack.md) uses only these sources | This index, source files, local and merged-main verification; [readiness audit](../docs/readiness-audit.md) | VERIFIED as a draft; eligibility and final protection review remain UNVERIFIED | 2026-10-31 |

## Evidence that must not be overextended

The 2026-10-13 Testnet record includes follow-up lockout attempts. The
2026-10-19 replay covers fixture plans and a LOW-threshold probe, and did not
repeat those follow-ups. Both ran at protocol 29. The corrected walkthrough
plan has a local checker verdict and supporting mechanism evidence, but no
live run of that exact plan. No historical transaction hash demonstrates
Protocol 30 compatibility or a current live account state. No dependency
scanner result constitutes an independent security audit.
