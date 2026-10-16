//! The availability document, `signerreach.availability/1`.
//!
//! See `docs/SPEC.md`, section 5.3. Availability is something the user
//! declares. Nothing in this tool can check that anyone holds a key.

use std::collections::{BTreeMap, BTreeSet};

use super::{
    InputIssue, Issues, Json, Obj, as_str, as_u32, check_schema, child, item, key_from_text,
    read_document,
};
use crate::account::SignerKey;

/// The schema identifier an availability document must carry.
pub const AVAILABILITY_SCHEMA: &str = "signerreach.availability/1";

/// The most named scenarios one document may hold. A limit of this tool.
pub const MAX_SCENARIOS: usize = 100;

/// The longest scenario name.
const MAX_NAME_LENGTH: usize = 64;

/// What the user declares about one signer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Status {
    /// The signer's key can be used.
    Available,
    /// The signer's key cannot be used.
    Lost,
    /// Nobody has said.
    Unknown,
}

impl Status {
    /// The name used in documents and reports.
    pub const fn as_str(self) -> &'static str {
        match self {
            Status::Available => "available",
            Status::Lost => "lost",
            Status::Unknown => "unknown",
        }
    }

    fn parse(text: &str) -> Option<Self> {
        match text {
            "available" => Some(Status::Available),
            "lost" => Some(Status::Lost),
            "unknown" => Some(Status::Unknown),
            _ => None,
        }
    }
}

/// Whether the plan is meant to be recoverable.
///
/// This changes how a result is presented, never the result itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intent {
    /// Administrative access is meant to remain (the default).
    Recoverable,
    /// The plan deliberately removes administrative access.
    Irreversible,
}

impl Intent {
    /// The name used in documents and reports.
    pub const fn as_str(self) -> &'static str {
        match self {
            Intent::Recoverable => "recoverable",
            Intent::Irreversible => "irreversible",
        }
    }
}

/// A named set of overrides applied on top of the baseline declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scenario {
    name: String,
    overrides: BTreeMap<SignerKey, Status>,
}

impl Scenario {
    /// The scenario's name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The overrides, in ascending key order.
    pub fn overrides(&self) -> impl Iterator<Item = (SignerKey, Status)> + '_ {
        self.overrides.iter().map(|(key, status)| (*key, *status))
    }
}

/// A validated availability declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Availability {
    default: Status,
    signers: BTreeMap<SignerKey, Status>,
    scenarios: Vec<Scenario>,
    require_survive_loss_of: u8,
    intent: Intent,
}

impl Availability {
    /// The status of a signer that is not listed.
    pub const fn default_status(&self) -> Status {
        self.default
    }

    /// The named scenarios, in the order given.
    pub fn scenarios(&self) -> &[Scenario] {
        &self.scenarios
    }

    /// How many available signers must be survivable: 0 or 1.
    pub const fn require_survive_loss_of(&self) -> u8 {
        self.require_survive_loss_of
    }

    /// The declared intent.
    pub const fn intent(&self) -> Intent {
        self.intent
    }

    /// The baseline status of a key: its own declaration, or the default.
    pub fn status(&self, key: SignerKey) -> Status {
        self.signers.get(&key).copied().unwrap_or(self.default)
    }

    /// The signers with a declared status, in ascending key order.
    pub fn signers(&self) -> impl Iterator<Item = (SignerKey, Status)> + '_ {
        self.signers.iter().map(|(key, status)| (*key, *status))
    }

    /// The status of a key in a scenario: the scenario's override, else the
    /// baseline.
    pub fn status_in(&self, scenario: &Scenario, key: SignerKey) -> Status {
        scenario
            .overrides
            .get(&key)
            .copied()
            .unwrap_or_else(|| self.status(key))
    }
}

/// Reads and validates an availability document.
///
/// Returns every problem found, not only the first.
pub fn parse_availability(text: &str) -> Result<Availability, Vec<InputIssue>> {
    let mut issues = Issues::default();
    let availability = read_availability(text, &mut issues);
    issues.into_result(availability)
}

fn read_availability(text: &str, issues: &mut Issues) -> Option<Availability> {
    let json = read_document(text, issues)?;
    let mut root = Obj::open(&json, "", issues)?;

    check_schema(&mut root, AVAILABILITY_SCHEMA, issues);
    let default = read_default(&mut root, issues);
    let signers = read_signers(&mut root, issues);
    let scenarios = read_scenarios(&mut root, issues);
    let require_survive_loss_of = read_survive(&mut root, issues);
    let intent = read_intent(&mut root, issues);
    root.finish(issues);

    Some(Availability {
        default,
        signers,
        scenarios,
        require_survive_loss_of,
        intent,
    })
}

