//! Pure evaluation core for SignerReach.
//!
//! This crate performs no I/O and opens no network connection. It holds the
//! account model, the authorization rules and the verdict logic described in
//! `docs/SPEC.md`.
//!
//! Implemented so far: the account model ([`account`]). The authorization
//! rules and verdicts are not implemented yet.

pub mod account;
pub mod error;

pub use account::{
    AccountState, MAX_ADDITIONAL_SIGNERS, SignerKey, Threshold, ThresholdClass, Thresholds, Weight,
    sum_weights,
};
pub use error::DomainError;
