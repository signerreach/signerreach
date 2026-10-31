# Problem and validation record

Checked 2026-10-31. This record distinguishes a demonstrated technical failure
mode from unverified adoption claims.

## Whose problem and why it matters

A person or team administering a Stellar classic account with several signers
can accidentally raise HIGH, remove a signer or set the master weight to zero
so that the remaining usable keys cannot change the account again. Operators
using multisig or key rotation are the intended users. Losing administrative
access can make later recovery or policy changes impossible without keys that
are gone. The repository's [Testnet cases 1b and 3b](../docs/evidence/testnet-2026-10-13/README.md)
observed follow-up attempts rejected after such changes. This demonstrates the
mechanism, not how often it occurs in practice.

The current workaround is manual review of signer weights, thresholds,
signature sets and transaction order, possibly with a wallet's warning or a
test transaction. A final-state check can miss a plan that fails halfway
through. SignerReach reads a declared plan and account snapshot, evaluates
each transaction against its pre-state, then checks reachability under declared
availability scenarios. It gives a reproducible explanation before the user
submits a real change.

## What a user supplies

The user must provide an accurate account snapshot, exact transaction
boundaries and operation order, the signatures assumed to be present, and
which keys are available, lost or unknown. A scenario can model a possible key
loss. The checker cannot establish possession or custody of a key and does
not verify signatures. A signed transaction may exist even if its key is later
lost, so availability and `signed_by` serve different purposes.

The decision is reliable only for the supported protocol 29 authorization
rules, one classic account, ed25519 signers and the modeled `SetOptions`
changes. It reports recognized unsupported input as inconclusive. A live
account may change after a snapshot. The snapshot does not prove reserve
sufficiency or other external application conditions, and a signer addition
can fail for low reserve according to the [official Set Options result codes](https://developers.stellar.org/docs/data/apis/horizon/api-reference/errors/result-codes/operation-specific/set-options).
The `SAFE WITHIN SCOPE` verdict is therefore a conditional decision aid.

## Serious remaining risks

The main correctness risk is a false safe result caused by a bad snapshot,
wrong signature assumption, unmodeled external state or a future protocol
change. The most important adoption risk is that users may treat a conditional
verdict as a guarantee or may not know how to prepare the input files. The
[walkthrough](../docs/walkthrough/README.md) and coverage text address that
communication risk, but there is no user interview or usage study in this
repository. No adoption, financial impact or funding estimate is asserted.
