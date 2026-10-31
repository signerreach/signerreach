# Protocol 30 pre-upgrade source review

**Review date:** 2026-10-31  
**Status:** PRE-UPGRADE / NOT VERIFIED FOR PROTOCOL 30  
**Repository baseline:** `fafd23dc6ab004f12c4f3ab79cc67e6e22a05c6f`  
**Scope:** Phase 1 source and offline regression review only. No live Testnet request or transaction was made.

## Decision

Keep the executable and real replay gates at Protocol 29. This source pass found no changed implementation in the classic `SetOptions` threshold selector or shared signature checker, and the envelope and operation authorization call paths retain their threshold checks in the inspected tagged source. However, Protocol 30 changes shared transaction validation and pre-apply sequencing, so this review does not establish full classic authorization equivalence. Horizon v30 response behavior has not been established from a live v30 ledger. **Protocol 30 compatibility is NOT VERIFIED.**

## Exact source identities and reproducibility

Tags resolve to these commits:

| Project | Tag | Commit | Comparison basis |
|---|---|---|---|
| Stellar Core | `v29.0.0` | [`a9d72b0cac3a89ebbcf926af449ac73089e0f8f7`](https://github.com/stellar/stellar-core/tree/a9d72b0cac3a89ebbcf926af449ac73089e0f8f7) | exact release tag |
| Stellar Core | `v30.0.0` | [`0e49751aa9ddf892302cca02cea7f3f5d97df9f1`](https://github.com/stellar/stellar-core/tree/0e49751aa9ddf892302cca02cea7f3f5d97df9f1) | exact release tag |
| Horizon | `v29.0.0` | [`c9e48b645f0777f19135109d3de5c46160cea737`](https://github.com/stellar/stellar-horizon/tree/c9e48b645f0777f19135109d3de5c46160cea737) | exact release tag |
| Horizon | `v30.0.0` | [`b3ff4119cc99ebcc8048334f5ec64e78c16d1666`](https://github.com/stellar/stellar-horizon/tree/b3ff4119cc99ebcc8048334f5ec64e78c16d1666) | exact release tag |

Core file blobs were independently fetched by tag and compared through GitHub's contents API, then checked from the tag trees in a local shallow checkout. Core tag comparison commands:

```sh
git clone --depth 1 --branch v29.0.0 https://github.com/stellar/stellar-core.git /tmp/core
git -C /tmp/core fetch --depth 1 origin v30.0.0
git -C /tmp/core diff --stat v29.0.0 FETCH_HEAD -- src/transactions
git -C /tmp/core diff v29.0.0 FETCH_HEAD -- \
  src/transactions/TransactionFrame.cpp \
  src/transactions/TransactionFrame.h \
  src/transactions/OperationFrame.cpp \
  src/transactions/TransactionUtils.cpp \
  src/transactions/TransactionUtils.h
git -C /tmp/core ls-tree v29.0.0 src/transactions/SetOptionsOpFrame.cpp src/transactions/SetOptionsOpFrame.h src/transactions/SignatureChecker.cpp src/transactions/SignatureChecker.h
git -C /tmp/core ls-tree FETCH_HEAD src/transactions/SetOptionsOpFrame.cpp src/transactions/SetOptionsOpFrame.h src/transactions/SignatureChecker.cpp src/transactions/SignatureChecker.h
```

The identical Git blob IDs verified in both trees are:

| Core file | Blob at both tags | Result |
|---|---|---|
| `src/transactions/SetOptionsOpFrame.cpp` | `251e877a16425db07143be9fd80b34fa103aef7c` | identical |
| `src/transactions/SetOptionsOpFrame.h` | `b83081ca417e1c49d62da7ddf3a49ed1abd1631a` | identical |
| `src/transactions/SignatureChecker.cpp` | `b80f5a50350a72c17182e00236ab870e25c81912` | identical |
| `src/transactions/SignatureChecker.h` | `5c9da958317f8f7b520ae96d3d6ea4a98518a379` | identical |

The following changed; IDs are Git blob IDs at v29 then v30:

| Core file | v29 blob | v30 blob | Review focus |
|---|---|---|---|
| `TransactionFrame.cpp` | `f0eb6968ec3aa979d2a561afbc9364348692bf76` | `9f54ef13f7b8a5818f9bb007528c3c4d017b38ff` | time checks, signature-processing view, Soroban pre-apply and sequence writes |
| `TransactionFrame.h` | `f46716a8dd3b4379ae858f319e4dfafb2e8742f3` | `623114a2c436b6187b3b982f7f111fa749f245a0` | related interfaces and apply-time types |
| `OperationFrame.cpp` | `003d929eb21a3e49f8e7f20e3103c901418b0388` | `3a6f9b02299604eaf190cefacbcd8455c91d8152` | apply plumbing/metrics; inspected signature methods unchanged in diff |
| `TransactionUtils.cpp` | `fd9f07d6ade063927c4ccf9b5ce0b39a5e2a8c2a` | `776ac5697f49d18103ec2a2db77719043d29b077` | apply-time conversion, close-time buffer rounding, sequence time |
| `TransactionUtils.h` | `58173b07285e7de9e9b93258e3a5e08ef3405aef` | `7837669b58d0e52c229361683abfb0da3c30db02` | `ApplyTime` parameter type |

## Classic authorization call-path comparison

| Question | v29/v30 source result | Outcome |
|---|---|---|
| Which threshold authorizes administrative SetOptions? | `SetOptionsOpFrame::getThresholdLevel` is byte-identical. Presence of master weight, any threshold, or signer selects HIGH; otherwise MEDIUM. | **UNCHANGED in source** |
| Which threshold authorizes the transaction envelope? | `TransactionFrame::checkAllTransactionSignatures` is textually unchanged in the reviewed method: it loads the transaction source account and calls `checkSignature` with `THRESHOLD_LOW`; protocol-gated extra signers remain checked afterwards. `checkSignature` builds master plus account signers and delegates to the identical `SignatureChecker`. | **UNCHANGED in inspected method** |
| Which threshold authorizes an operation, including an explicit operation source? | `OperationFrame::checkSignature` and its source-account lookup/threshold call are unchanged in the inspected tagged diff. `checkValid` still checks operation signatures in validation; for application it checks source-account existence as before. | **UNCHANGED in inspected methods** |
| How are signer weights accumulated and unused envelope signatures handled? | `SignatureChecker.cpp/.h` are blob-identical. The checker still owns signature matching, weight selection and “all signatures used” behavior. `TransactionFrame::processSignatures` still invokes operation signature checks and then `checkAllSignaturesUsed`; its ledger view and optional write target changed. | **Core algorithm unchanged; call context refactored** |
| Are signer changes visible to later operations in the same transaction? | Each operation's signature check occurs in `checkOperationSignatures` before application; the chain-backed protocol 29 cases and existing local fixtures show pre-transaction authorization. The v30 shared flow was refactored, and source inspection alone in this bounded pass does not prove every classic transaction ordering/ledger-interaction case. | **Protocol 29 verified; v30 semantic equivalence NOT VERIFIED** |
| Did v30 change source-account selection or account lookup in all transaction paths? | Operation lookup text is unchanged in the inspected method. `TransactionFrame` moved common pre-apply validation to accept a read-only `CheckValidLedgerViewWrapper` and a separate optional write transaction; sequence-number and one-time-signer writes are conditional on a write target. | **Targeted classic interleavings NOT VERIFIED** |
| Did v30 change time representation? | `TransactionFrame` now uses `ApplyTime` and extracts its legacy `timePoint()` for `TimeBounds` checks. `TransactionUtils` rounds expected close-time padding up to seconds and sets account `seqTime` from apply time. | **Changed; transaction timebounds remain whole-second. Review related sequence/precondition interactions before enabling** |

The threshold selection and shared signature algorithm are unchanged, but those facts alone are insufficient to enable Protocol 30. In particular, the pre-apply/view/write refactor and transaction ordering under parallel Soroban application need further semantic review against classic transaction behavior. No authorization rule was changed in SignerReach.

## Horizon contract review

| Horizon field/behavior | SignerReach handling | Release-source / live result |
|---|---|---|
| root `network_passphrase` | Required string, strictly compared to the expected public-network passphrase. | Existing adapter contract. Exact Horizon v30 serializer comparison: **NOT VERIFIED**. |
| root `current_protocol_version` | Required unsigned u32; copied into snapshot provenance. Core parser permits only 29. | Fake-server stable v30 response is fetched then rejected as `SR-RULES-01`; live v30 root: **NOT OBSERVED**. |
| root `history_latest_ledger` | Not used to establish account snapshot ledger. | Existing samples carry it; reliance on it is not asserted. |
| `Latest-Ledger` response header | Required plain positive decimal and used as `as_of_ledger`. | Fake-server behavior covered; no live v30 response observed. |
| HTTP `Date` | Required and parsed to RFC 3339 capture time. | Fake-server behavior covered; no live v30 response observed. |
| requested account ID / `id` / optional `account_id` | Requested account must be G address; response `id` must equal request, and if `account_id` is present it must agree. | Fake-server regressions cover mismatch. |
| `thresholds` and `signers` | Required; duplicate JSON object keys rejected; thresholds/weights bounded to 0..255; exactly one Ed25519 master entry required; unknown signer types refused. | Existing parser and unit/fake-server coverage. Exact v30 serializer behavior: **NOT VERIFIED**. |
| root-account-root consistency | The two parsed roots must compare equal (passphrase and protocol version); otherwise fetch fails. | Fake server verifies network/protocol changes fail. This is not an atomic snapshot and does not prove freshness or honesty. |

Horizon release tags checked: [`stellar/stellar-horizon` v29.0.0](https://github.com/stellar/stellar-horizon/tree/c9e48b645f0777f19135109d3de5c46160cea737) and [v30.0.0](https://github.com/stellar/stellar-horizon/tree/b3ff4119cc99ebcc8048334f5ec64e78c16d1666). A fresh live root/version observation was not obtained in this review. The version string or released Horizon binary alone is not evidence of a v30 ledger.

## Regression and preserved gates

Added an offline fake-Horizon test that serves a stable root with protocol 30, fetches a valid classic account, then proves `parse_snapshot` returns `SR-RULES-01`. Existing snapshot tests already reject unsupported protocol versions, contract and muxed accounts, and unsupported signer kinds. Existing adapter tests refuse unknown Horizon signer types, malformed/duplicate JSON, contradictory IDs, bad ledger claims and root changes. The public gate remains `SUPPORTED_PROTOCOL_VERSION = 29`; real Testnet replay continues to require 29. No protocol-30 authorization logic, W-address support, or live Testnet CI was added.

Relevant reproducible project commands:

```sh
cargo test --workspace --locked
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
node tools/replay/replay.mjs --dry-run
```

For changed source inspection, use the exact tag commands under “Exact source identities”. For the pending Phase 2, wait for actual Testnet activation; record live root passphrase/protocol, Core/Horizon/SDK versions, ledger sequence and UTC timestamp, and use fresh disposable identities. Preserve accepted hashes/ledgers, actual rejection result codes and sanitized raw responses. Do not edit historical 2026-10-13 or 2026-10-19 evidence. Resolve fixture format with issue #3 before adding follow-up attempts.

## Remaining questions / next acceptance gate

- Recheck the Core v30 transaction flow for classic-only envelopes through pre-apply, signature handling, sequence writes, failed-transaction semantics and interactions with operations in the same ledger. Where behavior cannot be established from tagged source/tests, retain **NOT VERIFIED**.
- Compare the actual Horizon v30 root/account serializers and response headers; then observe them against a confirmed active Protocol 30 Testnet ledger.
- Phase 2 is not authorized by this review until Testnet activation is confirmed. Only advance the gate after source analysis, Horizon compatibility, offline regression tests and chain observations agree.

No release, deployment, submission, branch-protection change or protocol gate change is part of this review.
