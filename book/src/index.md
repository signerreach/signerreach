# SignerReach

<div class="sr-hero">
  <p class="sr-eyebrow">STELLAR CLASSIC ACCOUNT SAFETY</p>
  <p class="sr-headline">Know whether your keys can still reach the threshold.</p>
  <p class="sr-lede">SignerReach reviews proposed signer and threshold changes <strong>before</strong> you submit them. It models ordered transactions and the keys you can actually use, and shows why a change may leave an account unmanageable.</p>
  <p class="sr-pill">Offline checks</p>
  <p class="sr-pill">No key custody</p>
  <p class="sr-pill">Protocol 29 and 30 verified scope</p>
</div>

## Start here

- **[Get started](getting-started.md):** download a v0.1.1 binary or build the Rust CLI, then run the FX-01 example.
- **[Understand the verdict](authorization.md):** learn what `SAFE WITHIN SCOPE`, `UNSAFE`, and `INCONCLUSIVE` mean.
- **[Review real evidence](evidence.md):** see protocol 29 and 30 Testnet observations and the follow-up authorization rejection.
- **[Know the limitations](security.md):** understand snapshots, unavailable keys, unsupported transactions, and protocol boundaries.

## The problem in one example

A classic account has two signers, each with weight 1, and HIGH threshold 1. A planned change raises HIGH to 2. That change may succeed, yet if one key is lost afterward, the remaining signer cannot make another administrative change by itself.

SignerReach evaluates the plan **in transaction order**, with the declared signer availability, and reports the resulting reachability. It does not sign or submit account changes.

> **Release status:** SignerReach v0.1.1 was published on November 4, 2026 (v0.1.0 on November 2). See the [published release](https://github.com/signerreach/signerreach/releases/tag/v0.1.1). Its verified authorization model is limited to Stellar **Protocol 29 and 30**, one classic G-address account, ed25519 signers, and a subset of `SetOptions`. It has not undergone an independent security audit. The CLI does not hold keys, sign or submit transactions. A result is evidence for review, not a guarantee; it cannot establish snapshot freshness, signer availability or reserve sufficiency.

[Read the specification on GitHub](https://github.com/signerreach/signerreach/blob/main/docs/SPEC.md) · [Browse the source](https://github.com/signerreach/signerreach) · [See the evidence ledger](https://github.com/signerreach/signerreach/blob/main/evidence/index.md)
