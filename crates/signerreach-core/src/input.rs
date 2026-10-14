//! Reading the input documents.
//!
//! All input is untrusted. Documents are read into a strict tree that keeps
//! duplicate keys, then validated field by field so every problem is reported
//! with its location and a diagnostic code.
//!
//! Messages never repeat a value taken from the document, except field names
//! and public account addresses. A document may contain something that must
//! not be echoed (for example a secret seed pasted into the wrong field).

pub mod availability;
pub mod plan;
pub mod snapshot;

use std::fmt;

use serde::de::{Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};

use stellar_strkey::Strkey;

use crate::account::{SignerKey, Threshold, Weight};
use crate::diagnostic::Code;

/// The largest document read, in bytes.
pub const MAX_INPUT_BYTES: usize = 1024 * 1024;

/// One problem found in an input document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputIssue {
    /// The diagnostic code.
    pub code: Code,
    /// A JSON Pointer to the offending value; empty for the whole document.
    pub location: String,
    /// What is wrong, without repeating the offending value.
    pub message: String,
}

impl fmt::Display for InputIssue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let location = if self.location.is_empty() {
            "(document)"
        } else {
            &self.location
        };
        write!(f, "{} at {}: {}", self.code, location, self.message)
    }
}

/// Collects issues while a document is validated.
#[derive(Debug, Default)]
pub(crate) struct Issues(Vec<InputIssue>);

impl Issues {
    pub(crate) fn push(&mut self, code: Code, location: &str, message: impl Into<String>) {
        self.0.push(InputIssue {
            code,
            location: location.to_string(),
            message: message.into(),
        });
    }

    pub(crate) fn input(&mut self, location: &str, message: impl Into<String>) {
        self.push(Code::Input01, location, message);
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// The value if nothing was reported, otherwise the issues.
    pub(crate) fn into_result<T>(self, value: Option<T>) -> Result<T, Vec<InputIssue>> {
        match (self.0.is_empty(), value) {
            (true, Some(value)) => Ok(value),
            (true, None) => Err(vec![InputIssue {
                code: Code::Input01,
                location: String::new(),
                message: "the document could not be read".to_string(),
            }]),
            (false, _) => Err(self.0),
        }
    }
}

/// A JSON value that keeps duplicate object keys, so they can be reported.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Json {
    Null,
    Bool(bool),
    Number(serde_json::Number),
    String(String),
    Array(Vec<Json>),
    Object(Vec<(String, Json)>),
}

impl<'de> Deserialize<'de> for Json {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(JsonVisitor)
    }
}

struct JsonVisitor;

impl<'de> Visitor<'de> for JsonVisitor {
    type Value = Json;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("any JSON value")
    }

    fn visit_bool<E>(self, value: bool) -> Result<Json, E> {
        Ok(Json::Bool(value))
    }

    fn visit_i64<E>(self, value: i64) -> Result<Json, E> {
        Ok(Json::Number(value.into()))
    }

    fn visit_u64<E>(self, value: u64) -> Result<Json, E> {
        Ok(Json::Number(value.into()))
    }

    fn visit_f64<E>(self, value: f64) -> Result<Json, E> {
        Ok(serde_json::Number::from_f64(value)
            .map(Json::Number)
            .unwrap_or(Json::Null))
    }

    fn visit_str<E>(self, value: &str) -> Result<Json, E> {
        Ok(Json::String(value.to_string()))
    }

    fn visit_string<E>(self, value: String) -> Result<Json, E> {
        Ok(Json::String(value))
    }

    fn visit_unit<E>(self) -> Result<Json, E> {
        Ok(Json::Null)
    }

    fn visit_none<E>(self) -> Result<Json, E> {
        Ok(Json::Null)
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Json, A::Error> {
        let mut items = Vec::new();
        while let Some(item) = seq.next_element()? {
            items.push(item);
        }
        Ok(Json::Array(items))
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Json, A::Error> {
        let mut pairs = Vec::new();
        while let Some((key, value)) = map.next_entry::<String, Json>()? {
            pairs.push((key, value));
        }
        Ok(Json::Object(pairs))
    }
}

/// Reads text into a [`Json`] tree, enforcing the size limit.
///
/// The parse error is deliberately reduced to a line and column: the library's
/// own message can quote the value it choked on.
pub(crate) fn read_document(text: &str, issues: &mut Issues) -> Option<Json> {
    if text.len() > MAX_INPUT_BYTES {
        issues.input("", "the document is larger than the 1 MiB limit");
        return None;
    }
    match serde_json::from_str::<Json>(text) {
        Ok(json) => Some(json),
        Err(error) => {
            issues.input(
                "",
                format!(
                    "not valid JSON at line {} column {}",
                    error.line(),
                    error.column()
                ),
            );
            None
        }
    }
}

/// Escapes one object key for use in a JSON Pointer.
fn escape(key: &str) -> String {
    key.replace('~', "~0").replace('/', "~1")
}

/// Appends a key to a JSON Pointer.
pub(crate) fn child(path: &str, key: &str) -> String {
    format!("{path}/{}", escape(key))
}

/// Appends an array index to a JSON Pointer.
pub(crate) fn item(path: &str, index: usize) -> String {
    format!("{path}/{index}")
}

/// A strict reader for one JSON object: duplicate and unknown keys are issues.
pub(crate) struct Obj<'a> {
    pairs: &'a [(String, Json)],
    path: String,
    seen: Vec<bool>,
}

