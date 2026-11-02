# Architecture and trust boundaries

SignerReach separates its offline evaluation, network reading, command-line interface and experimental Testnet replay.

| Component | Role | Important boundary |
| --- | --- | --- |
| `signerreach-core` | Parse and evaluate snapshots, plans and availability scenarios | Pure computation, without file, clock or network access |
| `signerreach-stellar` | Convert selected Horizon responses to snapshots | Read-only network adapter |
| `signerreach-cli` | Read files, invoke the checker and render text or JSON | No private signing keys or transaction submission |
| `tools/replay` | Generate disposable Testnet identities and replay chain-backed fixtures | Separate Node development tool; never part of the shipped CLI |

The core checks transactions in order, using account authorization state before each transaction. For unsupported protocol versions or recognized unsupported features, the verdict must remain **INCONCLUSIVE**, never falsely safe.

The replay harness runs its own Testnet and protocol guards before funding or submitting. Live submission is **never run as part of the repository's normal pull-request CI**. Its dry run builds and signs temporary transactions without a network connection.

Read the maintained [architecture document](https://github.com/signerreach/signerreach/blob/main/docs/architecture.md) and [security review](https://github.com/signerreach/signerreach/blob/main/docs/security-review.md) for implementation boundaries and evidence.
