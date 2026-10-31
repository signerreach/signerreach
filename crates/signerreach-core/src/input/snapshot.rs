//! The account snapshot document, `signerreach.snapshot/1`.
//!
//! See `docs/SPEC.md`, section 5.1. Anything the document leaves out that the
//! evaluation depends on is a refusal, never a default.

use stellar_strkey::Strkey;

use super::{
    InputIssue, Issues, Json, Obj, as_bool, as_str, as_threshold, as_u32, as_weight, check_schema,
    item, read_document, required,
};
use crate::account::{AccountState, SignerKey, Threshold, Thresholds, Weight};
use crate::diagnostic::Code;

/// The schema identifier a snapshot must carry.
pub const SNAPSHOT_SCHEMA: &str = "signerreach.snapshot/1";

/// The only protocol version whose rules are verified.
pub const SUPPORTED_PROTOCOL_VERSION: u32 = 29;

/// The Stellar network a snapshot was taken from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Network {
    /// The test network.
    Testnet,
    /// The public network.
    Pubnet,
}

impl Network {
    /// The name used in documents and reports.
    pub const fn as_str(self) -> &'static str {
        match self {
            Network::Testnet => "testnet",
            Network::Pubnet => "pubnet",
        }
    }
}

/// Where the snapshot data came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapshotSource {
    /// A Horizon account endpoint.
    Horizon,
    /// A Stellar RPC endpoint.
    Rpc,
    /// A hand-written or archived file.
    File,
}

impl SnapshotSource {
    /// The name used in documents and reports.
    pub const fn as_str(self) -> &'static str {
        match self {
            SnapshotSource::Horizon => "horizon",
            SnapshotSource::Rpc => "rpc",
            SnapshotSource::File => "file",
        }
    }
}

/// Where and when a snapshot was taken.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Provenance {
    as_of_ledger: u64,
    captured_at: String,
    source: SnapshotSource,
    protocol_version: u32,
}

impl Provenance {
    /// The ledger sequence the state is read as of.
    pub const fn as_of_ledger(&self) -> u64 {
        self.as_of_ledger
    }

    /// When the snapshot was captured, as an RFC 3339 timestamp.
    pub fn captured_at(&self) -> &str {
        &self.captured_at
    }

    /// Where the data came from.
    pub const fn source(&self) -> SnapshotSource {
        self.source
    }

    /// The protocol version the state belongs to.
    pub const fn protocol_version(&self) -> u32 {
        self.protocol_version
    }
}

/// A validated account snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snapshot {
    network: Network,
    provenance: Provenance,
    state: AccountState,
}

impl Snapshot {
    /// The network the snapshot was taken from.
    pub const fn network(&self) -> Network {
        self.network
    }

    /// Where and when it was taken.
    pub const fn provenance(&self) -> &Provenance {
        &self.provenance
    }

    /// The account state.
    pub const fn state(&self) -> &AccountState {
        &self.state
    }
}

/// Reads and validates a snapshot document.
///
/// Returns every problem found, not only the first.
pub fn parse_snapshot(text: &str) -> Result<Snapshot, Vec<InputIssue>> {
    let mut issues = Issues::default();
    let snapshot = read_snapshot(text, &mut issues);
    issues.into_result(snapshot)
}

fn read_snapshot(text: &str, issues: &mut Issues) -> Option<Snapshot> {
    let json = read_document(text, issues)?;
    let mut root = Obj::open(&json, "", issues)?;

    check_schema(&mut root, SNAPSHOT_SCHEMA, issues);
    let network = read_network(&mut root, issues);
    let account = read_account(&mut root, issues);
    let provenance = read_provenance(&mut root, issues);
    let master_weight = read_master_weight(&mut root, issues);
    let thresholds = read_thresholds(&mut root, issues);
    let signers = read_signers(&mut root, issues);
    read_sponsorship(&mut root, issues);
    root.finish(issues);

    if !issues.is_empty() {
        return None;
    }
    let state = match AccountState::new(account?, master_weight?, thresholds?, signers?) {
        Ok(state) => state,
        Err(error) => {
            issues.input("/signers", error.to_string());
            return None;
        }
    };
    Some(Snapshot {
        network: network?,
        provenance: provenance?,
        state,
    })
}

