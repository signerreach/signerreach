//! Reading an account from a Horizon server.
//!
//! This is the only code in the project that opens a network connection. It is
//! written to be hard to misuse and hard to mislead:
//!
//! - **Only `GET`.** Nothing is ever sent to the server except a request for two
//!   addresses (see [`HorizonUrl`]): the server root, and one account.
//! - **Encrypted, unless it is this machine.** The address rules are in
//!   [`HorizonUrl`]; the client also refuses plain http itself.
//! - **Redirects are never followed.** A redirect answer is an error, so the
//!   request cannot be steered to another host.
//! - **No proxy, no cookies, no credentials, no compression.** The library would
//!   read proxy settings from the environment by default; that is switched off,
//!   so the request goes only where the address says.
//! - **Bounded.** There is a limit on time for each request and on the size of
//!   each answer, and the body of an unsuccessful answer is never read.
//! - **Consistent.** The server root is read before and after the account. If
//!   the network name or protocol version differs between the two reads, the
//!   result is refused, because the snapshot would claim a protocol it was not
//!   read under.
//! - **Nothing the server sends is repeated in an error message.**

use std::time::Duration;

use signerreach_core::Network;
use ureq::Agent;

use crate::error::MapError;
use crate::horizon::{Context, snapshot_from_account};
use crate::http_date::http_date_to_rfc3339;
use crate::root::{RootInfo, parse_latest_ledger};
use crate::url::{HorizonUrl, UrlError};

/// How long to wait, and how much to accept.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// The longest to wait for a connection to be made.
    pub connect_timeout: Duration,
    /// The longest one request may take from start to finish.
    pub total_timeout: Duration,
    /// The largest answer to read, in bytes.
    pub max_body_bytes: u64,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            connect_timeout: Duration::from_secs(10),
            total_timeout: Duration::from_secs(30),
            max_body_bytes: 1024 * 1024,
        }
    }
}

/// Why an account could not be read.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum FetchError {
    /// The server address or the account was refused before any request.
    #[error("{0}")]
    Url(#[from] UrlError),

    /// The server's data could not be turned into a snapshot.
    #[error("{0}")]
    Map(#[from] MapError),

    /// A request took longer than allowed.
    #[error("the server did not answer in time")]
    Timeout,

    /// The server could not be found or reached.
    #[error("the server could not be found or reached")]
    Connect,

    /// The encrypted connection could not be set up.
    #[error("the encrypted connection could not be set up")]
    Tls,

    /// The server answered with a redirect, which is never followed.
    #[error("the server answered with a redirect, which is not followed")]
    Redirect,

    /// The account does not exist on this network.
    #[error("the account was not found on this network")]
    NotFound,

    /// The server answered with a status other than success.
    #[error("the server answered with status {0}")]
    Status(u16),

    /// An answer was larger than the limit.
    #[error("the server's answer is larger than the limit")]
    TooLarge,

    /// A header the snapshot needs is missing.
    #[error("the response has no `{0}` header")]
    MissingHeader(&'static str),

    /// The network name or protocol version changed while the data was read.
    #[error("the network changed while the data was being read; try again")]
    ChangedDuringFetch,

    /// The request failed for another reason.
    #[error("the request failed")]
    Other,
}

/// A successful answer: the headers the snapshot needs, and the body.
struct Reply {
    latest_ledger: Option<String>,
    date: Option<String>,
    body: String,
}

/// Reads an account from a Horizon server and returns the text of a snapshot.
///
/// The account is checked first, so an invalid one makes no request at all.
pub fn fetch_snapshot(
    network: Network,
    server: &HorizonUrl,
    account: &str,
    limits: &Limits,
) -> Result<String, FetchError> {
    let account_url = server.account_url(account)?;
    let root_url = server.root_url();
    let agent = agent(server, limits);

    let before = read_root(&agent, &root_url, limits)?;
    before.check_network(network)?;

    let reply = get(&agent, &account_url, limits).map_err(|error| match error {
        FetchError::Status(404) => FetchError::NotFound,
        other => other,
    })?;
    let as_of_ledger = parse_latest_ledger(
        reply
            .latest_ledger
            .as_deref()
            .ok_or(FetchError::MissingHeader("Latest-Ledger"))?,
    )?;
    let captured_at = http_date_to_rfc3339(
        reply
            .date
            .as_deref()
            .ok_or(FetchError::MissingHeader("Date"))?,
    )?;

    let after = read_root(&agent, &root_url, limits)?;
    if after != before {
        return Err(FetchError::ChangedDuringFetch);
    }

    let context = Context {
        network,
        as_of_ledger,
        captured_at: &captured_at,
        protocol_version: before.protocol_version,
    };
    Ok(snapshot_from_account(&reply.body, &context)?)
}

fn read_root(agent: &Agent, url: &str, limits: &Limits) -> Result<RootInfo, FetchError> {
    let reply = get(agent, url, limits)?;
    Ok(RootInfo::parse(&reply.body)?)
}

/// An agent with every default that could widen where a request goes switched off.
fn agent(server: &HorizonUrl, limits: &Limits) -> Agent {
    let config = Agent::config_builder()
        .timeout_connect(Some(limits.connect_timeout))
        .timeout_global(Some(limits.total_timeout))
        .max_redirects(0)
        .max_redirects_will_error(false)
        .http_status_as_error(false)
        .https_only(server.is_secure())
        .proxy(None)
        .user_agent(format!(
            "signerreach/{} (+https://github.com/signerreach/signerreach)",
            env!("CARGO_PKG_VERSION")
        ))
        .build();
    Agent::new_with_config(config)
}

/// One `GET`. Only a `200` answer is read; any other status is an error and its
/// body is left unread.
fn get(agent: &Agent, url: &str, limits: &Limits) -> Result<Reply, FetchError> {
    let mut response = agent.get(url).call().map_err(classify)?;

    let status = response.status().as_u16();
    if (300..400).contains(&status) {
        return Err(FetchError::Redirect);
    }
    if status != 200 {
        return Err(FetchError::Status(status));
    }

    let latest_ledger = header_text(&response, "latest-ledger");
    let date = header_text(&response, "date");
    let body = response
        .body_mut()
        .with_config()
        .limit(limits.max_body_bytes)
        .read_to_string()
        .map_err(classify)?;
    Ok(Reply {
        latest_ledger,
        date,
        body,
    })
}

/// A header's text, `None` if absent. A value that is not text becomes an empty
/// string, which the strict parsers then refuse.
fn header_text(response: &ureq::http::Response<ureq::Body>, name: &str) -> Option<String> {
    response
        .headers()
        .get(name)
        .map(|value| value.to_str().unwrap_or_default().to_string())
}

fn classify(error: ureq::Error) -> FetchError {
    use ureq::Error;
    match error {
        Error::Timeout(_) | Error::BodyStalled => FetchError::Timeout,
        Error::Io(io) if io.kind() == std::io::ErrorKind::TimedOut => FetchError::Timeout,
        Error::HostNotFound | Error::ConnectionFailed | Error::Io(_) => FetchError::Connect,
        Error::Tls(_) | Error::Rustls(_) | Error::TlsRequired | Error::RequireHttpsOnly(_) => {
            FetchError::Tls
        }
        Error::BodyExceedsLimit(_) => FetchError::TooLarge,
        Error::TooManyRedirects | Error::RedirectFailed => FetchError::Redirect,
        _ => FetchError::Other,
    }
}
