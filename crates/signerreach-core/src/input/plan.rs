//! The plan document, `signerreach.plan/1`.
//!
//! See `docs/SPEC.md`, section 5.2. A plan is an ordered list of transactions;
//! the boundaries between them are part of the meaning, so they are kept
//! exactly as written.
//!
//! This module only reads and validates the document. Whether a plan is
//! authorized by an account is decided later, against a snapshot.

use std::collections::BTreeSet;

use stellar_strkey::Strkey;

use super::{
    InputIssue, Issues, Json, Obj, as_str, as_threshold, as_u32, as_weight, check_schema, child,
    item, read_document, required,
};
use crate::account::{SignerKey, Threshold, Weight};
use crate::diagnostic::Code;

/// The schema identifier a plan must carry.
pub const PLAN_SCHEMA: &str = "signerreach.plan/1";

/// The most transactions one plan may hold. A limit of this tool, not of Stellar.
pub const MAX_TRANSACTIONS: usize = 1000;

/// The most operations one transaction may hold (a Stellar protocol limit).
pub const MAX_OPERATIONS_PER_TRANSACTION: usize = 100;

/// The most signatures one transaction may carry (a Stellar protocol limit).
pub const MAX_SIGNATURES_PER_TRANSACTION: usize = 20;

/// The longest transaction identifier.
const MAX_ID_LENGTH: usize = 64;

/// The longest home domain, in bytes (a Stellar protocol limit).
const MAX_HOME_DOMAIN_BYTES: usize = 32;

/// The account flag bits that exist at protocol 29.
const ACCOUNT_FLAG_MASK: u32 = 0xF;

/// A signer added, re-weighted or removed (weight 0) by an operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SignerChange {
    key: SignerKey,
    weight: Weight,
}

impl SignerChange {
    /// The signer's key.
    pub const fn key(&self) -> SignerKey {
        self.key
    }

    /// The new weight; 0 removes the signer.
    pub const fn weight(&self) -> Weight {
        self.weight
    }
}

/// A `SetOptions` operation. Every field is optional, as on the network.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetOptions {
    source: Option<SignerKey>,
    master_weight: Option<Weight>,
    low: Option<Threshold>,
    medium: Option<Threshold>,
    high: Option<Threshold>,
    signer: Option<SignerChange>,
    home_domain: Option<String>,
    set_flags: Option<u32>,
    clear_flags: Option<u32>,
}

impl SetOptions {
    /// The operation's own source account, if one is declared.
    pub const fn source(&self) -> Option<SignerKey> {
        self.source
    }

    /// A new master weight.
    pub const fn master_weight(&self) -> Option<Weight> {
        self.master_weight
    }

    /// A new LOW threshold.
    pub const fn low(&self) -> Option<Threshold> {
        self.low
    }

    /// A new MEDIUM threshold.
    pub const fn medium(&self) -> Option<Threshold> {
        self.medium
    }

    /// A new HIGH threshold.
    pub const fn high(&self) -> Option<Threshold> {
        self.high
    }

    /// A signer to add, re-weight or remove.
    pub const fn signer(&self) -> Option<SignerChange> {
        self.signer
    }

    /// A new home domain.
    pub fn home_domain(&self) -> Option<&str> {
        self.home_domain.as_deref()
    }

    /// Account flags to set.
    pub const fn set_flags(&self) -> Option<u32> {
        self.set_flags
    }

    /// Account flags to clear.
    pub const fn clear_flags(&self) -> Option<u32> {
        self.clear_flags
    }
}

/// One transaction of a plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedTransaction {
    id: String,
    source: Option<SignerKey>,
    signed_by: Vec<SignerKey>,
    operations: Vec<SetOptions>,
}

impl PlannedTransaction {
    /// The identifier used in reports.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// The transaction's source account, if one is declared.
    pub const fn source(&self) -> Option<SignerKey> {
        self.source
    }

    /// The keys that will sign the transaction.
    pub fn signed_by(&self) -> &[SignerKey] {
        &self.signed_by
    }

