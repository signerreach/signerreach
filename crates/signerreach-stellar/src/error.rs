//! Errors from reading data sent by a Stellar server.
//!
//! Messages never repeat a value from the response: a server, or something
//! between you and it, could send anything, and an error message is often logged.

/// Why a server's data could not be turned into a SignerReach document.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum MapError {
    /// The response is not valid JSON.
    #[error("the response is not valid JSON")]
    NotJson,

    /// A field the tool needs is missing.
    #[error("the response has no `{0}`")]
    MissingField(&'static str),

    /// A field is present but has an unexpected type or an out-of-range value.
    #[error("`{0}` has an unexpected type or value")]
    BadField(&'static str),

    /// The account id is not an ed25519 account address (`G...`).
    #[error("the account id is not an ed25519 account address (a G... address)")]
    NotAnAccount,

    /// The signers list has no entry for the account's own key.
    #[error("the signers list has no entry for the account's own key")]
    MissingMasterEntry,

    /// The signers list names the account's own key more than once.
    #[error("the signers list names the account's own key more than once")]
    DuplicateMasterEntry,

    /// A signer has a type this tool does not know.
    #[error("a signer has a type this tool does not know")]
    UnknownSignerType,

    /// The server reports a different network from the one that was asked for.
    #[error("the server is on a different network from the one requested")]
    WrongNetwork,

    /// The ledger number sent by the server is not a whole number of at least 1.
    #[error("the ledger number sent by the server is not valid")]
    BadLedger,

    /// The date is not an HTTP date such as `Sat, 17 Oct 2026 08:17:27 GMT`.
    #[error("the date is not an HTTP date such as `Sat, 17 Oct 2026 08:17:27 GMT`")]
    BadDate,
}
