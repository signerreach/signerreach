//! Builders shared by tests. Compiled only for tests.

use serde_json::{Value, json};

use crate::account::SignerKey;
use crate::input::availability::{Availability, parse_availability};
use crate::input::plan::{Plan, parse_plan};
use crate::input::snapshot::{Snapshot, parse_snapshot};

pub fn k(n: u8) -> String {
    SignerKey::from_bytes([n; 32]).to_string()
}

pub fn snapshot(master: u32, signers: &[(u8, u32)], thresholds: (u32, u32, u32)) -> Snapshot {
    snapshot_document(master, signers, thresholds, None)
}

/// The balance figures a funded snapshot carries: balance, selling
/// liabilities and sub-entry count, with no sponsorship and a base reserve of
/// 5,000,000 stroops.
pub fn snapshot_funded(
    master: u32,
    signers: &[(u8, u32)],
    thresholds: (u32, u32, u32),
    figures: (i64, i64, u32),
) -> Snapshot {
    let (balance, selling_liabilities, subentry_count) = figures;
    snapshot_document(
        master,
        signers,
        thresholds,
        Some(json!({
            "balance": balance,
            "selling_liabilities": selling_liabilities,
            "subentry_count": subentry_count,
            "num_sponsoring": 0,
            "num_sponsored": 0,
            "base_reserve": 5_000_000
        })),
    )
}

fn snapshot_document(
    master: u32,
    signers: &[(u8, u32)],
    thresholds: (u32, u32, u32),
    reserve: Option<Value>,
) -> Snapshot {
    let signers: Vec<Value> = signers
        .iter()
        .map(|&(n, w)| json!({ "key": k(n), "type": "ed25519", "weight": w }))
        .collect();
    let mut doc = json!({
        "schema": "signerreach.snapshot/1",
        "network": "testnet",
        "account": k(1),
        "provenance": {
            "as_of_ledger": 100,
            "captured_at": "2026-10-14T09:00:00Z",
            "source": "file",
            "protocol_version": 29
        },
        "master_weight": master,
        "thresholds": { "low": thresholds.0, "medium": thresholds.1, "high": thresholds.2 },
        "signers": signers
    });
    if let Some(reserve) = reserve {
        doc["reserve"] = reserve;
    }
    parse_snapshot(&doc.to_string()).expect("valid snapshot")
}

pub fn plan(transactions: Vec<Value>) -> Plan {
    let doc = json!({ "schema": "signerreach.plan/1", "transactions": transactions });
    parse_plan(&doc.to_string()).expect("valid plan")
}

pub fn tx(id: &str, signed_by: &[u8], operations: Vec<Value>) -> Value {
    let keys: Vec<String> = signed_by.iter().map(|&n| k(n)).collect();
    json!({ "id": id, "signed_by": keys, "operations": operations })
}

pub fn tx_with_fee(id: &str, signed_by: &[u8], fee: u32, operations: Vec<Value>) -> Value {
    let mut transaction = tx(id, signed_by, operations);
    transaction["fee"] = json!(fee);
    transaction
}

pub fn signer_op(n: u8, weight: u32) -> Value {
    json!({ "op": "set_options", "signer": { "key": k(n), "weight": weight } })
}

pub fn availability(statuses: &[(u8, &str)]) -> Availability {
    availability_with(statuses, json!([]), 0, "recoverable")
}

pub fn availability_with(
    statuses: &[(u8, &str)],
    scenarios: Value,
    survive: u32,
    intent: &str,
) -> Availability {
    let mut signers = serde_json::Map::new();
    for &(n, status) in statuses {
        signers.insert(k(n), json!(status));
    }
    let doc = json!({
        "schema": "signerreach.availability/1",
        "signers": signers,
        "scenarios": scenarios,
        "require_survive_loss_of": survive,
        "intent": intent
    });
    parse_availability(&doc.to_string()).expect("valid availability")
}
