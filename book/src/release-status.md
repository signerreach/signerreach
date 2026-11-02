# Release and submission status

SignerReach is preparing its first versioned candidate, **v0.1.0**. The
workspace is versioned `0.1.0`, but no `v0.1.0` tag or GitHub Release has been
created. This page describes preparation status; it is not a publication notice.

The full draft is in the repository's [v0.1.0 release notes](https://github.com/signerreach/signerreach/blob/main/RELEASE_NOTES_v0.1.0.md).

## Candidate scope

The candidate contains the implemented Rust `check` and `fetch` commands,
deterministic text/JSON reports, bounded input handling, fixtures, source and
security documentation, the mdBook, and a development-only replay harness.

Its verified rules boundary is Stellar **Protocol 29**: one classic G-address
account, ed25519 signers and the implemented `SetOptions` subset. Protocol 30
is not supported or advertised. Unsupported protocol versions and recognized
unsupported inputs remain `INCONCLUSIVE` or outside the model as documented.

## Security and evidence boundary

The shipped CLI does not hold keys, sign, submit transactions or make network
requests from `check`. `fetch` only reads through constrained Horizon requests.
The replay harness is a separate development tool that can create temporary
Testnet keys only when explicitly run. No independent security audit is claimed.

The candidate relies on the preserved [2026-10-13 Testnet record](https://github.com/signerreach/signerreach/blob/main/docs/evidence/testnet-2026-10-13/README.md), [2026-10-19 replay](https://github.com/signerreach/signerreach/blob/main/docs/evidence/replay-2026-10-19/README.md) and [2026-11-01 FX-01 follow-up](https://github.com/signerreach/signerreach/blob/main/docs/evidence/replay-2026-11-01-fx-01-followup/README.md). All are Protocol 29 evidence; they do not establish Protocol 30 compatibility, current account state or key custody.

## Release gates

Before publication, a maintainer must review the final main commit, versioned
lockfile, native CI packages and their manifests, checksums, documentation and
security scans. The packages must be rebuilt or re-verified from that exact
final commit. Only after explicit approval may a maintainer create the
annotated `v0.1.0` tag and GitHub Release described in the [draft checklist](https://github.com/signerreach/signerreach/blob/main/RELEASE_NOTES_v0.1.0.md#publication-checklist--maintainer-action-required).

## Check the authoritative status

- [Repository](https://github.com/signerreach/signerreach)
- [CI](https://github.com/signerreach/signerreach/actions/workflows/ci.yml)
- [Releases](https://github.com/signerreach/signerreach/releases)
- [Evidence index](https://github.com/signerreach/signerreach/blob/main/evidence/index.md)