fn as_status(value: &Json, path: &str, issues: &mut Issues) -> Option<Status> {
    let text = as_str(value, path, issues)?;
    let status = Status::parse(text);
    if status.is_none() {
        issues.input(path, "the status must be `available`, `lost` or `unknown`");
    }
    status
}

fn read_default(root: &mut Obj<'_>, issues: &mut Issues) -> Status {
    let path = root.path_of("default");
    match root.field("default", issues) {
        Some(value) => as_status(value, &path, issues).unwrap_or(Status::Unknown),
        None => Status::Unknown,
    }
}

fn read_signers(root: &mut Obj<'_>, issues: &mut Issues) -> BTreeMap<SignerKey, Status> {
    let path = root.path_of("signers");
    match root.field("signers", issues) {
        Some(value) => read_status_map(value, &path, issues),
        None => BTreeMap::new(),
    }
}

/// An object of `G...` address to status.
///
/// An entry whose key is not an address is reported against the object, by
/// position, so that the offending text is not repeated.
fn read_status_map(value: &Json, path: &str, issues: &mut Issues) -> BTreeMap<SignerKey, Status> {
    let mut map = BTreeMap::new();
    let Json::Object(pairs) = value else {
        issues.input(path, "expected an object");
        return map;
    };

    let mut seen = BTreeSet::new();
    for (index, (name, status)) in pairs.iter().enumerate() {
        let mut local = Issues::default();
        let key = key_from_text(name, path, &mut local);
        for mut issue in local.0 {
            issue.message = format!("entry {index}: {}", issue.message);
            issues.0.push(issue);
        }
        let Some(key) = key else {
            continue;
        };

        let entry_path = child(path, name);
        if !seen.insert(key) {
            issues.input(&entry_path, "the key appears more than once");
            continue;
        }
        if let Some(status) = as_status(status, &entry_path, issues) {
            map.insert(key, status);
        }
    }
    map
}

fn read_scenarios(root: &mut Obj<'_>, issues: &mut Issues) -> Vec<Scenario> {
    let path = root.path_of("scenarios");
    let Some(value) = root.field("scenarios", issues) else {
        return Vec::new();
    };
    let Json::Array(entries) = value else {
        issues.input(&path, "expected an array");
        return Vec::new();
    };
    if entries.len() > MAX_SCENARIOS {
        issues.input(
            &path,
            format!("at most {MAX_SCENARIOS} scenarios are supported"),
        );
        return Vec::new();
    }

    let mut names = BTreeSet::new();
    let mut scenarios = Vec::new();
    for (index, entry) in entries.iter().enumerate() {
        let entry_path = item(&path, index);
        let Some(mut obj) = Obj::open(entry, &entry_path, issues) else {
            continue;
        };

        let name = read_name(&mut obj, &mut names, issues);

        let overrides_path = obj.path_of("overrides");
        let overrides = match obj.field("overrides", issues) {
            Some(value) => Some(read_status_map(value, &overrides_path, issues)),
            None => {
                issues.input(&overrides_path, "missing required field");
                None
            }
        };
        obj.finish(issues);

        if let (Some(name), Some(overrides)) = (name, overrides) {
            scenarios.push(Scenario { name, overrides });
        }
    }
    scenarios
}

fn read_name(
    obj: &mut Obj<'_>,
    names: &mut BTreeSet<String>,
    issues: &mut Issues,
) -> Option<String> {
    let path = obj.path_of("name");
    let Some(value) = obj.field("name", issues) else {
        issues.input(&path, "missing required field");
        return None;
    };
    let text = as_str(value, &path, issues)?;
    let well_formed = !text.is_empty()
        && text.len() <= MAX_NAME_LENGTH
        && text
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'));
    if !well_formed {
        issues.input(
            &path,
            format!(
                "a name is 1 to {MAX_NAME_LENGTH} characters from letters, digits, `.`, `_` and `-`"
            ),
        );
        return None;
    }
    if !names.insert(text.to_string()) {
        issues.input(&path, "the name is used by more than one scenario");
        return None;
    }
    Some(text.to_string())
}