/// A required provenance field; a missing one means the freshness of the data
/// cannot be established.
fn required_provenance<'a>(obj: &mut Obj<'a>, key: &str, issues: &mut Issues) -> Option<&'a Json> {
    let found = obj.field(key, issues);
    if found.is_none() {
        issues.push(
            Code::Fresh01,
            &obj.path_of(key),
            "a required provenance field is missing",
        );
    }
    found
}

fn read_network(root: &mut Obj<'_>, issues: &mut Issues) -> Option<Network> {
    let path = root.path_of("network");
    let Some(value) = root.field("network", issues) else {
        issues.push(Code::Fresh01, &path, "the network is required");
        return None;
    };
    match as_str(value, &path, issues)? {
        "testnet" => Some(Network::Testnet),
        "pubnet" => Some(Network::Pubnet),
        _ => {
            issues.input(&path, "the network must be `testnet` or `pubnet`");
            None
        }
    }
}

fn read_account(root: &mut Obj<'_>, issues: &mut Issues) -> Option<SignerKey> {
    let path = root.path_of("account");
    let value = required(root, "account", issues)?;
    let text = as_str(value, &path, issues)?;
    match SignerKey::from_strkey(text) {
        Ok(key) => Some(key),
        Err(_) => {
            match Strkey::from_string(text) {
                Ok(
                    Strkey::Contract(_) | Strkey::MuxedAccountEd25519(_) | Strkey::MuxedContract(_),
                ) => issues.push(
                    Code::Input02,
                    &path,
                    "contract and muxed accounts are not supported; use a G... address",
                ),
                _ => issues.input(&path, "not a valid G... account address"),
            }
            None
        }
    }
}

fn read_provenance(root: &mut Obj<'_>, issues: &mut Issues) -> Option<Provenance> {
    let path = root.path_of("provenance");
    let Some(value) = root.field("provenance", issues) else {
        issues.push(Code::Fresh01, &path, "provenance is required");
        return None;
    };
    let mut obj = Obj::open(value, &path, issues)?;

    let mut as_of_ledger = None;
    if let Some(value) = required_provenance(&mut obj, "as_of_ledger", issues) {
        let at = obj.path_of("as_of_ledger");
        match value {
            Json::Number(number) => match number.as_u64() {
                Some(ledger) if ledger >= 1 => as_of_ledger = Some(ledger),
                _ => issues.input(&at, "expected a ledger sequence of at least 1"),
            },
            _ => issues.input(&at, "expected a ledger sequence of at least 1"),
        }
    }

    let mut captured_at = None;
    if let Some(value) = required_provenance(&mut obj, "captured_at", issues) {
        let at = obj.path_of("captured_at");
        if let Some(text) = as_str(value, &at, issues) {
            if is_rfc3339(text) {
                captured_at = Some(text.to_string());
            } else {
                issues.input(
                    &at,
                    "expected an RFC 3339 timestamp such as 2026-10-14T09:00:00Z",
                );
            }
        }
    }

    let mut source = None;
    if let Some(value) = required_provenance(&mut obj, "source", issues) {
        let at = obj.path_of("source");
        match as_str(value, &at, issues) {
            Some("horizon") => source = Some(SnapshotSource::Horizon),
            Some("rpc") => source = Some(SnapshotSource::Rpc),
            Some("file") => source = Some(SnapshotSource::File),
            Some(_) => issues.input(&at, "the source must be `horizon`, `rpc` or `file`"),
            None => {}
        }
    }

    let mut protocol_version = None;
    if let Some(value) = required_provenance(&mut obj, "protocol_version", issues) {
        let at = obj.path_of("protocol_version");
        if let Some(version) = as_u32(value, &at, issues) {
            if version == SUPPORTED_PROTOCOL_VERSION {
                protocol_version = Some(version);
            } else {
                issues.push(
                    Code::Rules01,
                    &at,
                    format!("only protocol {SUPPORTED_PROTOCOL_VERSION} is supported"),
                );
            }
        }
    }
    obj.finish(issues);

    Some(Provenance {
        as_of_ledger: as_of_ledger?,
        captured_at: captured_at?,
        source: source?,
        protocol_version: protocol_version?,
    })
}

