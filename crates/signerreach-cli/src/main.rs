use clap::Parser;

/// Offline checker for changes to Stellar classic-account signers and thresholds.
#[derive(Parser)]
#[command(name = "signerreach", version, arg_required_else_help = true)]
struct Cli {}

fn main() {
    let _cli = Cli::parse();
}
