# Repository readiness audit

Original readiness work checked 2026-10-31; post-merge state rechecked the same day.
[PR #6](https://github.com/signerreach/signerreach/pull/6) merged into upstream `main` at `be8b082e9c6e5dad4be2ab4f9ddd5d353e39e702`, and
[main CI](https://github.com/signerreach/signerreach/actions/runs/41182773470) completed successfully. The historical 13-stage matrix
tracks the maintainer's implementation assignment derived from Playbook v2.0.0;
its stage numbers are not the playbook's 36 phase numbers. This post-merge
review also considers applicability of the complete playbook, without claiming
external project approval or an independent security audit. `PASS` means the
stated repository work was checked. `CONDITIONAL` preserves a limitation or
unresolved decision. `BLOCKED` requires an external gate.

## Current state after publication (2026-11-02)

- At the final audit, upstream `main` is `1c4c92ee4e26aba2b08ca50dec905e9097108ef1`; [PR #19](https://github.com/signerreach/signerreach/pull/19) merged the README correction. The published v0.1.0 release remains built from verified source commit `e1a37cea9101bbf2cb68372f111c6e4a5ecd1ba4`, merged by [PR #17](https://github.com/signerreach/signerreach/pull/17).
- SignerReach v0.1.0 was published, not as a draft or prerelease, on November 2, 2026. Annotated tag `v0.1.0` (tag object `2a01d2eedb4800fa7b443f344eb931a7f1eadb5f`) points to the exact source commit above. The [publication workflow](https://github.com/signerreach/signerreach/actions/runs/41451872483) succeeded.
- The release contains nine uploaded and verified assets: a package, SHA-256 file and manifest for Linux `x86_64-unknown-linux-gnu`, macOS `aarch64-apple-darwin`, and Windows `x86_64-pc-windows-msvc`. The workflow verified them; a follow-up download also confirmed each archive checksum and each manifest's source commit.
- The [published release](https://github.com/signerreach/signerreach/releases/tag/v0.1.0) is limited to Protocol 29 for one classic G-address account, ed25519 signers and the implemented `SetOptions` subset. The CLI does not hold keys, sign or submit transactions. No independent audit is claimed. Historical Testnet records do not prove current network compatibility; snapshot freshness, signer availability and reserve sufficiency are not guaranteed.
- The release-commit CI and documentation build passed. Current-main [CI](https://github.com/signerreach/signerreach/actions/runs/41459984154) and the [documentation build](https://github.com/signerreach/signerreach/actions/runs/41459984009) passed at `1c4c92e`; the push-triggered deploy was skipped. The manual [Documentation workflow](https://github.com/signerreach/signerreach/actions/runs/41464392358) then deployed successfully from that same commit. The published book and release-status page return HTTP 200 and show the post-PR #18 publication status. The repository Homepage field points to the book.
- Issues [#1](https://github.com/signerreach/signerreach/issues/1), [#2](https://github.com/signerreach/signerreach/issues/2), and [#7](https://github.com/signerreach/signerreach/issues/7) remain open. No other pull request was open at the final audit check before this audit PR.
- Release publication is complete. The read-only audit below verifies the limited technical claims; it does not establish program eligibility. No program application has been submitted, accepted or approved. Detailed branch-protection verification remains the last operational task. Protocol 30 support and an independent security audit are not claimed.

## Final read-only submission audit (2026-11-02)

### Technical scope and demonstration evidence

- The source defines `SUPPORTED_PROTOCOL_VERSION` as 29. Snapshot parsing rejects other versions with the rules diagnostic, and the Horizon adapter test `a_stable_protocol_30_horizon_root_cannot_produce_a_safe_snapshot` confirms that a Protocol 30 response cannot be used for a safe verdict. The published v0.1.0 rules claim is limited to Protocol 29, one classic G-address account, ed25519 signers and the implemented `SetOptions` subset.
- The unsafe walkthrough uses the FX-01 fixture and records the historical Protocol 29 Testnet authorization result. The corrected plan in `docs/walkthrough/corrected-plan.json` returns `SAFE WITHIN SCOPE` locally; that exact corrected plan was not submitted to Testnet. Its mechanism is supported by separate Protocol 29 cases. Neither result guarantees current state or future network behavior.
- The shipped CLI does not hold keys, sign or submit transactions. `check` is offline; `fetch` makes constrained read-only Horizon requests. The separate replay harness can submit disposable Testnet transactions only when explicitly run. This audit made no Testnet submission.
- The security policy and review state that there is no independent audit. Signer availability is declared, snapshots can become stale, and reserve sufficiency and external conditions are not guaranteed. The evidence index and preserved Testnet records support Protocol 29 claims only.

### Scheduled Protocol 30 Testnet vote

On this audit date, the [Stellar Development Foundation's Bell announcement](https://stellar.org/blog/developers/introducing-bell-protocol-30-on-stellar) listed the Protocol 30 Testnet upgrade vote for November 3, 2026 at 17:00 UTC. Its proposed timeline says dates may change. This schedule does not change SignerReach's verified scope.

If validators approve the upgrade and Testnet moves to Protocol 30, `fetch` can record that protocol version, but the current checker will reject the snapshot as unsupported and return `INCONCLUSIVE`. Before any future live demonstration, verify the network's current protocol. After a move to Protocol 30, do not present the shipped release as a supported live checker; use the preserved Protocol 29 fixtures only as historical offline examples until Protocol 30 is separately implemented and verified. No Protocol 30 support or live demonstration is claimed here.

### Program eligibility and submission verdict

The release and current-main CI establish technical status within the documented Protocol 29 boundary. They do not establish eligibility for any funding program. Eligibility, program-specific requirements, adoption evidence and any application plan remain unverified; no target award, invitation, budget, application, acceptance or approval is claimed. **NOT READY FOR PROGRAM SUBMISSION** until those facts and requirements are verified and the submission is reviewed. Detailed branch-protection verification is reserved as the final operational task after the other readiness work.

The dated pre-publication findings below are retained as historical evidence.
Use this section for current release status; older candidate and authorization
statements describe the state before publication.

## Pre-publication status record (2026-11-02)

The following notes recorded release preparation before the tag was created.
They are preserved as a snapshot and are superseded by the current state above.

- Upstream `main` is now `15105788b60f8d3d05188cbe0d23d6ad51dbafe6` after the documentation PR was merged. The published mdBook was independently verified at [https://signerreach.github.io/signerreach/](https://signerreach.github.io/signerreach/) with HTTP 200 pages/assets, valid internal links, navigation, search, responsive metadata and light/dark theme assets.
- The release-preparation branch changes the workspace to `0.1.0` while retaining `publish = false`; no tag, GitHub Release or crates.io publication exists. [Draft v0.1.0 notes](../RELEASE_NOTES_v0.1.0.md) define the candidate boundary and maintainer-only publication checklist.
- Review-only native candidate artifacts were built successfully in [run 41431206670](https://github.com/signerreach/signerreach/actions/runs/41431206670) from preparation-branch commit `9d7985f6dfec05f92db472de6304212f16ffcc6c`; their exact manifests and checksums are recorded in the draft notes. They are not release assets and must be rebuilt from the final approved main commit.

- `main` was `57324abcfe52f331ad28f4d3ab406e21f425edec` before the documentation merge; the current merged head is `15105788b60f8d3d05188cbe0d23d6ad51dbafe6`.
- The latest verification recorded 278 Rust tests and 51 replay tests.
- [PR #12](https://github.com/signerreach/signerreach/pull/12) delivered replay-only `after.json` support. [PR #13](https://github.com/signerreach/signerreach/pull/13) recorded a fresh Protocol 29 FX-01 follow-up rejection.
- Issue [#3](https://github.com/signerreach/signerreach/issues/3) is closed; issues #1, #2, and #7 remain open.
- The maintained README and documentation book are published; the Pages URL above is verified live. No release/tag, Protocol 30 support, or independent security audit is claimed. Detailed branch-protection verification remains the final operational step.

## Historical gap matrix (reviewed before publication)

| Stage and requirement | Starting status | Action and evidence | Final gate | Remaining condition |
|---|---|---|---|---|
| 0. Re-establish repository truth | Historical snapshot only | Rechecked merged `main` at `be8b082e9c6e5dad4be2ab4f9ddd5d353e39e702`, with no open PRs and [six successful CI jobs](https://github.com/signerreach/signerreach/actions/runs/41182773470). | PASS | Recheck mutable state immediately before submission. |
| 1. Correct stale public material | Spec and core docs described unimplemented CLI; output and key policy wording conflicted | Corrected [SPEC](SPEC.md), core module docs, [README](../README.md), [SECURITY.md](../SECURITY.md) and [CONTRIBUTING.md](../CONTRIBUTING.md); added error regressions. | PASS | Recheck claims when interface changes. |
| 2. Research, validation and architecture | No dedicated records | Added [ecosystem research](../research/ecosystem-recon.md), [problem record](../research/problem-validation.md) and [architecture](architecture.md), with official sources and limits. | CONDITIONAL | Adoption, user demand and program eligibility remain unverified. |
| 3. Core correctness and security | 269 tests in historical CI; security review incomplete | Reproduced local gates, added failing regressions before three corrections, reached 277 passing tests; recorded [security review](security-review.md). | CONDITIONAL | No independent audit or current live Protocol 30 verification. |
| 4. CI coverage and safety | Rust jobs passed historically; no replay dry run or release build in CI | Added Ubuntu release build and offline replay job; [post-merge CI](https://github.com/signerreach/signerreach/actions/runs/41182773470) passed all six jobs on `be8b082`, including 277 Rust tests and the Ubuntu release build. | PASS | Recheck after any further commit. |
| 5. Testnet evidence and protocol transition | Two historical protocol 29 records; second replay omitted follow-up attempts | Preserved both records; later captured a guarded [FX-01 follow-up replay](evidence/replay-2026-11-01-fx-01-followup/README.md) after the approved [issue #3](https://github.com/signerreach/signerreach/issues/3) format, while keeping the protocol 29 gate; opened [issue #7](https://github.com/signerreach/signerreach/issues/7). | CONDITIONAL | Protocol 30 rules and live compatibility have not been verified; other historical follow-ups have not been replayed. |
| 6. Evidence and public claims | No central ledger | Added [evidence and claim ledger](../evidence/index.md) and linked the merged main-branch CI result. | PASS | Update records after protocol verification and any new release. |
| 7. External documentation experience | README and specification existed without a short route through them | Added [documentation index](README.md), architecture and evidence links; checked 177 Markdown links with zero errors. | PASS | Repository documentation is the review surface; a hosted site is not required. |
| 8. Identity, issues and release decision | Correct description, Apache-2.0 license and topics; no release | Verified identity, retained open issues #1 and #2, closed issue #3 through the replay follow-up work, added only protocol issue #7, and recorded a truthful first release boundary in the [submission pack](../submission/submission-pack.md). Review-only candidate artifacts are now recorded, but no tag or release exists. | CONDITIONAL | Release remains a maintainer decision; candidate artifacts must be rebuilt from the final approved main commit. |
| 9. Submission pack | Absent | Added a [draft pack](../submission/submission-pack.md) based on the ledger and real CLI output. | PASS | No application submitted. |
| 10. External reviewer simulation | Not recorded | Walked 12 reviewer questions, corrected stale wording and checked links; readiness changes are now visible on upstream `main`. | CONDITIONAL | Program-specific eligibility and final submission links still need review. |
| 11. Commit, PR and verification discipline | No readiness branch | Hollujay's focused [PR #6](https://github.com/signerreach/signerreach/pull/6) was merged as `be8b082`; upstream CI passed. | PASS | Keep future corrections in focused PRs and do not bypass protection. |
| 12. Final audit and freeze | Previously blocked pending PR #6 | Main contains the readiness changes and the main-head CI passed; this post-merge documentation correction removes old blocked claims. | CONDITIONAL | Merge this documentation-only correction, recheck final main, verify current eligibility, and complete submission freeze. |
| 13. Branch protection, last operational task | Main marked protected; detailed settings inaccessible | Read-only inspection reports main protected, but the integration cannot read required-check/review/bypass details. No policy change was made. | BLOCKED | An authorized maintainer must inspect the actual policy only after every other readiness task is complete. |

Hosting the CLI itself is not applicable. The GitHub Pages documentation book is
published and verified at the URL recorded in the current-state section above.
A contract, frontend, database, deployed backend, and cross-repository parity are
also **NOT APPLICABLE** to its current architecture. The maintainer excluded
Dependabot work from this assessment. At the time of this historical review,
release and submission were future maintainer decisions. Release publication
is now complete; submission readiness remains unassessed.

## Playbook v2.0.0 applicability

The original playbook's research and scope phases are represented by the
research records, specification, and architecture. Its implementation,
environment, testing, security, CI, documentation, and evidence requirements
are represented by source, tests, CI, reviews, and the evidence ledger.
Historical Testnet observations support only protocol 29. Protocol 30 and any
expansion beyond the observed FX-01 follow-up replay remain tracked issues.
At the time of the original review, release, eligibility, submission, and final
maintenance were future work. Release publication is now complete; submission
readiness and the final branch-protection check remain for the separate audit.

## Historical reviewer simulation

The simulated review below was recorded before publication. Release-status
answers reflect that earlier snapshot.

| First-time reviewer question | Result and path |
|---|---|
| 1. What is this? | [README opening](../README.md) states the purpose and unreleased status. |
| 2. Why Stellar? | [Research](../research/ecosystem-recon.md#mechanism-and-official-sources) and [SPEC rules](SPEC.md#4-verified-rules-this-specification-relies-on) explain account authorization. |
| 3. What is implemented? | [README commands](../README.md#what-it-does) and [SPEC opening](SPEC.md) distinguish shipped and planned work. |
| 4. Can it build? | Local Rust 1.87.0 and current toolchain tests passed; [main CI](https://github.com/signerreach/signerreach/actions/runs/41182773470) completed all six jobs at `be8b082`, including 277 tests and an Ubuntu release build. |
| 5. Can the examples run? | [Walkthrough](walkthrough/README.md) and [submission commands](../submission/submission-pack.md#reproducible-demonstration) give real unsafe and corrected output. |
| 6. Where is the design? | [Architecture](architecture.md) shows the CLI, core, adapter and replay tool. |
| 7. What are its limits? | [Architecture limits](architecture.md#inputs-and-trust-boundaries) and [security review](security-review.md#outstanding-limitations) state the assumptions. |
| 8. Is there real Testnet evidence? | [Evidence index](../evidence/index.md) links all three historical records and explains their differing coverage. |
| 9. Can claims be traced? | [Claim ledger](../evidence/index.md#public-claim-ledger) links public wording to source and tests. |
| 10. How are security reports handled? | [SECURITY.md](../SECURITY.md) names private reporting; the repository API returned `enabled: true`. |
| 11. Can someone contribute? | [CONTRIBUTING.md](../CONTRIBUTING.md) gives setup, tests and evidence rules. |
| 12. What remains open? | Issues [#1](https://github.com/signerreach/signerreach/issues/1), [#2](https://github.com/signerreach/signerreach/issues/2) and [#7](https://github.com/signerreach/signerreach/issues/7) remain open; [#3](https://github.com/signerreach/signerreach/issues/3) is closed. |

The local link checker reported 177 checked Markdown links and zero errors
after two broken SPEC anchors were corrected. This checks reachability and
local anchors at the review time; it does not prove that every external page's
content remains unchanged.

## Post-merge verification

- Upstream default branch: `be8b082e9c6e5dad4be2ab4f9ddd5d353e39e702`.
- [PR #6](https://github.com/signerreach/signerreach/pull/6): merged on 2026-10-31.
- [Main-branch CI](https://github.com/signerreach/signerreach/actions/runs/41182773470): six jobs successful, with 277 Rust tests,
  minimum-Rust testing, offline replay dry run, and Ubuntu release build.
- Open issues: #1, #2, and #7; issue #3 is closed. No open pull requests at the review time.
- No independent audit or live Protocol 30 transaction was performed here. A subsequent guarded Protocol-29 [FX-01 follow-up replay](evidence/replay-2026-11-01-fx-01-followup/README.md) is recorded separately.

## Remaining findings

- **A, submission gates:** No new core-code or CI failure was found on merged
  main. Current program eligibility, final submission freeze, and detailed
  branch-protection settings remain unverified. Inspect branch protection
  last, with an authorized maintainer account.
- **B, stale material:** The old PR-pending and final-main-blocked claims
  are corrected in this post-merge documentation change.
- **C, known limitations:** Only protocol 29 is verified; snapshot freshness,
  key availability, reserve sufficiency, and current live compatibility
  are not guaranteed. FX-01 follow-up replay has been repeated once, but
  remaining follow-up coverage is incomplete and there is no independent audit
  or measured adoption.
- **D, important fix:** This documentation-only correction reconciles
  public readiness records with GitHub's actual merged and tested state.
- **E, legitimate backlog:** Issues #1 and #2 are contributor enhancements;
  #3 is now closed after FX-01 follow-up verification; #7
  tracks Protocol 30 verification.

## Historical release and submission decision (before publication)

The implemented `check` and `fetch` commands, text and JSON reports,
protocol 29 fixtures and documentation are a coherent initial-release
candidate. The release-preparation branch versions the workspace `0.1.0` and
retains `publish = false`; no tag or GitHub Release exists. Native artifact
builds, final CI and maintainer approval remain release gates. No program
application has been submitted or its eligibility verified. **CONDITIONALLY
READY** for the remaining maintainer and program checks, not cleared for
publication or submission.