fn read_master_weight(root: &mut Obj<'_>, issues: &mut Issues) -> Option<Weight> {
    let path = root.path_of("master_weight");
    let value = required(root, "master_weight", issues)?;
    as_weight(value, &path, issues)
}

fn read_thresholds(root: &mut Obj<'_>, issues: &mut Issues) -> Option<Thresholds> {
    let path = root.path_of("thresholds");
    let value = required(root, "thresholds", issues)?;
    let mut obj = Obj::open(value, &path, issues)?;
    let low = read_threshold(&mut obj, "low", issues);
    let medium = read_threshold(&mut obj, "medium", issues);
    let high = read_threshold(&mut obj, "high", issues);
    obj.finish(issues);
    Some(Thresholds::new(low?, medium?, high?))
}

fn read_threshold(obj: &mut Obj<'_>, key: &str, issues: &mut Issues) -> Option<Threshold> {
    let path = obj.path_of(key);
    let value = required(obj, key, issues)?;
    as_threshold(value, &path, issues)
}

fn read_signers(root: &mut Obj<'_>, issues: &mut Issues) -> Option<Vec<(SignerKey, Weight)>> {
    let path = root.path_of("signers");
    let value = required(root, "signers", issues)?;
    let Json::Array(entries) = value else {
        issues.input(&path, "expected an array");
        return None;
    };

    let mut signers = Vec::new();
    for (index, entry) in entries.iter().enumerate() {
        let entry_path = item(&path, index);
        let Some(mut obj) = Obj::open(entry, &entry_path, issues) else {
            continue;
        };

        let type_path = obj.path_of("type");
        let kind = match required(&mut obj, "type", issues) {
            Some(value) => as_str(value, &type_path, issues),
            None => None,
        };

        let key_path = obj.path_of("key");
        let key_text = match required(&mut obj, "key", issues) {
            Some(value) => as_str(value, &key_path, issues),
            None => None,
        };

        let weight_path = obj.path_of("weight");
        let weight = match required(&mut obj, "weight", issues) {
            Some(value) => as_weight(value, &weight_path, issues),
            None => None,
        };
        obj.finish(issues);

        match kind {
            Some("ed25519") => {
                if let Some(text) = key_text {
                    match SignerKey::from_strkey(text) {
                        Ok(key) => {
                            if let Some(weight) = weight {
                                signers.push((key, weight));
                            }
                        }
                        Err(_) => issues.input(&key_path, "not a valid G... signer address"),
                    }
                }
            }
            Some("pre_auth_tx" | "hash_x" | "ed25519_signed_payload") => issues.push(
                Code::Signer01,
                &entry_path,
                "only ed25519 signers are supported",
            ),
            Some(_) => issues.input(&type_path, "unknown signer type"),
            None => {}
        }
    }
    Some(signers)
}

fn read_sponsorship(root: &mut Obj<'_>, issues: &mut Issues) {
    let path = root.path_of("sponsored_signers");
    let Some(value) = root.field("sponsored_signers", issues) else {
        return;
    };
    if as_bool(value, &path, issues) == Some(true) {
        issues.push(
            Code::State01,
            &path,
            "sponsored signers and reserves are not modelled",
        );
    }
}

