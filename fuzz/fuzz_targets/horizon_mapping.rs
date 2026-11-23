//! Turning a Horizon account record into a snapshot.
//!
//! The record comes from a network server, so it is outside data. The promise
//! checked here is the one the `fetch` command makes: whatever Horizon sends,
//! the mapping either refuses it or writes a snapshot that `check` can read
//! (it may still refuse the contents on purpose: a signer it does not model, a
//! sponsored signer).
//!
//! The first 10 bytes choose the context: the network, the protocol version
//! (29 or 30, the two the checker models), the ledger, and whether the snapshot
//! carries `reserve` data and with what base reserve. The rest is the record.

#![no_main]

use libfuzzer_sys::fuzz_target;
use signerreach_core::{Code, Network, parse_snapshot};
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
        // Bit 0 of byte 6 asks for `reserve` data; bytes 7 to 9 give the base
        // reserve, so it is between 1 and 16,777,216 stroops.
        base_reserve: (head[6] & 1 == 1)
            .then(|| i64::from(u32::from_le_bytes([head[7], head[8], head[9], 0])) + 1),
    };

    if let Ok(snapshot) = snapshot_from_account(record, &context) {
        if let Err(issues) = parse_snapshot(&snapshot) {
            // Two refusals are the checker doing its job, not a bad mapping: a
            // signer of a kind it does not model (`SR-SIGNER-01`, written out
            // faithfully so the checker can say so) and a sponsored signer
            // (`SR-STATE-01`). Anything else means the mapping wrote a document
            // the checker cannot read.
            let by_design = |code: Code| matches!(code, Code::Signer01 | Code::State01);
            if issues.iter().any(|issue| !by_design(issue.code)) {
                let issues: Vec<String> = issues.iter().map(ToString::to_string).collect();
                panic!(
                    "the mapping wrote a snapshot that the checker cannot read: {}",
                    issues.join("; ")
                );
            }
        }
    }
});