fn read_survive(root: &mut Obj<'_>, issues: &mut Issues) -> u8 {
    let path = root.path_of("require_survive_loss_of");
    let Some(value) = root.field("require_survive_loss_of", issues) else {
        return 0;
    };
    match as_u32(value, &path, issues) {
        Some(0) => 0,
        Some(1) => 1,
        Some(_) => {
            issues.input(&path, "only 0 or 1 is supported");
            0
        }
        None => 0,
    }
}

fn read_intent(root: &mut Obj<'_>, issues: &mut Issues) -> Intent {
    let path = root.path_of("intent");
    let Some(value) = root.field("intent", issues) else {
        return Intent::Recoverable;
    };
    match as_str(value, &path, issues) {
        Some("recoverable") => Intent::Recoverable,
        Some("irreversible") => Intent::Irreversible,
        Some(_) => {
            issues.input(&path, "the intent must be `recoverable` or `irreversible`");
            Intent::Recoverable
        }
        None => Intent::Recoverable,
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::*;
    use crate::diagnostic::Code;

    fn k(n: u8) -> SignerKey {
        SignerKey::from_bytes([n; 32])
    }

    fn key(n: u8) -> String {
        k(n).to_string()
    }

    fn valid() -> Value {
        json!({
            "schema": "signerreach.availability/1",
            "default": "unknown",
            "signers": { key(1): "available", key(2): "lost" },
            "scenarios": [
                { "name": "A-lost", "overrides": { key(1): "lost" } },
                { "name": "B_back", "overrides": { key(2): "available" } }
            ],
            "require_survive_loss_of": 1,
            "intent": "recoverable"
        })
    }

    fn parse(value: &Value) -> Result<Availability, Vec<InputIssue>> {
        parse_availability(&value.to_string())
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
    fn accepts_a_valid_document() {
        let a = parse(&valid()).expect("valid");
        assert_eq!(a.default_status(), Status::Unknown);
        assert_eq!(a.status(k(1)), Status::Available);
        assert_eq!(a.status(k(2)), Status::Lost);
        assert_eq!(a.status(k(9)), Status::Unknown);
        assert_eq!(a.scenarios().len(), 2);
        assert_eq!(a.scenarios()[0].name(), "A-lost");
        assert_eq!(a.require_survive_loss_of(), 1);
        assert_eq!(a.intent(), Intent::Recoverable);
    }

    #[test]
    fn only_the_schema_is_required_and_the_rest_defaults() {
        let doc = json!({ "schema": "signerreach.availability/1" });
        let a = parse(&doc).expect("valid");
        assert_eq!(a.default_status(), Status::Unknown);
        assert_eq!(a.status(k(1)), Status::Unknown);
        assert!(a.scenarios().is_empty());
        assert_eq!(a.require_survive_loss_of(), 0);
        assert_eq!(a.intent(), Intent::Recoverable);
    }

    #[test]
    fn a_scenario_overrides_the_baseline_which_overrides_the_default() {
        let mut doc = valid();
        doc["default"] = json!("lost");
        let a = parse(&doc).expect("valid");
        let scenario = &a.scenarios()[0];
        assert_eq!(a.status(k(1)), Status::Available);
        assert_eq!(a.status_in(scenario, k(1)), Status::Lost);
        assert_eq!(a.status_in(scenario, k(2)), Status::Lost);
        assert_eq!(a.status(k(9)), Status::Lost);
        assert_eq!(a.status_in(scenario, k(9)), Status::Lost);
        let back = &a.scenarios()[1];
        assert_eq!(a.status_in(back, k(2)), Status::Available);
    }

    #[test]
    fn a_wrong_or_missing_schema_is_refused() {
        let mut doc = valid();
        doc["schema"] = json!("signerreach.plan/1");
        assert!(has(&issues(&doc), Code::Input01, "/schema"));
        doc.as_object_mut().unwrap().remove("schema");
        assert!(has(&issues(&doc), Code::Input01, "/schema"));
    }

    #[test]
    fn statuses_must_be_known_words() {
        let mut doc = valid();
        doc["default"] = json!("maybe");
        assert!(has(&issues(&doc), Code::Input01, "/default"));
        let mut doc = valid();
        doc["signers"][key(1)] = json!(true);
        assert!(has(
            &issues(&doc),
            Code::Input01,
            &format!("/signers/{}", key(1))
        ));
    }

    #[test]
    fn signer_keys_must_be_g_addresses() {
        let hash_x = format!("{}", stellar_strkey::HashX([5u8; 32]));
        let mut doc = valid();
        doc["signers"][hash_x] = json!("lost");
        assert!(has(&issues(&doc), Code::Signer01, "/signers"));
        let mut doc = valid();
        doc["signers"]["not a key"] = json!("lost");
        assert!(has(&issues(&doc), Code::Input01, "/signers"));
    }

    #[test]
    fn a_duplicate_signer_entry_is_refused() {
        let text = format!(
            r#"{{"schema":"signerreach.availability/1","signers":{{"{0}":"available","{0}":"lost"}}}}"#,
            key(1)
        );
        let found = parse_availability(&text).expect_err("duplicate");
        assert!(has(&found, Code::Input01, &format!("/signers/{}", key(1))));
    }

    #[test]
    fn scenarios_need_a_unique_well_formed_name_and_overrides() {
        for bad in ["", "has space", &"a".repeat(65)] {
            let mut doc = valid();
            doc["scenarios"][0]["name"] = json!(bad);
            assert!(
                has(&issues(&doc), Code::Input01, "/scenarios/0/name"),
                "{bad:?}"
            );
        }
        let mut doc = valid();
        doc["scenarios"][1]["name"] = json!("A-lost");
        assert!(has(&issues(&doc), Code::Input01, "/scenarios/1/name"));

        let mut doc = valid();
        doc["scenarios"][0]
            .as_object_mut()
            .unwrap()
            .remove("overrides");
        assert!(has(&issues(&doc), Code::Input01, "/scenarios/0/overrides"));
        let mut doc = valid();
        doc["scenarios"][0].as_object_mut().unwrap().remove("name");
        assert!(has(&issues(&doc), Code::Input01, "/scenarios/0/name"));
    }

    #[test]
    fn a_scenario_with_no_overrides_is_accepted() {
        let mut doc = valid();
        doc["scenarios"][0]["overrides"] = json!({});
        assert!(parse(&doc).is_ok());
    }

    #[test]
    fn at_most_100_scenarios_are_accepted() {
        let make = |count: usize| -> Value {
            let many: Vec<Value> = (0..count)
                .map(|n| json!({ "name": format!("s{n}"), "overrides": {} }))
                .collect();
            json!({ "schema": "signerreach.availability/1", "scenarios": many })
        };
        assert!(parse(&make(100)).is_ok());
        assert!(has(&issues(&make(101)), Code::Input01, "/scenarios"));
    }

    #[test]
    fn survive_loss_is_0_or_1() {
        for good in [0, 1] {
            let mut doc = valid();
            doc["require_survive_loss_of"] = json!(good);
            assert!(parse(&doc).is_ok(), "{good}");
        }
        for bad in [json!(2), json!(-1), json!("1"), json!(0.5)] {
            let mut doc = valid();
            doc["require_survive_loss_of"] = bad.clone();
            assert!(
                has(&issues(&doc), Code::Input01, "/require_survive_loss_of"),
                "{bad}"
            );
        }
    }

    #[test]
    fn intent_is_recoverable_or_irreversible() {
        let mut doc = valid();
        doc["intent"] = json!("irreversible");
        assert_eq!(parse(&doc).expect("valid").intent(), Intent::Irreversible);
        doc["intent"] = json!("whatever");
        assert!(has(&issues(&doc), Code::Input01, "/intent"));
    }

    #[test]
    fn unknown_fields_are_refused() {
        let mut doc = valid();
        doc["extra"] = json!(1);
        doc["scenarios"][0]["note"] = json!("x");
        let found = issues(&doc);
        assert!(has(&found, Code::Input01, "/extra"));
        assert!(has(&found, Code::Input01, "/scenarios/0/note"));
    }

    #[test]
    fn values_from_the_document_are_never_repeated_in_messages() {
        let marker = "SECRETMARKER0123456789";
        let mut doc = valid();
        doc["default"] = json!(marker);
        doc["signers"][marker] = json!("lost");
        doc["signers"][key(1)] = json!(marker);
        doc["scenarios"][0]["name"] = json!(marker.to_string() + " bad name");
        doc["scenarios"][1]["overrides"][marker] = json!("lost");
        doc["require_survive_loss_of"] = json!(marker);
        doc["intent"] = json!(marker);
        let found = issues(&doc);
        assert!(found.len() >= 6);
        for issue in &found {
            assert!(!issue.to_string().contains(marker), "{issue}");
        }
    }

    #[test]
    fn text_that_is_not_json_is_refused() {
        assert!(parse_availability("{").is_err());
        assert!(parse_availability("[]").is_err());
        assert!(parse_availability("").is_err());
    }
}