/// Whether `text` has the shape of an RFC 3339 timestamp with a plausible date
/// and time. This does not check the calendar (for example February 30).
fn is_rfc3339(text: &str) -> bool {
    let bytes = text.as_bytes();
    if !text.is_ascii() || bytes.len() < 20 {
        return false;
    }
    let separators = bytes[4] == b'-'
        && bytes[7] == b'-'
        && (bytes[10] == b'T' || bytes[10] == b't')
        && bytes[13] == b':'
        && bytes[16] == b':';
    let digits = [0, 1, 2, 3, 5, 6, 8, 9, 11, 12, 14, 15, 17, 18]
        .iter()
        .all(|&i| bytes[i].is_ascii_digit());
    if !separators || !digits {
        return false;
    }
    let number = |from: usize| text[from..from + 2].parse::<u32>().unwrap_or(u32::MAX);
    let in_range = (1..=12).contains(&number(5))
        && (1..=31).contains(&number(8))
        && number(11) <= 23
        && number(14) <= 59
        && number(17) <= 60;
    if !in_range {
        return false;
    }

    let mut rest = &text[19..];
    if let Some(after_dot) = rest.strip_prefix('.') {
        let fraction = after_dot.bytes().take_while(u8::is_ascii_digit).count();
        if fraction == 0 {
            return false;
        }
        rest = &after_dot[fraction..];
    }
    match rest {
        "Z" | "z" => true,
        offset if offset.len() == 6 => {
            let b = offset.as_bytes();
            (b[0] == b'+' || b[0] == b'-')
                && b[3] == b':'
                && [1, 2, 4, 5].iter().all(|&i| b[i].is_ascii_digit())
                && offset[1..3].parse::<u32>().is_ok_and(|hour| hour <= 23)
                && offset[4..6].parse::<u32>().is_ok_and(|minute| minute <= 59)
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::*;

    fn key(n: u8) -> String {
        SignerKey::from_bytes([n; 32]).to_string()
    }

    fn valid() -> Value {
        json!({
            "schema": "signerreach.snapshot/1",
            "network": "testnet",
            "account": key(1),
            "provenance": {
                "as_of_ledger": 5180915,
                "captured_at": "2026-10-14T09:00:00Z",
                "source": "horizon",
                "protocol_version": 29
            },
            "master_weight": 1,
            "thresholds": { "low": 1, "medium": 2, "high": 2 },
            "signers": [
                { "key": key(2), "type": "ed25519", "weight": 1 },
                { "key": key(3), "type": "ed25519", "weight": 1 }
            ],
            "sponsored_signers": false
        })
    }

    fn parse(value: &Value) -> Result<Snapshot, Vec<InputIssue>> {
        parse_snapshot(&value.to_string())
    }

    fn issues(value: &Value) -> Vec<InputIssue> {
        parse(value).expect_err("the document is refused")
    }

    fn has(found: &[InputIssue], code: Code, location: &str) -> bool {
        found
            .iter()
            .any(|issue| issue.code == code && issue.location == location)
    }

    #[test]
    fn accepts_a_valid_snapshot() {
        let snapshot = parse(&valid()).expect("valid snapshot");
        assert_eq!(snapshot.network(), Network::Testnet);
        assert_eq!(snapshot.provenance().as_of_ledger(), 5180915);
        assert_eq!(snapshot.provenance().captured_at(), "2026-10-14T09:00:00Z");
        assert_eq!(snapshot.provenance().source(), SnapshotSource::Horizon);
        assert_eq!(snapshot.provenance().protocol_version(), 29);
        assert_eq!(snapshot.state().additional_signer_count(), 2);
        assert_eq!(snapshot.state().master_weight().get(), 1);
    }

    #[test]
    fn sponsored_signers_is_optional_and_false_is_accepted() {
        let mut doc = valid();
        doc.as_object_mut().unwrap().remove("sponsored_signers");
        assert!(parse(&doc).is_ok());
    }

    #[test]
    fn sponsored_signers_true_is_not_modelled() {
        let mut doc = valid();
        doc["sponsored_signers"] = json!(true);
        assert!(has(&issues(&doc), Code::State01, "/sponsored_signers"));
    }

    #[test]
    fn a_wrong_or_missing_schema_is_refused() {
        let mut doc = valid();
        doc["schema"] = json!("signerreach.snapshot/2");
        assert!(has(&issues(&doc), Code::Input01, "/schema"));
        doc.as_object_mut().unwrap().remove("schema");
        assert!(has(&issues(&doc), Code::Input01, "/schema"));
    }

    #[test]
    fn a_missing_network_means_provenance_is_not_established() {
        let mut doc = valid();
        doc.as_object_mut().unwrap().remove("network");
        assert!(has(&issues(&doc), Code::Fresh01, "/network"));
    }

    #[test]
    fn an_unknown_network_is_an_input_error() {
        let mut doc = valid();
        doc["network"] = json!("mainnet");
        assert!(has(&issues(&doc), Code::Input01, "/network"));
    }

    #[test]
    fn missing_provenance_is_fresh_01() {
        let mut doc = valid();
        doc.as_object_mut().unwrap().remove("provenance");
        assert!(has(&issues(&doc), Code::Fresh01, "/provenance"));
    }

    #[test]
    fn each_missing_provenance_field_is_fresh_01() {
        for field in ["as_of_ledger", "captured_at", "source", "protocol_version"] {
            let mut doc = valid();
            doc["provenance"].as_object_mut().unwrap().remove(field);
            let location = format!("/provenance/{field}");
            assert!(has(&issues(&doc), Code::Fresh01, &location), "{field}");
        }
    }

    #[test]
    fn ledger_zero_is_refused() {
        let mut doc = valid();
        doc["provenance"]["as_of_ledger"] = json!(0);
        assert!(has(
            &issues(&doc),
            Code::Input01,
            "/provenance/as_of_ledger"
        ));
    }

    #[test]
    fn protocol_other_than_29_is_rules_01() {
        let mut doc = valid();
        doc["provenance"]["protocol_version"] = json!(28);
        assert!(has(
            &issues(&doc),
            Code::Rules01,
            "/provenance/protocol_version"
        ));
    }

    #[test]
    fn a_contract_account_is_input_02() {
        let mut doc = valid();
        doc["account"] = json!(format!("{}", stellar_strkey::Contract([4u8; 32])));
        assert!(has(&issues(&doc), Code::Input02, "/account"));
    }

    #[test]
    fn a_muxed_account_is_input_02() {
        let mut doc = valid();
        let muxed = stellar_strkey::ed25519::MuxedAccount {
            ed25519: [4u8; 32],
            id: 7,
        };
        doc["account"] = json!(format!("{muxed}"));
        assert!(has(&issues(&doc), Code::Input02, "/account"));
    }

    #[test]
    fn a_garbage_account_is_input_01() {
        let mut doc = valid();
        doc["account"] = json!("not an account");
        assert!(has(&issues(&doc), Code::Input01, "/account"));
    }

    #[test]
    fn values_from_the_document_are_never_repeated_in_messages() {
        let marker = "SECRETMARKER0123456789";
        let mut doc = valid();
        doc["account"] = json!(marker);
        doc["master_weight"] = json!(marker);
        doc["thresholds"]["low"] = json!(marker);
        doc["signers"][0]["key"] = json!(marker);
        doc["signers"][0]["weight"] = json!(marker);
        doc["signers"][1]["type"] = json!(marker);
        doc["network"] = json!(marker);
        doc["provenance"]["source"] = json!(marker);
        doc["provenance"]["captured_at"] = json!(marker);
        let found = issues(&doc);
        assert!(found.len() >= 5);
        for issue in &found {
            assert!(!issue.to_string().contains(marker), "{issue}");
        }
    }

    #[test]
    fn weights_outside_the_range_or_not_whole_are_refused() {
        for bad in [json!(256), json!(1.5), json!(-1), json!("1"), json!(null)] {
            let mut doc = valid();
            doc["master_weight"] = bad.clone();
            assert!(has(&issues(&doc), Code::Input01, "/master_weight"), "{bad}");
        }
    }

    #[test]
    fn thresholds_above_255_are_refused() {
        let mut doc = valid();
        doc["thresholds"]["high"] = json!(256);
        assert!(has(&issues(&doc), Code::Input01, "/thresholds/high"));
    }

    #[test]
    fn a_missing_threshold_is_refused() {
        let mut doc = valid();
        doc["thresholds"].as_object_mut().unwrap().remove("medium");
        assert!(has(&issues(&doc), Code::Input01, "/thresholds/medium"));
    }

    #[test]
    fn signers_of_other_types_are_signer_01() {
        for kind in ["pre_auth_tx", "hash_x", "ed25519_signed_payload"] {
            let mut doc = valid();
            doc["signers"][0]["type"] = json!(kind);
            assert!(has(&issues(&doc), Code::Signer01, "/signers/0"), "{kind}");
        }
    }

    #[test]
    fn an_unknown_signer_type_is_input_01() {
        let mut doc = valid();
        doc["signers"][0]["type"] = json!("rsa");
        assert!(has(&issues(&doc), Code::Input01, "/signers/0/type"));
    }

    #[test]
    fn a_signer_that_is_the_account_is_refused() {
        let mut doc = valid();
        doc["signers"][0]["key"] = json!(key(1));
        assert!(has(&issues(&doc), Code::Input01, "/signers"));
    }

    #[test]
    fn a_duplicate_signer_is_refused() {
        let mut doc = valid();
        doc["signers"][1]["key"] = json!(key(2));
        assert!(has(&issues(&doc), Code::Input01, "/signers"));
    }

    #[test]
    fn a_zero_weight_signer_is_refused() {
        let mut doc = valid();
        doc["signers"][0]["weight"] = json!(0);
        assert!(has(&issues(&doc), Code::Input01, "/signers"));
    }

    #[test]
    fn more_than_20_additional_signers_are_refused() {
        let mut doc = valid();
        let many: Vec<Value> = (10..31u8)
            .map(|n| json!({ "key": key(n), "type": "ed25519", "weight": 1 }))
            .collect();
        assert_eq!(many.len(), 21);
        doc["signers"] = json!(many);
        assert!(has(&issues(&doc), Code::Input01, "/signers"));
    }

    #[test]
    fn exactly_20_additional_signers_are_accepted() {
        let mut doc = valid();
        let twenty: Vec<Value> = (10..30u8)
            .map(|n| json!({ "key": key(n), "type": "ed25519", "weight": 1 }))
            .collect();
        doc["signers"] = json!(twenty);
        assert_eq!(
            parse(&doc)
                .expect("valid")
                .state()
                .additional_signer_count(),
            20
        );
    }

    #[test]
    fn unknown_fields_are_refused_at_every_level() {
        let mut doc = valid();
        doc["extra"] = json!(1);
        doc["thresholds"]["top"] = json!(1);
        doc["signers"][0]["note"] = json!("x");
        doc["provenance"]["who"] = json!("x");
        let found = issues(&doc);
        for location in ["", "/thresholds", "/signers/0", "/provenance"] {
            assert!(has(&found, Code::Input01, location), "{location}");
        }
    }

    #[test]
    fn a_duplicate_key_is_refused() {
        let text = valid().to_string();
        let body = text.strip_suffix('}').expect("an object");
        let doubled = format!("{body},\"master_weight\":9}}");
        let found = parse_snapshot(&doubled).expect_err("duplicate key");
        assert!(has(&found, Code::Input01, "/master_weight"));
    }

    #[test]
    fn several_problems_are_all_reported() {
        let mut doc = valid();
        doc["master_weight"] = json!(300);
        doc["thresholds"]["low"] = json!(300);
        doc["provenance"]["protocol_version"] = json!(1);
        assert!(issues(&doc).len() >= 3);
    }

    #[test]
    fn text_that_is_not_json_is_refused_with_a_position() {
        let found = parse_snapshot("{").expect_err("not json");
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].code, Code::Input01);
        assert!(found[0].message.contains("line 1"), "{}", found[0]);
        assert!(parse_snapshot("").is_err());
        assert!(parse_snapshot("{} trailing").is_err());
    }

    #[test]
    fn a_document_that_is_not_an_object_is_refused() {
        let found = parse_snapshot("[]").expect_err("array");
        assert!(has(&found, Code::Input01, ""));
    }

    #[test]
    fn deeply_nested_input_is_refused_without_a_crash() {
        let text = "[".repeat(5000);
        assert!(parse_snapshot(&text).is_err());
    }

    #[test]
    fn oversize_input_is_refused() {
        let text = " ".repeat(super::super::MAX_INPUT_BYTES + 1);
        let found = parse_snapshot(&text).expect_err("too large");
        assert!(found[0].message.contains("limit"));
    }

    #[test]
    fn timestamps_accept_utc_fraction_and_offset() {
        for good in [
            "2026-10-14T09:00:00Z",
            "2026-10-14T09:00:00.123Z",
            "2026-10-14T09:00:00+01:00",
            "2026-10-14t09:00:00-05:30",
        ] {
            assert!(is_rfc3339(good), "{good}");
        }
    }

    #[test]
    fn timestamps_refuse_other_shapes() {
        for bad in [
            "",
            "yesterday",
            "2026-10-14",
            "2026-10-14 09:00:00Z",
            "2026-13-14T09:00:00Z",
            "2026-10-32T09:00:00Z",
            "2026-10-14T24:00:00Z",
            "2026-10-14T09:60:00Z",
            "2026-10-14T09:00:00",
            "2026-10-14T09:00:00.Z",
            "2026-10-14T09:00:00+0100",
            "2026-10-14T09:00:00+25:00",
        ] {
            assert!(!is_rfc3339(bad), "{bad}");
        }
    }
}