impl<'a> Obj<'a> {
    /// Opens a value as an object, or reports that it is not one.
    pub(crate) fn open(value: &'a Json, path: &str, issues: &mut Issues) -> Option<Self> {
        match value {
            Json::Object(pairs) => Some(Self {
                pairs,
                path: path.to_string(),
                seen: vec![false; pairs.len()],
            }),
            _ => {
                issues.input(path, "expected an object");
                None
            }
        }
    }

    /// The pointer of a field of this object.
    pub(crate) fn path_of(&self, key: &str) -> String {
        child(&self.path, key)
    }

    /// A field, if present. A key that appears twice is reported.
    pub(crate) fn field(&mut self, key: &str, issues: &mut Issues) -> Option<&'a Json> {
        let mut found: Option<&'a Json> = None;
        for (index, (name, value)) in self.pairs.iter().enumerate() {
            if name == key {
                self.seen[index] = true;
                if found.is_some() {
                    issues.input(&self.path_of(key), "the key appears more than once");
                } else {
                    found = Some(value);
                }
            }
        }
        found
    }

    /// Reports every key that no `field` call asked for.
    pub(crate) fn finish(self, issues: &mut Issues) {
        for ((name, _), seen) in self.pairs.iter().zip(&self.seen) {
            if !seen {
                let shown: String = name.chars().take(40).collect();
                issues.input(&child(&self.path, name), format!("unknown field `{shown}`"));
            }
        }
    }
}

/// A string value, or an issue.
pub(crate) fn as_str<'a>(value: &'a Json, path: &str, issues: &mut Issues) -> Option<&'a str> {
    match value {
        Json::String(text) => Some(text),
        _ => {
            issues.input(path, "expected a string");
            None
        }
    }
}

/// A whole number that fits in 32 bits, or an issue.
pub(crate) fn as_u32(value: &Json, path: &str, issues: &mut Issues) -> Option<u32> {
    let number = match value {
        Json::Number(number) => number.as_u64().and_then(|whole| u32::try_from(whole).ok()),
        _ => None,
    };
    if number.is_none() {
        issues.input(path, "expected a non-negative whole number");
    }
    number
}

/// A boolean value, or an issue.
pub(crate) fn as_bool(value: &Json, path: &str, issues: &mut Issues) -> Option<bool> {
    match value {
        Json::Bool(flag) => Some(*flag),
        _ => {
            issues.input(path, "expected true or false");
            None
        }
    }
}

/// A required field; a missing one is an input issue.
pub(crate) fn required<'a>(obj: &mut Obj<'a>, key: &str, issues: &mut Issues) -> Option<&'a Json> {
    let found = obj.field(key, issues);
    if found.is_none() {
        issues.input(&obj.path_of(key), "missing required field");
    }
    found
}

/// A weight from 0 to 255, or an issue.
pub(crate) fn as_weight(value: &Json, path: &str, issues: &mut Issues) -> Option<Weight> {
    let number = as_u32(value, path, issues)?;
    match Weight::new(number) {
        Ok(weight) => Some(weight),
        Err(error) => {
            issues.input(path, error.to_string());
            None
        }
    }
}

/// A threshold from 0 to 255, or an issue.
pub(crate) fn as_threshold(value: &Json, path: &str, issues: &mut Issues) -> Option<Threshold> {
    let number = as_u32(value, path, issues)?;
    match Threshold::new(number) {
        Ok(threshold) => Some(threshold),
        Err(error) => {
            issues.input(path, error.to_string());
            None
        }
    }
}

/// The `schema` field: required, and equal to `expected`.
pub(crate) fn check_schema(root: &mut Obj<'_>, expected: &str, issues: &mut Issues) {
    let path = root.path_of("schema");
    let Some(value) = required(root, "schema", issues) else {
        return;
    };
    if as_str(value, &path, issues).is_some_and(|text| text != expected) {
        issues.input(&path, format!("the schema must be `{expected}`"));
    }
}

/// An ed25519 account key from text. Other key kinds are recognised and
/// reported as such. The text is never repeated in the message.
pub(crate) fn key_from_text(text: &str, path: &str, issues: &mut Issues) -> Option<SignerKey> {
    match SignerKey::from_strkey(text) {
        Ok(key) => Some(key),
        Err(_) => {
            match Strkey::from_string(text) {
                Ok(Strkey::PreAuthTx(_) | Strkey::HashX(_) | Strkey::SignedPayloadEd25519(_)) => {
                    issues.push(Code::Signer01, path, "only ed25519 keys are supported");
                }
                _ => issues.input(path, "not a valid G... address"),
            }
            None
        }
    }
}

/// An ed25519 account key from a JSON string, or an issue.
pub(crate) fn as_key(value: &Json, path: &str, issues: &mut Issues) -> Option<SignerKey> {
    let text = as_str(value, path, issues)?;
    key_from_text(text, path, issues)
}