    /// The operations, in order.
    pub fn operations(&self) -> &[SetOptions] {
        &self.operations
    }
}

/// A validated plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    transactions: Vec<PlannedTransaction>,
}

impl Plan {
    /// The transactions, in the order they are applied. May be empty.
    pub fn transactions(&self) -> &[PlannedTransaction] {
        &self.transactions
    }
}

/// Reads and validates a plan document.
///
/// Returns every problem found, not only the first.
pub fn parse_plan(text: &str) -> Result<Plan, Vec<InputIssue>> {
    let mut issues = Issues::default();
    let plan = read_plan(text, &mut issues);
    issues.into_result(plan)
}

fn read_plan(text: &str, issues: &mut Issues) -> Option<Plan> {
    let json = read_document(text, issues)?;
    let mut root = Obj::open(&json, "", issues)?;

    check_schema(&mut root, PLAN_SCHEMA, issues);
    let transactions = read_transactions(&mut root, issues);
    root.finish(issues);

    Some(Plan {
        transactions: transactions?,
    })
}

fn read_transactions(root: &mut Obj<'_>, issues: &mut Issues) -> Option<Vec<PlannedTransaction>> {
    let path = root.path_of("transactions");
    let value = required(root, "transactions", issues)?;
    let Json::Array(entries) = value else {
        issues.input(&path, "expected an array");
        return None;
    };
    if entries.len() > MAX_TRANSACTIONS {
        issues.input(
            &path,
            format!("at most {MAX_TRANSACTIONS} transactions are supported"),
        );
        return None;
    }

    let mut ids = BTreeSet::new();
    let mut transactions = Vec::new();
    for (index, entry) in entries.iter().enumerate() {
        let entry_path = item(&path, index);
        if let Some(transaction) = read_transaction(entry, &entry_path, &mut ids, issues) {
            transactions.push(transaction);
        }
    }
    Some(transactions)
}

fn read_transaction(
    entry: &Json,
    path: &str,
    ids: &mut BTreeSet<String>,
    issues: &mut Issues,
) -> Option<PlannedTransaction> {
    let mut obj = Obj::open(entry, path, issues)?;

    let id = read_id(&mut obj, ids, issues);
    let source = read_optional_key(&mut obj, "source", issues);
    let signed_by = read_signed_by(&mut obj, issues);
    let operations = read_operations(&mut obj, issues);

    for unsupported in ["fee_bump", "preconditions"] {
        if obj.field(unsupported, issues).is_some() {
            issues.push(
                Code::Tx04,
                &obj.path_of(unsupported),
                "fee-bump transactions and preconditions are not supported",
            );
        }
    }
    obj.finish(issues);

    Some(PlannedTransaction {
        id: id?,
        source,
        signed_by: signed_by?,
        operations: operations?,
    })
}

fn read_id(obj: &mut Obj<'_>, ids: &mut BTreeSet<String>, issues: &mut Issues) -> Option<String> {
    let path = obj.path_of("id");
    let value = required(obj, "id", issues)?;
    let text = as_str(value, &path, issues)?;
    let well_formed = !text.is_empty()
        && text.len() <= MAX_ID_LENGTH
        && text
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'));
    if !well_formed {
        issues.input(
            &path,
            format!(
                "an id is 1 to {MAX_ID_LENGTH} characters from letters, digits, `.`, `_` and `-`"
            ),
        );
        return None;
    }
    if !ids.insert(text.to_string()) {
        issues.input(&path, "the id is used by more than one transaction");
        return None;
    }
    Some(text.to_string())
}

