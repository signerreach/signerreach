# Release and submission status

SignerReach **v0.1.0 was published on November 2, 2026**. The release and its
nine verified assets were published from main commit
[`e1a37cea9101bbf2cb68372f111c6e4a5ecd1ba4`](https://github.com/signerreach/signerreach/commit/e1a37cea9101bbf2cb68372f111c6e4a5ecd1ba4).
The annotated `v0.1.0` tag points to that commit.

Read the [published release notes and verified assets](https://github.com/signerreach/signerreach/releases/tag/v0.1.0),
the [publication workflow](https://github.com/signerreach/signerreach/actions/runs/41451872483),
and the [release commit's successful main CI](https://github.com/signerreach/signerreach/actions/runs/41447399997).

## Published packages

The release contains one archive, checksum file and manifest for each target:

| Platform | Target triple | Archive |
|---|---|---|
| Linux x86-64 | `x86_64-unknown-linux-gnu` | [tar.gz](https://github.com/signerreach/signerreach/releases/download/v0.1.0/signerreach-v0.1.0-x86_64-unknown-linux-gnu.tar.gz) |
| macOS Apple Silicon | `aarch64-apple-darwin` | [tar.gz](https://github.com/signerreach/signerreach/releases/download/v0.1.0/signerreach-v0.1.0-aarch64-apple-darwin.tar.gz) |
| Windows x86-64 | `x86_64-pc-windows-msvc` | [zip](https://github.com/signerreach/signerreach/releases/download/v0.1.0/signerreach-v0.1.0-x86_64-pc-windows-msvc.zip) |

All nine release assets are uploaded. Their archive checksums and manifests
were verified against downloaded release files and record the exact source
commit above. No other architecture is included.

## Implemented scope

The release contains the implemented Rust `check` and `fetch` commands,
deterministic text and JSON reports, bounded input handling, fixtures, source
and security documentation, the mdBook, and a development-only replay harness.

Its verified rules boundary is Stellar **Protocol 29**: one classic G-address
account, ed25519 signers and the implemented `SetOptions` subset. Protocol 30
is not supported or advertised. Unsupported protocol versions and recognized
unsupported inputs remain `INCONCLUSIVE` or outside the model as documented.

## Security and evidence boundary

The shipped CLI does not hold keys, sign or submit transactions. `check` makes
no network request. `fetch` uses constrained, read-only Horizon requests. The
replay harness is a separate development tool that can create temporary
Testnet keys only when explicitly run. No independent security audit is
claimed.

The release notes refer to the preserved [2026-10-13 Testnet record](https://github.com/signerreach/signerreach/blob/main/docs/evidence/testnet-2026-10-13/README.md),
[2026-10-19 replay](https://github.com/signerreach/signerreach/blob/main/docs/evidence/replay-2026-10-19/README.md)
and [2026-11-01 FX-01 follow-up](https://github.com/signerreach/signerreach/blob/main/docs/evidence/replay-2026-11-01-fx-01-followup/README.md).
These are Protocol 29 evidence only. They do not establish Protocol 30
compatibility, current network compatibility, current account state or key
custody. Snapshot freshness, signer availability and reserve sufficiency are
not guaranteed.

## Publication and submission status

Release publication is complete and verified. This does not establish
submission readiness. No program application has been submitted, accepted or
approved. Eligibility, program-specific requirements and the final submission
freeze remain for the separate submission-readiness audit. Detailed branch
protection verification remains the last operational task in that audit.

The repository homepage links to the [published documentation book](https://signerreach.github.io/signerreach/).

## Authoritative records

- [Repository](https://github.com/signerreach/signerreach)
- [CI](https://github.com/signerreach/signerreach/actions/workflows/ci.yml)
- [Releases](https://github.com/signerreach/signerreach/releases)
- [Evidence index](https://github.com/signerreach/signerreach/blob/main/evidence/index.md)
