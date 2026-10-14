//! Errors raised when a value violates a domain invariant.

use crate::account::SignerKey;

/// A value that cannot be represented in the account model.
///
/// Every variant is a reason to refuse an input rather than guess. The
/// evaluation layer maps these to the `SR-INPUT-*` diagnostics.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DomainError {
    /// A weight is above 255.
    #[error("weight {0} is outside the range 0 to 255")]
    WeightOutOfRange(u32),

    /// A threshold is above 255.
    #[error("threshold {0} is outside the range 0 to 255")]
    ThresholdOutOfRange(u32),

    /// The text is not a Stellar ed25519 account key (a `G...` address).
    #[error("not a valid ed25519 account key (expected a G... address): {0}")]
    InvalidAccountKey(#[source] stellar_strkey::DecodeError),

    /// The same signer key is listed twice.
    #[error("signer {0} is listed more than once")]
    DuplicateSigner(SignerKey),

    /// A signer entry names the account's own key.
    #[error("signer {0} is the account's own key; the master key is set with the master weight")]
    SignerIsAccount(SignerKey),

    /// A signer entry has weight 0, which means the signer is removed.
    #[error("signer {0} has weight 0; a signer with weight 0 is not part of an account state")]
    ZeroWeightSigner(SignerKey),

    /// More additional signers than an account can hold.
    #[error("{count} additional signers exceed the limit of {max}")]
    TooManySigners {
        /// Number of additional signers supplied.
        count: usize,
        /// The most an account can hold.
        max: usize,
    },
}
