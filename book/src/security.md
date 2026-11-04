# Security and limitations

SignerReach helps review a proposed transaction; it is **not** an account custody service or a transaction execution guarantee.

## Deliberately limited scope

- The verified rule sets are **Stellar Protocol 29** and **Protocol 30**. Other protocol versions return **INCONCLUSIVE**. Protocol 30 is accepted from v0.1.1; v0.1.0 accepts Protocol 29 only. See [issue #7](https://github.com/signerreach/signerreach/issues/7).
- The model covers one classic G-address account, ed25519 signers and the implemented `SetOptions` subset.
- Signer availability is declared by the operator. No command can prove that a private key still exists or is accessible.
- A snapshot describes a point in ledger history, not a guaranteed current account state.
- Historical Testnet observations are not proof of current network compatibility.
- Reserve sufficiency, account flags, fee-bump transactions, other operation sources, supported preconditions outside the model, sponsored signers, contract addresses and other unsupported inputs are not silently assumed safe.
- The tool has **not** been independently audited or established as suitable for production account-security decisions.

## Keys and network boundaries

`signerreach check` is offline. `signerreach fetch` uses constrained read-only Horizon requests. Neither command accepts or handles a private signing key. The separate replay development tool generates temporary keys in memory for Testnet exercises and must not print, save or disclose the secrets.

Read the [security policy](https://github.com/signerreach/signerreach/blob/main/SECURITY.md) for private vulnerability reporting, and the [security review](https://github.com/signerreach/signerreach/blob/main/docs/security-review.md) for known trust assumptions and unresolved boundaries.
