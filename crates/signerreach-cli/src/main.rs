//! The `signerreach` command line.
//!
//! Exit codes (`docs/SPEC.md`, section 9): 0 SAFE WITHIN SCOPE, 1 UNSAFE,
//! 2 INCONCLUSIVE (including rejected input), 64 usage error. `fetch` has no
//! verdict: it exits 0 with a snapshot, 64 for bad arguments, or 69 when the
//! account could not be read.

mod check;
mod fetch;

use std::process::ExitCode;

use clap::{Parser, Subcommand};

/// Offline checker for changes to Stellar classic-account signers and thresholds.
#[derive(Parser)]
#[command(name = "signerreach", version, arg_required_else_help = true)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Check whether a plan keeps an account administrable
    Check(check::Args),
    /// Read an account from a Horizon server and print a snapshot (read-only)
    Fetch(fetch::Args),
}

fn main() -> ExitCode {
    // clap exits with 2 on a usage error, but 2 means INCONCLUSIVE here.
    // Usage errors use 64; help and version output use 0.
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(error) => {
            let code = if error.use_stderr() {
                check::USAGE_ERROR
            } else {
                0
            };
            let _ = error.print();
            return ExitCode::from(code);
        }
    };
    match cli.command {
        Command::Check(args) => check::run(&args),
        Command::Fetch(args) => fetch::run(&args),
    }
}