fn read_signed_by(obj: &mut Obj<'_>, issues: &mut Issues) -> Option<Vec<SignerKey>> {
    let path = obj.path_of("signed_by");
    let value = required(obj, "signed_by", issues)?;
    let Json::Array(entries) = value else {
        issues.input(&path, "expected an array");
        return None;
    };
    if entries.len() > MAX_SIGNATURES_PER_TRANSACTION {
        issues.input(
            &path,
            format!("a transaction carries at most {MAX_SIGNATURES_PER_TRANSACTION} signatures"),
        );
        return None;
    }

    let mut keys: Vec<SignerKey> = Vec::new();
    for (index, entry) in entries.iter().enumerate() {
        let entry_path = item(&path, index);
        let Some(key) = as_key(entry, &entry_path, issues) else {
            continue;
        };
        if keys.contains(&key) {
            issues.input(&entry_path, "the key is listed more than once");
        } else {
            keys.push(key);
        }
    }
    Some(keys)
}

fn read_operations(obj: &mut Obj<'_>, issues: &mut Issues) -> Option<Vec<SetOptions>> {
    let path = obj.path_of("operations");
    let value = required(obj, "operations", issues)?;
    let Json::Array(entries) = value else {
        issues.input(&path, "expected an array");
        return None;
    };
    if entries.is_empty() {
        issues.input(&path, "a transaction needs at least one operation");
        return None;
    }
    if entries.len() > MAX_OPERATIONS_PER_TRANSACTION {
        issues.input(
            &path,
            format!("a transaction holds at most {MAX_OPERATIONS_PER_TRANSACTION} operations"),
        );
        return None;
    }

    let mut operations = Vec::new();
    for (index, entry) in entries.iter().enumerate() {
        if let Some(operation) = read_operation(entry, &item(&path, index), issues) {
            operations.push(operation);
        }
    }
    Some(operations)
}

fn read_operation(entry: &Json, path: &str, issues: &mut Issues) -> Option<SetOptions> {
    let Json::Object(pairs) = entry else {
        issues.input(path, "expected an object");
        return None;
    };

    // The operation type decides which other fields are expected, so it is
    // read first and an unsupported type is reported as such, not as a pile of
    // unknown fields.
    let op_path = child(path, "op");
    match pairs.iter().find(|(name, _)| name == "op") {
        None => {
            issues.input(&op_path, "missing required field");
            return None;
        }
        Some((_, Json::String(name))) if name == "set_options" => {}
        Some((_, Json::String(_))) => {
            issues.push(Code::Op01, &op_path, "only set_options is supported");
            return None;
        }
        Some(_) => {
            issues.input(&op_path, "expected a string");
            return None;
        }
    }

    let mut obj = Obj::open(entry, path, issues)?;
    let _ = obj.field("op", issues);

    let source = read_optional_key(&mut obj, "source", issues);
    let master_weight = read_optional(&mut obj, "master_weight", issues, as_weight);
    let low = read_optional(&mut obj, "low", issues, as_threshold);
    let medium = read_optional(&mut obj, "medium", issues, as_threshold);
    let high = read_optional(&mut obj, "high", issues, as_threshold);
    let signer = read_signer(&mut obj, issues);
    let home_domain = read_optional(&mut obj, "home_domain", issues, as_home_domain);
    let set_flags = read_optional(&mut obj, "set_flags", issues, as_flags);
    let clear_flags = read_optional(&mut obj, "clear_flags", issues, as_flags);
    obj.finish(issues);

    if set_flags
        .zip(clear_flags)
        .is_some_and(|(set, clear)| set & clear != 0)
    {
        issues.input(path, "the same account flag is both set and cleared");
    }

    Some(SetOptions {
        source,
        master_weight,
        low,
        medium,
        high,
        signer,
        home_domain,
        set_flags,
        clear_flags,
    })
}

fn read_signer(obj: &mut Obj<'_>, issues: &mut Issues) -> Option<SignerChange> {
    let path = obj.path_of("signer");
    let value = obj.field("signer", issues)?;
    let mut signer = Obj::open(value, &path, issues)?;

    let key_path = signer.path_of("key");
    let key = match required(&mut signer, "key", issues) {
        Some(value) => as_key(value, &key_path, issues),
        None => None,
    };
    let weight_path = signer.path_of("weight");
    let weight = match required(&mut signer, "weight", issues) {
        Some(value) => as_weight(value, &weight_path, issues),
        None => None,
    };
    signer.finish(issues);

    Some(SignerChange {
        key: key?,
        weight: weight?,
    })
}

