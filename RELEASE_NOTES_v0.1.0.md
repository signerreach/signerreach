# SignerReach v0.1.0

**Historical release-preparation notes.** This file preserves the draft and
candidate-artifact record written before publication. The published v0.1.0
release and its authoritative final notes, checksums and manifests are at
the [GitHub Release](https://github.com/signerreach/signerreach/releases/tag/v0.1.0).
The candidate checksums below belong to source commit
`9d7985f6dfec05f92db472de6304212f16ffcc6c`; they are not the final release
checksums. The publication checklist below is historical and has been
completed. Do not use it to create or update the published tag.

## What is included

- `signerreach check` evaluates an account snapshot, ordered transaction plan
  and declared signer availability, returning deterministic text or JSON with
  `SAFE WITHIN SCOPE`, `UNSAFE` or `INCONCLUSIVE`.
- `signerreach fetch` performs a constrained, read-only Horizon account read
  and emits a provenance-bearing snapshot.
- The workspace includes bounded JSON parsing, stable diagnostics and reports,
  offline checking, fixture coverage, a security review and a reproducible
  documentation book.
- The separate `tools/replay` program is a development-only Testnet harness;
  it is not part of the shipped CLI and is not a release dependency.

## Supported rules and protocol boundary

This candidate supports the verified Stellar **Protocol 29** rules for one
classic G-address account, ed25519 signers and the implemented `SetOptions`
subset. It models transaction pre-state authorization, ordered transactions,
availability scenarios and the LOW envelope plus MEDIUM/HIGH operation
thresholds relevant to that subset.

Protocol 30 is **not supported or advertised**. Later or unsupported protocol
versions, contract or muxed accounts, fee-bump transactions, unsupported signer
types, other operation types, additional source accounts, sponsorship, account
flags and reserve effects remain outside the safe-verdict boundary and are
reported or documented as `INCONCLUSIVE` where applicable.

## Security boundaries

The shipped CLI never accepts, stores or uses private signing keys, never signs
transactions and never submits them. `check` is offline. `fetch` uses
constrained read-only Horizon requests. The replay harness creates temporary
Testnet keys only for explicitly run development experiments and must not be
used as a production transaction path.

This project has not received an independent security audit. A result is an
aid to review, not a guarantee of key custody, current state, reserve
sufficiency or successful future submission.

## Reproducible evidence

The repository preserves historical Protocol-29 records:

- [2026-10-13 Testnet observations](docs/evidence/testnet-2026-10-13/README.md)
- [2026-10-19 seven-fixture replay](docs/evidence/replay-2026-10-19/README.md)
- [2026-11-01 FX-01 follow-up replay](docs/evidence/replay-2026-11-01-fx-01-followup/README.md)

The FX-01 follow-up repeated the master-only rejection after raising HIGH to 2.
It did not verify Protocol 30, every historical follow-up case, current
account state or the exact corrected walkthrough plan on live Testnet. No live
Testnet submission is part of this release-preparation task.

## Candidate artifacts

Native Linux, macOS and Windows packages are produced by the preparation-only
[GitHub Actions run 41431206670](https://github.com/signerreach/signerreach/actions/runs/41431206670).
That run built the following review-only artifacts from source commit
`9d7985f6dfec05f92db472de6304212f16ffcc6c`:

| Platform | Target triple | Toolchain | Archive | SHA-256 |
|---|---|---|---|---|
| Linux | `x86_64-unknown-linux-gnu` | `rustc 1.99.1 (58cf8f9c3 2026-10-13)` / `cargo 1.99.1 (5f94df478 2026-08-27)` | `signerreach-v0.1.0-x86_64-unknown-linux-gnu.tar.gz` | `188a14b5f3ff65f2e770d56104a96c4b3bb640f27cc6d9a3549e1370e54c15d0` |
| macOS | `aarch64-apple-darwin` | `rustc 1.99.1 (58cf8f9c3 2026-10-13)` / `cargo 1.99.1 (5f94df478 2026-08-27)` | `signerreach-v0.1.0-aarch64-apple-darwin.tar.gz` | `bfa9e9f48a1f1ab489fad78479e43105cd291092e7797f05aee249bc30f93b43` |
| Windows | `x86_64-pc-windows-msvc` | `rustc 1.99.1 (58cf8f9c3 2026-10-13)` / `cargo 1.99.1 (5f94df478 2026-08-27)` | `signerreach-v0.1.0-x86_64-pc-windows-msvc.zip` | `d7e043d8eee93e8ace9aa8278897f54fa5303624bdd0c2c7129c6b308bd09d56` |

The manifests and downloaded archives were checked against these values, and
each package contains only the intended CLI binary, `LICENSE` and `README.md`.
These are CI artifacts for review, not GitHub Release assets. They must be
rebuilt or re-verified from the final approved main commit before publication.

## Publication checklist — maintainer action required

Do not run these steps until the release-preparation PR is reviewed and the
final main commit is approved:

1. Confirm the final reviewed `main` commit, clean working tree, versioned
   workspace and all required CI, documentation, security-audit and replay
   checks.
2. Confirm the artifact manifest target triples, toolchains and SHA-256 values
   match artifacts rebuilt from that exact final commit.
3. Create an annotated tag locally; do not create a lightweight tag:

   ```sh
   git fetch origin main
   git tag --annotate v0.1.0 <FINAL_MAIN_SHA> --message "SignerReach v0.1.0"
   git show --no-patch --decorate v0.1.0
   ```

4. After explicit maintainer approval, push the tag with `git push origin
   v0.1.0`.
5. Create the GitHub Release from the annotated tag using these notes and only
   the verified packages and checksum files. Do not claim Protocol 30 support,
   an independent audit or live behavior beyond the evidence above.

