//! Pure evaluation core for SignerReach.
//!
//! This crate performs no I/O and opens no network connection. It holds the
//! account model, the authorization rules and the verdict logic described in
//! `docs/SPEC.md`.
//!
//! Implemented so far: the account model ([`account`]), the diagnostic codes
//! ([`diagnostic`]) and snapshot and plan parsing ([`input`]). The availability document, the
//! authorization rules and the verdicts are not implemented yet.

pub mod account;
pub mod diagnostic;
pub mod error;
pub mod input;

pub use account::{
    AccountState, MAX_ADDITIONAL_SIGNERS, SignerKey, Threshold, ThresholdClass, Thresholds, Weight,
    sum_weights,
};
pub use diagnostic::Code;
pub use error::DomainError;
pub use input::plan::{Plan, PlannedTransaction, SetOptions, SignerChange, parse_plan};
pub use input::snapshot::{Network, Provenance, Snapshot, SnapshotSource, parse_snapshot};
pub use input::{InputIssue, MAX_INPUT_BYTES};