/// An optional field read with `read`; absent gives `None`, invalid gives an
/// issue and `None`.
fn read_optional<T>(
    obj: &mut Obj<'_>,
    key: &str,
    issues: &mut Issues,
    read: fn(&Json, &str, &mut Issues) -> Option<T>,
) -> Option<T> {
    let path = obj.path_of(key);
    let value = obj.field(key, issues)?;
    read(value, &path, issues)
}

fn read_optional_key(obj: &mut Obj<'_>, key: &str, issues: &mut Issues) -> Option<SignerKey> {
    read_optional(obj, key, issues, as_key)
}

/// An ed25519 account key. Other key kinds are recognised and reported as such.
fn as_key(value: &Json, path: &str, issues: &mut Issues) -> Option<SignerKey> {
    let text = as_str(value, path, issues)?;
    match SignerKey::from_strkey(text) {
        Ok(key) => Some(key),
        Err(_) => {
            match Strkey::from_string(text) {
                Ok(Strkey::PreAuthTx(_) | Strkey::HashX(_) | Strkey::SignedPayloadEd25519(_)) => {
                    issues.push(Code::Signer01, path, "only ed25519 keys are supported")
                }
                _ => issues.input(path, "not a valid G... address"),
            }
            None
        }
    }
}

fn as_home_domain(value: &Json, path: &str, issues: &mut Issues) -> Option<String> {
    let text = as_str(value, path, issues)?;
    let printable = text.bytes().all(|b| (0x20..=0x7E).contains(&b));
    if text.len() > MAX_HOME_DOMAIN_BYTES || !printable {
        issues.input(
            path,
            format!("a home domain is at most {MAX_HOME_DOMAIN_BYTES} printable ASCII characters"),
        );
        return None;
    }
    Some(text.to_string())
}

