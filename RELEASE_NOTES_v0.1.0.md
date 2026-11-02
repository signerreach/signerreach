# SignerReach v0.1.0

**Draft release notes for maintainer review — not a published release.** No
`v0.1.0` tag or GitHub Release has been created. The release-preparation PR and
its native CI artifacts must be reviewed before publication.

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
GitHub Actions workflow. The workflow records the exact source commit, native
target triple, toolchain versions, archive name and SHA-256 checksum in each
artifact. These are CI artifacts for review, not GitHub Release assets. They
must be rebuilt or re-verified from the final approved main commit before
publication.

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

