//! Reject ambiguous JSON before interpreting a Horizon response.

use std::collections::BTreeSet;

use serde::de::{Deserialize, Deserializer, Error, MapAccess, SeqAccess, Visitor};
use serde_json::Value;

use crate::error::MapError;

pub(crate) fn parse(text: &str) -> Result<Value, MapError> {
    serde_json::from_str::<NoDuplicates>(text).map_err(|_| MapError::NotJson)?;
    serde_json::from_str(text).map_err(|_| MapError::NotJson)
}

struct NoDuplicates;

impl<'de> Deserialize<'de> for NoDuplicates {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(UniqueVisitor)
    }
}

struct UniqueVisitor;

impl<'de> Visitor<'de> for UniqueVisitor {
    type Value = NoDuplicates;

    fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("JSON without duplicate object keys")
    }

    fn visit_bool<E: Error>(self, _: bool) -> Result<Self::Value, E> {
        Ok(NoDuplicates)
    }

    fn visit_i64<E: Error>(self, _: i64) -> Result<Self::Value, E> {
        Ok(NoDuplicates)
    }

    fn visit_u64<E: Error>(self, _: u64) -> Result<Self::Value, E> {
        Ok(NoDuplicates)
    }

    fn visit_f64<E: Error>(self, _: f64) -> Result<Self::Value, E> {
        Ok(NoDuplicates)
    }

    fn visit_str<E: Error>(self, _: &str) -> Result<Self::Value, E> {
        Ok(NoDuplicates)
    }

    fn visit_string<E: Error>(self, _: String) -> Result<Self::Value, E> {
        Ok(NoDuplicates)
    }

    fn visit_unit<E: Error>(self) -> Result<Self::Value, E> {
        Ok(NoDuplicates)
    }

    fn visit_none<E: Error>(self) -> Result<Self::Value, E> {
        Ok(NoDuplicates)
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        while seq.next_element::<NoDuplicates>()?.is_some() {}
        Ok(NoDuplicates)
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let mut seen = BTreeSet::new();
        while let Some((key, _)) = map.next_entry::<String, NoDuplicates>()? {
            if !seen.insert(key) {
                return Err(A::Error::custom("duplicate object key"));
            }
        }
        Ok(NoDuplicates)
    }
}