fn as_flags(value: &Json, path: &str, issues: &mut Issues) -> Option<u32> {
    let flags = as_u32(value, path, issues)?;
    if flags & !ACCOUNT_FLAG_MASK != 0 {
        issues.input(
            path,
            "the value contains account flag bits that do not exist",
        );
        return None;
    }
    Some(flags)
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
            "schema": "signerreach.plan/1",
            "transactions": [
                {
                    "id": "tx1",
                    "signed_by": [key(1), key(2)],
                    "operations": [
                        { "op": "set_options", "signer": { "key": key(3), "weight": 1 } },
                        { "op": "set_options", "signer": { "key": key(4), "weight": 0 } }
                    ]
                },
                {
                    "id": "tx2",
                    "signed_by": [key(1)],
                    "operations": [
                        { "op": "set_options", "master_weight": 2, "high": 3 }
                    ]
                }
            ]
        })
    }

    fn parse(value: &Value) -> Result<Plan, Vec<InputIssue>> {
        parse_plan(&value.to_string())
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
    fn accepts_a_valid_plan_and_keeps_transaction_boundaries() {
        let plan = parse(&valid()).expect("valid plan");
        assert_eq!(plan.transactions().len(), 2);
        let first = &plan.transactions()[0];
        assert_eq!(first.id(), "tx1");
        assert_eq!(first.signed_by().len(), 2);
        assert_eq!(first.operations().len(), 2);
        let added = first.operations()[0].signer().expect("signer");
        assert_eq!(added.weight().get(), 1);
        let removed = first.operations()[1].signer().expect("signer");
        assert!(removed.weight().is_zero());
        let second = &plan.transactions()[1];
        assert_eq!(
            second.operations()[0].master_weight().map(Weight::get),
            Some(2)
        );
        assert_eq!(second.operations()[0].high().map(Threshold::get), Some(3));
        assert!(second.operations()[0].low().is_none());
    }

    #[test]
    fn an_empty_plan_is_a_state_only_check() {
        let doc = json!({ "schema": "signerreach.plan/1", "transactions": [] });
        assert!(parse(&doc).expect("valid").transactions().is_empty());
    }

    #[test]
    fn an_operation_with_no_fields_is_accepted() {
        let mut doc = valid();
        doc["transactions"][0]["operations"] = json!([{ "op": "set_options" }]);
        assert!(parse(&doc).is_ok());
    }

    #[test]
    fn a_wrong_or_missing_schema_is_refused() {
        let mut doc = valid();
        doc["schema"] = json!("signerreach.snapshot/1");
        assert!(has(&issues(&doc), Code::Input01, "/schema"));
        doc.as_object_mut().unwrap().remove("schema");
        assert!(has(&issues(&doc), Code::Input01, "/schema"));
    }

    #[test]
    fn transactions_must_be_an_array() {
        let mut doc = valid();
        doc["transactions"] = json!({});
        assert!(has(&issues(&doc), Code::Input01, "/transactions"));
        doc.as_object_mut().unwrap().remove("transactions");
        assert!(has(&issues(&doc), Code::Input01, "/transactions"));
    }

    #[test]
    fn more_than_1000_transactions_are_refused() {
        let many: Vec<Value> = (0..1001)
            .map(|n| {
                json!({
                    "id": format!("t{n}"),
                    "signed_by": [],
                    "operations": [{ "op": "set_options" }]
                })
            })
            .collect();
        let doc = json!({ "schema": "signerreach.plan/1", "transactions": many });
        assert!(has(&issues(&doc), Code::Input01, "/transactions"));
    }

    #[test]
    fn a_transaction_needs_between_1_and_100_operations() {
        let mut doc = valid();
        doc["transactions"][0]["operations"] = json!([]);
        assert!(has(
            &issues(&doc),
            Code::Input01,
            "/transactions/0/operations"
        ));
        let hundred: Vec<Value> = (0..100).map(|_| json!({ "op": "set_options" })).collect();
        doc["transactions"][0]["operations"] = json!(hundred);
        assert!(parse(&doc).is_ok());
        let too_many: Vec<Value> = (0..101).map(|_| json!({ "op": "set_options" })).collect();
        doc["transactions"][0]["operations"] = json!(too_many);
        assert!(has(
            &issues(&doc),
            Code::Input01,
            "/transactions/0/operations"
        ));
    }

    #[test]
    fn ids_must_be_well_formed_and_unique() {
        for bad in ["", "has space", "slash/ed", &"a".repeat(65)] {
            let mut doc = valid();
            doc["transactions"][0]["id"] = json!(bad);
            assert!(
                has(&issues(&doc), Code::Input01, "/transactions/0/id"),
                "{bad:?}"
            );
        }
        let mut doc = valid();
        doc["transactions"][1]["id"] = json!("tx1");
        assert!(has(&issues(&doc), Code::Input01, "/transactions/1/id"));
    }

    #[test]
    fn signed_by_is_required_and_may_be_empty() {
        let mut doc = valid();
        doc["transactions"][0]
            .as_object_mut()
            .unwrap()
            .remove("signed_by");
        assert!(has(
            &issues(&doc),
            Code::Input01,
            "/transactions/0/signed_by"
        ));
        let mut doc = valid();
        doc["transactions"][0]["signed_by"] = json!([]);
        assert!(parse(&doc).is_ok());
    }

    #[test]
    fn signed_by_refuses_duplicates_and_more_than_20() {
        let mut doc = valid();
        doc["transactions"][0]["signed_by"] = json!([key(1), key(1)]);
        assert!(has(
            &issues(&doc),
            Code::Input01,
            "/transactions/0/signed_by/1"
        ));
        let twenty: Vec<String> = (1..=20u8).map(key).collect();
        doc["transactions"][0]["signed_by"] = json!(twenty);
        assert!(parse(&doc).is_ok());
        let twenty_one: Vec<String> = (1..=21u8).map(key).collect();
        doc["transactions"][0]["signed_by"] = json!(twenty_one);
        assert!(has(
            &issues(&doc),
            Code::Input01,
            "/transactions/0/signed_by"
        ));
    }

    #[test]
    fn keys_of_other_kinds_are_signer_01_and_garbage_is_input_01() {
        let hash_x = format!("{}", stellar_strkey::HashX([5u8; 32]));
        let mut doc = valid();
        doc["transactions"][0]["signed_by"] = json!([hash_x]);
        assert!(has(
            &issues(&doc),
            Code::Signer01,
            "/transactions/0/signed_by/0"
        ));
        doc["transactions"][0]["signed_by"] = json!(["nope"]);
        assert!(has(
            &issues(&doc),
            Code::Input01,
            "/transactions/0/signed_by/0"
        ));
    }

    #[test]
    fn other_operation_types_are_op_01() {
        for name in ["account_merge", "payment", "create_account", ""] {
            let mut doc = valid();
            doc["transactions"][0]["operations"][0] = json!({ "op": name, "destination": key(9) });
            assert!(
                has(&issues(&doc), Code::Op01, "/transactions/0/operations/0/op"),
                "{name:?}"
            );
        }
    }

    #[test]
    fn a_missing_or_non_string_op_is_input_01() {
        let mut doc = valid();
        doc["transactions"][0]["operations"][0] = json!({ "master_weight": 1 });
        assert!(has(
            &issues(&doc),
            Code::Input01,
            "/transactions/0/operations/0/op"
        ));
        doc["transactions"][0]["operations"][0] = json!({ "op": 7 });
        assert!(has(
            &issues(&doc),
            Code::Input01,
            "/transactions/0/operations/0/op"
        ));
    }

    #[test]
    fn weights_and_thresholds_are_range_checked() {
        let at = "/transactions/1/operations/0";
        for (field, bad) in [("master_weight", 256), ("high", 256), ("low", 300)] {
            let mut doc = valid();
            doc["transactions"][1]["operations"][0][field] = json!(bad);
            let location = format!("{at}/{field}");
            assert!(has(&issues(&doc), Code::Input01, &location), "{field}");
        }
        let mut doc = valid();
        doc["transactions"][0]["operations"][0]["signer"]["weight"] = json!(256);
        assert!(has(
            &issues(&doc),
            Code::Input01,
            "/transactions/0/operations/0/signer/weight"
        ));
    }

    #[test]
    fn a_signer_of_another_type_is_signer_01() {
        let pre_auth = format!("{}", stellar_strkey::PreAuthTx([6u8; 32]));
        let mut doc = valid();
        doc["transactions"][0]["operations"][0]["signer"]["key"] = json!(pre_auth);
        assert!(has(
            &issues(&doc),
            Code::Signer01,
            "/transactions/0/operations/0/signer/key"
        ));
    }

    #[test]
    fn a_signer_needs_both_key_and_weight() {
        let mut doc = valid();
        doc["transactions"][0]["operations"][0]["signer"] = json!({ "key": key(3) });
        assert!(has(
            &issues(&doc),
            Code::Input01,
            "/transactions/0/operations/0/signer/weight"
        ));
    }

    #[test]
    fn home_domain_is_limited_to_32_printable_ascii_characters() {
        let at = "/transactions/1/operations/0/home_domain";
        let mut doc = valid();
        doc["transactions"][1]["operations"][0]["home_domain"] = json!("example.org");
        assert!(parse(&doc).is_ok());
        doc["transactions"][1]["operations"][0]["home_domain"] = json!("a".repeat(32));
        assert!(parse(&doc).is_ok());
        for bad in [
            "a".repeat(33),
            "caf\u{e9}.org".to_string(),
            "tab\there".to_string(),
        ] {
            doc["transactions"][1]["operations"][0]["home_domain"] = json!(bad);
            assert!(has(&issues(&doc), Code::Input01, at));
        }
    }

    #[test]
    fn account_flags_must_exist_and_not_overlap() {
        let at = "/transactions/1/operations/0";
        let mut doc = valid();
        doc["transactions"][1]["operations"][0]["set_flags"] = json!(15);
        assert!(parse(&doc).is_ok());
        doc["transactions"][1]["operations"][0]["set_flags"] = json!(16);
        assert!(has(
            &issues(&doc),
            Code::Input01,
            &format!("{at}/set_flags")
        ));
        doc["transactions"][1]["operations"][0]["set_flags"] = json!(3);
        doc["transactions"][1]["operations"][0]["clear_flags"] = json!(2);
        assert!(has(&issues(&doc), Code::Input01, at));
        doc["transactions"][1]["operations"][0]["clear_flags"] = json!(4);
        assert!(parse(&doc).is_ok());
    }

    #[test]
    fn fee_bump_and_preconditions_are_tx_04() {
        for field in ["fee_bump", "preconditions"] {
            let mut doc = valid();
            doc["transactions"][0][field] = json!({});
            let location = format!("/transactions/0/{field}");
            assert!(has(&issues(&doc), Code::Tx04, &location), "{field}");
        }
    }

    #[test]
    fn declared_sources_are_kept_for_the_evaluation() {
        let mut doc = valid();
        doc["transactions"][0]["source"] = json!(key(8));
        doc["transactions"][0]["operations"][0]["source"] = json!(key(9));
        let plan = parse(&doc).expect("valid");
        let tx = &plan.transactions()[0];
        assert_eq!(tx.source(), Some(SignerKey::from_bytes([8; 32])));
        assert_eq!(
            tx.operations()[0].source(),
            Some(SignerKey::from_bytes([9; 32]))
        );
        assert_eq!(plan.transactions()[1].source(), None);
    }

    #[test]
    fn unknown_and_duplicate_fields_are_refused() {
        let mut doc = valid();
        doc["extra"] = json!(1);
        doc["transactions"][0]["note"] = json!("x");
        doc["transactions"][0]["operations"][0]["inflation_dest"] = json!(key(1));
        doc["transactions"][0]["operations"][0]["signer"]["label"] = json!("x");
        let found = issues(&doc);
        for location in [
            "/extra",
            "/transactions/0/note",
            "/transactions/0/operations/0/inflation_dest",
            "/transactions/0/operations/0/signer/label",
        ] {
            assert!(has(&found, Code::Input01, location), "{location}");
        }

        let text = r#"{"schema":"signerreach.plan/1","transactions":[
            {"id":"a","signed_by":[],"operations":[{"op":"set_options","high":1,"high":2}]}]}"#;
        let found = parse_plan(text).expect_err("duplicate key");
        assert!(has(
            &found,
            Code::Input01,
            "/transactions/0/operations/0/high"
        ));
    }

    #[test]
    fn values_from_the_document_are_never_repeated_in_messages() {
        let marker = "SECRETMARKER0123456789";
        let mut doc = valid();
        doc["transactions"][0]["id"] = json!(marker.to_string() + " bad id");
        doc["transactions"][0]["signed_by"] = json!([marker]);
        doc["transactions"][0]["operations"][0]["signer"]["key"] = json!(marker);
        doc["transactions"][0]["operations"][0]["signer"]["weight"] = json!(marker);
        doc["transactions"][1]["operations"][0]["home_domain"] = json!(marker.repeat(3));
        doc["transactions"][1]["operations"][0]["master_weight"] = json!(marker);
        doc["transactions"][1]["operations"][0]["source"] = json!(marker);
        let found = issues(&doc);
        assert!(found.len() >= 6);
        for issue in &found {
            assert!(!issue.to_string().contains(marker), "{issue}");
        }
    }

    #[test]
    fn text_that_is_not_json_is_refused() {
        assert!(parse_plan("{").is_err());
        assert!(parse_plan("[]").is_err());
        assert!(parse_plan("").is_err());
    }
}
