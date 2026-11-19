//! Turning a Horizon account record into a snapshot.
//!
//! The record comes from a network server, so it is outside data. The promise
//! checked here is the one the `fetch` command makes: whatever Horizon sends,
//! the mapping either refuses it or writes a snapshot that `check` accepts.
//!
//! The first 10 bytes choose the context: the network, the protocol version
//! (29 or 30, the two the checker models) and the ledger. The rest is the record.

#![no_main]

use libfuzzer_sys::fuzz_target;
use signerreach_core::{Network, parse_snapshot};
use signerreach_stellar::{Context, snapshot_from_account};

fuzz_target!(|data: &[u8]| {
    let Some((head, record)) = data.split_at_checked(10) else {
        return;
    };
    let Ok(record) = std::str::from_utf8(record) else {
        return;
    };

    let network = if head[0] & 1 == 0 {
        Network::Testnet
    } else {
        Network::Pubnet
    };
    let protocol_version = if head[1] & 1 == 0 { 29 } else { 30 };
    let ledger = u64::from(u32::from_le_bytes([head[2], head[3], head[4], head[5]])) + 1;
    let context = Context {
        network,
        as_of_ledger: ledger,
        captured_at: "2026-11-19T00:00:00Z",
        protocol_version,
        requested_account: None,
    };

    if let Ok(snapshot) = snapshot_from_account(record, &context) {
        if let Err(issues) = parse_snapshot(&snapshot) {
            let issues: Vec<String> = issues.iter().map(ToString::to_string).collect();
            panic!(
                "the mapping wrote a snapshot that the checker refuses: {}",
                issues.join("; ")
            );
        }
    }
});
