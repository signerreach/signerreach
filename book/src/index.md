# SignerReach

<div class="sr-hero">
  <p class="sr-eyebrow">STELLAR CLASSIC ACCOUNT SAFETY</p>
  <p class="sr-headline">Know whether your keys can still reach the threshold.</p>
  <p class="sr-lede">SignerReach reviews proposed signer and threshold changes <strong>before</strong> you submit them. It models ordered transactions and the keys you can actually use, and shows why a change may leave an account unmanageable.</p>
  <p class="sr-pill">Offline checks</p>
  <p class="sr-pill">No key custody</p>
  <p class="sr-pill">Protocol 29 verified scope</p>
</div>

## Start here

- **[Get started](getting-started.md):** build the Rust CLI and run the FX-01 example.
- **[Understand the verdict](authorization.md):** learn what `SAFE WITHIN SCOPE`, `UNSAFE`, and `INCONCLUSIVE` mean.
- **[Review real evidence](evidence.md):** see protocol 29 Testnet observations and the follow-up authorization rejection.
- **[Know the limitations](security.md):** understand snapshots, unavailable keys, unsupported transactions, and protocol boundaries.

## The problem in one example

A classic account has two signers, each with weight 1, and HIGH threshold 1. A planned change raises HIGH to 2. That change may succeed, yet if one key is lost afterward, the remaining signer cannot make another administrative change by itself.

SignerReach evaluates the plan **in transaction order**, with the declared signer availability, and reports the resulting reachability. It does not sign or submit account changes.

> **Important:** SignerReach is in v0.1.0 release-candidate preparation; no tag or GitHub Release has been published. Its verified authorization model is limited to Stellar **Protocol 29**, one classic G-address account, ed25519 signers, and a subset of `SetOptions`. It has not undergone an independent security audit. A result is evidence for review, not a guarantee.

[Read the specification on GitHub](https://github.com/signerreach/signerreach/blob/main/docs/SPEC.md) · [Browse the source](https://github.com/signerreach/signerreach) · [See the evidence ledger](https://github.com/signerreach/signerreach/blob/main/evidence/index.md)
