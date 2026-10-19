//! Reads Stellar network data into SignerReach input documents.
//!
//! This crate holds the pieces that deal with what Stellar's servers send. The
//! evaluation in `signerreach-core` never sees them. Everything here so far is
//! pure: it turns text into text and makes no network connection. The code that
//! talks to a server is added separately, on top of these functions.
//!
//! - [`snapshot_from_account`] turns a Horizon account record into a
//!   `signerreach.snapshot/1` document.
//! - [`http_date_to_rfc3339`] turns a server's `Date` header into the timestamp a
//!   snapshot records.
//! - [`HorizonUrl`] checks a server address and builds the only two addresses
//!   the tool will ever request.
//! - [`RootInfo`] and [`parse_latest_ledger`] read what a server says about its
//!   network and the ledger a response is as of.

mod error;
mod horizon;
mod http_date;
mod root;
mod url;

pub use error::MapError;
pub use horizon::{Context, snapshot_from_account};
pub use http_date::http_date_to_rfc3339;
pub use root::{RootInfo, parse_latest_ledger, passphrase_of};
pub use url::{HorizonUrl, UrlError};
