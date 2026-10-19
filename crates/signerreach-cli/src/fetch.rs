//! The `fetch` command: read an account from a Horizon server and print a
//! snapshot document.
//!
//! It only reads. The snapshot goes to standard output and nothing is written to
//! disk, so the result is saved or piped by the caller. The rules for the
//! connection are in `signerreach_stellar::fetch_snapshot`.

use std::process::ExitCode;

use clap::{Args as ClapArgs, ValueEnum};
use signerreach_core::Network;
use signerreach_stellar::{FetchError, HorizonUrl, Limits, fetch_snapshot};

use crate::check::{USAGE_ERROR, emit};

/// Exit code when the account could not be read: the server was unreachable,
/// refused, or sent something that is not a usable account record.
pub const UNAVAILABLE: u8 = 69;

/// Arguments of `signerreach fetch`.
#[derive(Debug, ClapArgs)]
pub struct Args {
    /// The network the account is on
    #[arg(long, value_enum)]
    network: NetworkName,

    /// The account address (G...)
    #[arg(long, value_name = "ADDRESS")]
    account: String,

    /// Read from this Horizon server instead of the public one for the network
    /// (https, or http for this machine only)
    #[arg(long, value_name = "URL")]
    horizon_url: Option<String>,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum NetworkName {
    /// The test network
    Testnet,
    /// The public network
    Pubnet,
}

impl NetworkName {
    const fn network(self) -> Network {
        match self {
            NetworkName::Testnet => Network::Testnet,
            NetworkName::Pubnet => Network::Pubnet,
        }
    }
}

/// Runs the command and returns the process exit code.
pub fn run(args: &Args) -> ExitCode {
    let network = args.network.network();
    let server = match &args.horizon_url {
        Some(text) => match HorizonUrl::parse(text) {
            Ok(server) => server,
            Err(error) => return usage(&error),
        },
        None => HorizonUrl::for_network(network),
    };

    match fetch_snapshot(network, &server, &args.account, &Limits::default()) {
        Ok(snapshot) => emit(&snapshot, 0),
        Err(FetchError::Url(error)) => usage(&error),
        Err(error) => {
            eprintln!("signerreach: could not read the account: {error}");
            ExitCode::from(UNAVAILABLE)
        }
    }
}

fn usage(error: &dyn std::fmt::Display) -> ExitCode {
    eprintln!("signerreach: {error}");
    ExitCode::from(USAGE_ERROR)
}
