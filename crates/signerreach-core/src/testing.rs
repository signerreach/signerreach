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
    let signers: Vec<Value> = signers
        .iter()
        .map(|&(n, w)| json!({ "key": k(n), "type": "ed25519", "weight": w }))
        .collect();
    let doc = json!({
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
