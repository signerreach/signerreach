# Release and submission status

SignerReach **v0.1.2 was published on November 27, 2026**. The release and its
nine assets were published from main commit
[`9d78409e5d49ca8cd7c974728ff1f8ecb59a5642`](https://github.com/signerreach/signerreach/commit/9d78409e5d49ca8cd7c974728ff1f8ecb59a5642).
The annotated `v0.1.2` tag points to that commit. It adds the balance, reserve
and fee rules: a plan that adds a signer is checked against the account's
spendable balance, and `fetch` records the data this needs. v0.1.1 (published on
November 4, 2026 from main commit
[`f950a821044c479c29550e6e003612b2de7b9175`](https://github.com/signerreach/signerreach/commit/f950a821044c479c29550e6e003612b2de7b9175))
added Protocol 30 support but does not check the reserve or the fee. The first
release, v0.1.0 (published on November 2, 2026 from main commit
`e1a37cea9101bbf2cb68372f111c6e4a5ecd1ba4`), accepts Protocol 29 only. Both
earlier releases remain available.

Read the [published release notes and assets](https://github.com/signerreach/signerreach/releases/tag/v0.1.2),
the [publication workflow](https://github.com/signerreach/signerreach/actions/runs/46029982136),
and the [release commit's successful main CI](https://github.com/signerreach/signerreach/actions/runs/46028481732).

## Published packages

The release contains one archive, checksum file and manifest for each target:

| Platform | Target triple | Archive |
|---|---|---|
| Linux x86-64 | `x86_64-unknown-linux-gnu` | [tar.gz](https://github.com/signerreach/signerreach/releases/download/v0.1.2/signerreach-v0.1.2-x86_64-unknown-linux-gnu.tar.gz) |
| macOS Apple Silicon | `aarch64-apple-darwin` | [tar.gz](https://github.com/signerreach/signerreach/releases/download/v0.1.2/signerreach-v0.1.2-aarch64-apple-darwin.tar.gz) |
| Windows x86-64 | `x86_64-pc-windows-msvc` | [zip](https://github.com/signerreach/signerreach/releases/download/v0.1.2/signerreach-v0.1.2-x86_64-pc-windows-msvc.zip) |

All nine release assets are uploaded. The publication workflow verified each
package's manifest, checksum and contents before upload, and the archive
checksums in the release notes match the digests GitHub computed for the
uploaded files. The manifests record the exact source commit above. No other
architecture is included.

## Implemented scope

The release contains the implemented Rust `check` and `fetch` commands,
deterministic text and JSON reports, bounded input handling, fixtures, source
and security documentation, the mdBook, and a development-only replay harness.
Since v0.1.2, `check` also tests the fee and, for a plan that adds a signer, the
reserve (`SR-RESERVE-01`), and gives `INCONCLUSIVE` when the snapshot has no
`reserve` object.

Its verified rules boundary is Stellar **Protocol 29 and 30**: one classic
G-address account, ed25519 signers and the implemented `SetOptions` subset.
Protocol 30 was verified by a source review and a Testnet replay; v0.1.0
supported Protocol 29 only. Unsupported protocol versions and recognized
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
These are Protocol 29 evidence. The
[2026-11-04 Protocol 30 replay](https://github.com/signerreach/signerreach/blob/main/docs/evidence/replay-2026-11-04-protocol-30/README.md)
is the Protocol 30 record. The reserve and fee rules are backed by the
[2026-11-20 Testnet reserve failure](https://github.com/signerreach/signerreach/blob/main/docs/evidence/testnet-2026-11-20-reserve-gap/README.md)
that led to them, seven exact-balance Testnet cases
([2026-11-23 reserve boundaries](https://github.com/signerreach/signerreach/blob/main/docs/evidence/testnet-2026-11-23-reserve-boundaries/README.md)),
and state-only checks on three public Mainnet accounts
([2026-11-23](https://github.com/signerreach/signerreach/blob/main/docs/evidence/mainnet-2026-11-23-protocol-30/README.md)).
Selling liabilities, the 1000 sub-entry limit, sponsorship, the order of the
three signer checks and any base reserve other than 5,000,000 stroops come from
the stellar-core source only and have not been recorded on a network. None of them establishes current network compatibility, current account state or key
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
