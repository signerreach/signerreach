//! Turning a Horizon account record into a `signerreach.snapshot/1` document.
//!
//! Horizon lists the account's own key among its signers, with the master
//! weight. The snapshot keeps the master weight separate, so the entry whose key
//! is the account itself becomes `master_weight` and every other entry becomes a
//! signer.
//!
//! The mapping is faithful, not selective. A signer of a kind the checker does
//! not model is written out with its kind, so that `signerreach check` reports it
//! (`SR-SIGNER-01`) instead of the fetch silently dropping it.
//!
//! Horizon's signer types are `ed25519_public_key`, `sha256_hash`,
//! `preauth_tx` and `ed25519_signed_payload`. The first three are in its
//! published account documentation. All four are in the `KeyTypeNames` table of
//! `protocols/horizon/main.go` in `stellar/go-stellar-sdk`, which was read on
//! 2026-10-20. A signed-payload signer has not been seen in a live response. A
//! type with any other name is refused.

use serde_json::{Value, json};
use signerreach_core::account::MAX_ADDITIONAL_SIGNERS;
use signerreach_core::{Network, Reserve, SignerKey};

use crate::error::MapError;
use crate::strict_json;

/// What a snapshot records about where its data came from.
#[derive(Debug, Clone, Copy)]
pub struct Context<'a> {
    /// The network the data was read from.
    pub network: Network,
    /// The ledger the state is read as of.
    pub as_of_ledger: u64,
    /// When the data was captured, as an RFC 3339 timestamp.
    pub captured_at: &'a str,
    /// The network's protocol version when the data was read.
    pub protocol_version: u32,
    /// The account address requested from Horizon, if this came from a fetch.
    pub requested_account: Option<&'a str>,
    /// The network's base reserve at `as_of_ledger`, in stroops. When it is
    /// given, the snapshot carries a `reserve` object built from the account's
    /// balance, selling liabilities and entry counts; when it is not, the
    /// snapshot has none.
    pub base_reserve: Option<i64>,
}

/// Builds the text of a snapshot document from a Horizon account response.
///
/// Fields Horizon sends that the snapshot does not need are ignored. The output
/// is indented, ends with a newline, and lists signers in ascending key order, so
/// the same record always gives the same bytes.
pub fn snapshot_from_account(account: &str, context: &Context<'_>) -> Result<String, MapError> {
    let record: Value = strict_json::parse(account)?;

    let id = text(&record, "id")?;
    SignerKey::from_strkey(id).map_err(|_| MapError::NotAnAccount)?;
    if context
        .requested_account
        .is_some_and(|requested| requested != id)
        || record
            .get("account_id")
            .is_some_and(|value| value.as_str() != Some(id))
    {
        return Err(MapError::AccountMismatch);
    }
    if let Some(modified) = record.get("last_modified_ledger") {
        let ledger = modified.as_u64().ok_or(MapError::BadLedger)?;
        if ledger == 0 || ledger > context.as_of_ledger {
            return Err(MapError::BadLedger);
        }
    }

    let thresholds = record
        .get("thresholds")
        .ok_or(MapError::MissingField("thresholds"))?;
    let low = byte(thresholds, "low_threshold", "thresholds.low_threshold")?;
    let medium = byte(thresholds, "med_threshold", "thresholds.med_threshold")?;
    let high = byte(thresholds, "high_threshold", "thresholds.high_threshold")?;

    let entries = record
        .get("signers")
        .ok_or(MapError::MissingField("signers"))?
        .as_array()
        .ok_or(MapError::BadField("signers"))?;

    let mut master_weight = None;
    let mut others: Vec<(String, u8, &'static str)> = Vec::new();
    let mut sponsored = false;

    for entry in entries {
        let key = text(entry, "key")?;
        let weight = byte(entry, "weight", "signers.weight")?;
        let kind = signer_kind(text(entry, "type")?)?;
        // A sponsor means someone else pays the reserve for this signer.
        sponsored |= entry
            .get("sponsor")
            .is_some_and(|sponsor| !sponsor.is_null());

        if key == id {
            if kind != "ed25519" {
                return Err(MapError::BadField("signers.type"));
            }
            if master_weight.replace(weight).is_some() {
                return Err(MapError::DuplicateMasterEntry);
            }
        } else {
            // `check` refuses an ed25519 signer whose key is not a valid address
            // or whose weight is 0 (a signer with no weight is not in the account
            // state), so the mapping must too: a snapshot is only ever written if
            // the checker will accept it. Other kinds keep their key as sent,
            // because the checker reports them by kind (SR-SIGNER-01) without
            // reading it.
            if kind == "ed25519" {
                if SignerKey::from_strkey(key).is_err() {
                    return Err(MapError::BadField("signers.key"));
                }
                if weight == 0 {
                    return Err(MapError::BadField("signers.weight"));
                }
            }
            others.push((key.to_string(), weight, kind));
        }
    }
    let master_weight = master_weight.ok_or(MapError::MissingMasterEntry)?;
    others.sort();

    // The same two rules the checker applies to the account state: no signer
    // listed twice, and at most `MAX_ADDITIONAL_SIGNERS` besides the account's own key.
    let ed25519 = || others.iter().filter(|(_, _, kind)| *kind == "ed25519");
    if ed25519().count() > MAX_ADDITIONAL_SIGNERS {
        return Err(MapError::BadField("signers"));
    }
    if ed25519()
        .zip(ed25519().skip(1))
        .any(|(first, second)| first.0 == second.0)
    {
        return Err(MapError::BadField("signers.key"));
    }

    let signers: Vec<Value> = others
        .iter()
        .map(|(key, weight, kind)| json!({ "key": key, "type": kind, "weight": weight }))
        .collect();

    let reserve = match context.base_reserve {
        Some(base_reserve) => Some(reserve_from(&record, base_reserve, others.len())?),
        None => None,
    };

    let mut snapshot = json!({
        "schema": "signerreach.snapshot/1",
        "network": context.network.as_str(),
        "account": id,
        "provenance": {
            "as_of_ledger": context.as_of_ledger,
            "captured_at": context.captured_at,
            "source": "horizon",
            "protocol_version": context.protocol_version,
        },
        "master_weight": master_weight,
        "thresholds": { "low": low, "medium": medium, "high": high },
        "signers": signers,
        "sponsored_signers": sponsored,
    });
    if let Some(reserve) = reserve {
        snapshot["reserve"] = reserve;
    }
    Ok(format!("{snapshot:#}\n"))
}

/// The `reserve` object: the native balance and selling liabilities (decimal
/// strings in XLM, turned into whole stroops without any floating point), the
/// entry counts, and the base reserve the caller read from the ledger.
///
/// The figures are checked by the same constructor the checker uses, so a
/// snapshot is only ever written if the checker will accept its `reserve`.
fn reserve_from(record: &Value, base_reserve: i64, signers: usize) -> Result<Value, MapError> {
    let balances = record
        .get("balances")
        .ok_or(MapError::MissingField("balances"))?
        .as_array()
        .ok_or(MapError::BadField("balances"))?;
    let mut native = balances
        .iter()
        .filter(|entry| entry.get("asset_type").and_then(Value::as_str) == Some("native"));
    let entry = native.next().ok_or(MapError::MissingField("balances"))?;
    if native.next().is_some() {
        return Err(MapError::BadField("balances"));
    }
    let balance = stroops(text(entry, "balance")?, "balances.balance")?;
    let selling_liabilities = stroops(
        text(entry, "selling_liabilities")?,
        "balances.selling_liabilities",
    )?;
    let subentry_count = count(record, "subentry_count")?;
    let num_sponsoring = count(record, "num_sponsoring")?;
    let num_sponsored = count(record, "num_sponsored")?;

    let reserve = Reserve::new(
        balance,
        selling_liabilities,
        subentry_count,
        num_sponsoring,
        num_sponsored,
        base_reserve,
    )
    .map_err(|_| MapError::BadField("reserve"))?;
    // Every signer, of any kind, is a sub-entry.
    reserve
        .check_signer_count(signers)
        .map_err(|_| MapError::BadField("subentry_count"))?;

    Ok(json!({
        "balance": balance,
        "selling_liabilities": selling_liabilities,
        "subentry_count": subentry_count,
        "num_sponsoring": num_sponsoring,
        "num_sponsored": num_sponsored,
        "base_reserve": base_reserve,
    }))
}

/// A whole number that fits in 32 bits, or an error naming the field.
fn count(value: &Value, field: &'static str) -> Result<u32, MapError> {
    value
        .get(field)
        .ok_or(MapError::MissingField(field))?
        .as_u64()
        .and_then(|number| u32::try_from(number).ok())
        .ok_or(MapError::BadField(field))
}

/// Horizon's amount text, such as `10791.0428944`, as whole stroops.
///
/// The shape is strict: digits, a point, exactly seven digits, no sign, no
/// exponent, no leading zero in the whole part. Anything else is refused
/// instead of being rounded.
fn stroops(amount: &str, field: &'static str) -> Result<i64, MapError> {
    let (whole, fraction) = amount.split_once('.').ok_or(MapError::BadField(field))?;
    let digits = |part: &str| !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit());
    if !digits(whole)
        || !digits(fraction)
        || fraction.len() != 7
        || whole.len() > 12
        || (whole.len() > 1 && whole.starts_with('0'))
    {
        return Err(MapError::BadField(field));
    }
    let whole: i64 = whole.parse().map_err(|_| MapError::BadField(field))?;
    let fraction: i64 = fraction.parse().map_err(|_| MapError::BadField(field))?;
    whole
        .checked_mul(10_000_000)
        .and_then(|stroops| stroops.checked_add(fraction))
        .ok_or(MapError::BadField(field))
}

/// A string field, or an error naming the field.
fn text<'a>(value: &'a Value, field: &'static str) -> Result<&'a str, MapError> {
    value
        .get(field)
        .ok_or(MapError::MissingField(field))?
        .as_str()
        .ok_or(MapError::BadField(field))
}

/// A whole number from 0 to 255, or an error naming the field.
fn byte(value: &Value, field: &str, described: &'static str) -> Result<u8, MapError> {
    value
        .get(field)
        .ok_or(MapError::MissingField(described))?
        .as_u64()
        .and_then(|number| u8::try_from(number).ok())
        .ok_or(MapError::BadField(described))
}

/// Horizon's name for a signer type, as the snapshot names it.
fn signer_kind(horizon: &str) -> Result<&'static str, MapError> {
    match horizon {
        "ed25519_public_key" => Ok("ed25519"),
        "preauth_tx" => Ok("pre_auth_tx"),
        "sha256_hash" => Ok("hash_x"),
        "ed25519_signed_payload" => Ok("ed25519_signed_payload"),
        _ => Err(MapError::UnknownSignerType),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::Value;
    use signerreach_core::{
        Code, Verdict, evaluate, parse_availability, parse_plan, parse_snapshot,
    };

    use super::*;

    const THREE_SIGNERS: &str = include_str!("../tests/data/horizon-testnet-three-signers.json");
    const SINGLE_KEY: &str = include_str!("../tests/data/horizon-testnet-single-key.json");
    const ZERO_MASTER: &str = include_str!("../tests/data/horizon-pubnet-zero-master.json");

    fn context(network: Network) -> Context<'static> {
        Context {
            network,
            as_of_ledger: 5_240_432,
            captured_at: "2026-10-17T08:29:07Z",
            protocol_version: 29,
            requested_account: None,
            base_reserve: None,
        }
    }

    fn map(account: &str) -> Result<String, MapError> {
        snapshot_from_account(account, &context(Network::Testnet))
    }

    fn json_of(text: &str) -> Value {
        serde_json::from_str(text).expect("valid JSON")
    }

    /// A minimal Horizon-shaped record for a given account and signer list.
    fn record(id: &str, signers: Value) -> String {
        json!({
            "id": id,
            "thresholds": { "low_threshold": 1, "med_threshold": 2, "high_threshold": 3 },
            "signers": signers,
            "something_else": { "ignored": true },
        })
        .to_string()
    }

    fn account_key(n: u8) -> String {
        SignerKey::from_bytes([n; 32]).to_string()
    }

    #[test]
    fn a_real_multisig_account_maps_to_a_snapshot_the_checker_accepts() {
        let text = map(THREE_SIGNERS).expect("maps");
        let snapshot = parse_snapshot(&text).expect("the checker accepts it");
        let state = snapshot.state();

        assert_eq!(snapshot.network(), Network::Testnet);
        assert_eq!(snapshot.provenance().as_of_ledger(), 5_240_432);
        assert_eq!(snapshot.provenance().captured_at(), "2026-10-17T08:29:07Z");
        assert_eq!(snapshot.provenance().protocol_version(), 29);
        assert_eq!(state.master_weight().get(), 1);
        assert_eq!(state.additional_signer_count(), 2);
        assert_eq!(
            state.account().to_string(),
            json_of(THREE_SIGNERS)["id"].as_str().unwrap()
        );
        use signerreach_core::ThresholdClass::{High, Low, Medium};
        assert_eq!(state.threshold(Low).get(), 1);
        assert_eq!(state.threshold(Medium).get(), 2);
        assert_eq!(state.threshold(High).get(), 2);
    }

    #[test]
    fn duplicate_account_fields_are_refused() {
        let text = format!(r#"{{"id":"{}",{}"#, account_key(1), &THREE_SIGNERS[1..]);
        assert!(map(&text).is_err());
    }

    #[test]
    fn the_master_entry_is_not_listed_as_an_additional_signer() {
        let value = json_of(&map(THREE_SIGNERS).expect("maps"));
        let id = value["account"].as_str().unwrap().to_string();
        let keys: Vec<&str> = value["signers"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s["key"].as_str().unwrap())
            .collect();
        assert_eq!(keys.len(), 2);
        assert!(!keys.contains(&id.as_str()));
        let mut sorted = keys.clone();
        sorted.sort_unstable();
        assert_eq!(keys, sorted, "signers are in ascending key order");
        assert!(value["signers"][0]["type"] == "ed25519");
    }

    #[test]
    fn a_single_key_account_has_no_additional_signers() {
        let text = map(SINGLE_KEY).expect("maps");
        let snapshot = parse_snapshot(&text).expect("accepted");
        assert_eq!(snapshot.state().master_weight().get(), 1);
        assert_eq!(snapshot.state().additional_signer_count(), 0);
        assert!(json_of(&text)["sponsored_signers"] == false);
    }

    #[test]
    fn a_real_locked_mainnet_account_is_reported_unsafe_end_to_end() {
        // Mainnet account whose master weight is 0 and which has no other signer.
        let context = Context {
            network: Network::Pubnet,
            as_of_ledger: 64_988_002,
            captured_at: "2026-10-17T08:29:55Z",
            protocol_version: 29,
            requested_account: None,
            base_reserve: None,
        };
        let text = snapshot_from_account(ZERO_MASTER, &context).expect("maps");
        let snapshot = parse_snapshot(&text).expect("accepted");
        assert_eq!(snapshot.network(), Network::Pubnet);
        assert_eq!(snapshot.state().master_weight().get(), 0);

        let plan = parse_plan(r#"{"schema":"signerreach.plan/1","transactions":[]}"#).unwrap();
        let availability =
            parse_availability(r#"{"schema":"signerreach.availability/1"}"#).unwrap();
        let result = evaluate(&snapshot, &plan, &availability);
        assert_eq!(result.verdict, Verdict::Unsafe);
        assert!(
            result
                .scenarios
                .iter()
                .flat_map(|s| s.findings.iter())
                .any(|f| f.code == Code::Lock01)
        );
    }

    #[test]
    fn the_same_record_gives_the_same_bytes() {
        assert_eq!(map(THREE_SIGNERS), map(THREE_SIGNERS));
        let text = map(THREE_SIGNERS).unwrap();
        assert!(text.ends_with("}\n"));
        assert!(text.is_ascii());
    }

    #[test]
    fn signer_types_are_renamed_and_kept_not_dropped() {
        let id = account_key(1);
        let signers = json!([
            { "key": id, "weight": 1, "type": "ed25519_public_key" },
            { "key": account_key(2), "weight": 1, "type": "ed25519_public_key" },
            { "key": "TAAAA", "weight": 1, "type": "preauth_tx" },
            { "key": "XAAAA", "weight": 1, "type": "sha256_hash" },
        ]);
        let value = json_of(&map(&record(&id, signers)).expect("maps"));
        let kinds: Vec<&str> = value["signers"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s["type"].as_str().unwrap())
            .collect();
        assert_eq!(kinds.len(), 3);
        assert!(kinds.contains(&"ed25519"));
        assert!(kinds.contains(&"pre_auth_tx"));
        assert!(kinds.contains(&"hash_x"));
    }

    #[test]
    fn a_signer_the_checker_does_not_model_is_reported_by_the_checker() {
        let id = account_key(1);
        let signers = json!([
            { "key": id, "weight": 1, "type": "ed25519_public_key" },
            { "key": "TAAAA", "weight": 1, "type": "preauth_tx" },
        ]);
        let text = map(&record(&id, signers)).expect("maps");
        let issues = parse_snapshot(&text).expect_err("not modelled");
        assert!(issues.iter().any(|i| i.code == Code::Signer01));
    }

    #[test]
    fn a_sponsored_signer_sets_the_flag_and_the_checker_reports_it() {
        let id = account_key(1);
        let signers = json!([
            { "key": id, "weight": 1, "type": "ed25519_public_key" },
            { "key": account_key(2), "weight": 1, "type": "ed25519_public_key",
              "sponsor": account_key(3) },
        ]);
        let text = map(&record(&id, signers)).expect("maps");
        assert!(json_of(&text)["sponsored_signers"] == true);
        let issues = parse_snapshot(&text).expect_err("sponsorship is not modelled");
        assert!(issues.iter().any(|i| i.code == Code::State01));
    }

    #[test]
    fn a_null_sponsor_is_not_a_sponsor() {
        let id = account_key(1);
        let signers = json!([
            { "key": id, "weight": 1, "type": "ed25519_public_key", "sponsor": null },
        ]);
        let text = map(&record(&id, signers)).expect("maps");
        assert!(json_of(&text)["sponsored_signers"] == false);
    }

    #[test]
    fn a_missing_master_entry_is_refused_not_read_as_weight_zero() {
        let id = account_key(1);
        let signers = json!([
            { "key": account_key(2), "weight": 1, "type": "ed25519_public_key" },
        ]);
        assert_eq!(
            map(&record(&id, signers)),
            Err(MapError::MissingMasterEntry)
        );
    }

    #[test]
    fn a_master_entry_listed_twice_is_refused() {
        let id = account_key(1);
        let entry = json!({ "key": id, "weight": 1, "type": "ed25519_public_key" });
        assert_eq!(
            map(&record(&id, json!([entry.clone(), entry]))),
            Err(MapError::DuplicateMasterEntry)
        );
    }

    #[test]
    fn a_master_entry_of_another_type_is_refused() {
        let id = account_key(1);
        let signers = json!([{ "key": id, "weight": 1, "type": "sha256_hash" }]);
        assert_eq!(
            map(&record(&id, signers)),
            Err(MapError::BadField("signers.type"))
        );
    }

    #[test]
    fn an_unknown_signer_type_is_refused() {
        let id = account_key(1);
        let signers = json!([
            { "key": id, "weight": 1, "type": "ed25519_public_key" },
            { "key": account_key(2), "weight": 1, "type": "quantum" },
        ]);
        assert_eq!(map(&record(&id, signers)), Err(MapError::UnknownSignerType));
    }

    #[test]
    fn numbers_outside_0_to_255_or_not_whole_are_refused() {
        let id = account_key(1);
        for bad in [json!(256), json!(-1), json!(1.5), json!("1"), json!(null)] {
            let signers = json!([{ "key": id, "weight": bad, "type": "ed25519_public_key" }]);
            assert_eq!(
                map(&record(&id, signers)),
                Err(MapError::BadField("signers.weight")),
                "{bad}"
            );
        }
        let mut value = json_of(&record(&id, json!([])));
        value["thresholds"]["high_threshold"] = json!(300);
        assert_eq!(
            map(&value.to_string()),
            Err(MapError::BadField("thresholds.high_threshold"))
        );
    }

    #[test]
    fn missing_pieces_are_named() {
        let id = account_key(1);
        let full = json_of(&record(
            &id,
            json!([{ "key": id, "weight": 1, "type": "ed25519_public_key" }]),
        ));
        for (field, expected) in [
            ("id", MapError::MissingField("id")),
            ("thresholds", MapError::MissingField("thresholds")),
            ("signers", MapError::MissingField("signers")),
        ] {
            let mut value = full.clone();
            value.as_object_mut().unwrap().remove(field);
            assert_eq!(map(&value.to_string()), Err(expected), "{field}");
        }
        let mut value = full.clone();
        value["thresholds"]
            .as_object_mut()
            .unwrap()
            .remove("med_threshold");
        assert_eq!(
            map(&value.to_string()),
            Err(MapError::MissingField("thresholds.med_threshold"))
        );
        let mut value = full;
        value["signers"][0].as_object_mut().unwrap().remove("type");
        assert_eq!(map(&value.to_string()), Err(MapError::MissingField("type")));
    }

    #[test]
    fn something_that_is_not_json_or_has_the_wrong_shape_is_refused() {
        assert_eq!(map("not json"), Err(MapError::NotJson));
        assert_eq!(map(""), Err(MapError::NotJson));
        assert_eq!(map("[]"), Err(MapError::MissingField("id")));
        let id = account_key(1);
        let mut value = json_of(&record(&id, json!([])));
        value["signers"] = json!({});
        assert_eq!(map(&value.to_string()), Err(MapError::BadField("signers")));
    }

    #[test]
    fn an_id_that_is_not_an_ed25519_account_is_refused() {
        // The last one is a well-formed contract address (C...), not an account.
        for id in [
            "",
            "nonsense",
            "CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABSC4",
        ] {
            assert_eq!(
                map(&record(id, json!([]))),
                Err(MapError::NotAnAccount),
                "{id}"
            );
        }
    }

    /// Found by fuzzing (`fuzz/fuzz_targets/horizon_mapping.rs`): the second key
    /// below is a valid-looking address whose checksum is wrong. The mapping used
    /// to copy it into the snapshot, and `check` then refused the snapshot.
    #[test]
    fn an_ed25519_signer_with_a_bad_checksum_is_refused_not_copied() {
        let id = "GDIIIH6U2TEPSP4ZQUVN25AERQQXBVJIQB52XD4TKR5IZBT4EIBXXGZO";
        let bad_checksum = "GC3UO3RS66QQTMAOIQJP4KFQN7PE3CUXPROX6I7XJREHBBBPQPHRKS7R";
        assert!(SignerKey::from_strkey(id).is_ok());
        assert!(SignerKey::from_strkey(bad_checksum).is_err());

        let signers = json!([
            { "key": id, "weight": 1, "type": "ed25519_public_key" },
            { "key": bad_checksum, "weight": 1, "type": "ed25519_public_key" },
        ]);
        let error = map(&record(id, signers)).expect_err("refused");
        assert_eq!(error, MapError::BadField("signers.key"));
        assert!(!error.to_string().contains(bad_checksum), "{error}");
    }

    /// Found by fuzzing: a signer with weight 0 was copied into the snapshot, and
    /// `check` then refused it ("a signer with weight 0 is not part of an account
    /// state"). The account's own key is different: master weight 0 is a real,
    /// valid state, and is still mapped.
    #[test]
    fn an_ed25519_signer_with_weight_zero_is_refused_but_a_zero_master_is_not() {
        let id = account_key(1);
        let signers = json!([
            { "key": id, "weight": 1, "type": "ed25519_public_key" },
            { "key": account_key(2), "weight": 0, "type": "ed25519_public_key" },
        ]);
        assert_eq!(
            map(&record(&id, signers)),
            Err(MapError::BadField("signers.weight"))
        );

        let zero_master = json!([
            { "key": id, "weight": 0, "type": "ed25519_public_key" },
            { "key": account_key(2), "weight": 1, "type": "ed25519_public_key" },
        ]);
        let text = map(&record(&id, zero_master)).expect("a zero master weight maps");
        assert!(parse_snapshot(&text).is_ok());
    }

    #[test]
    fn a_signer_listed_twice_is_refused() {
        let id = account_key(1);
        let signers = json!([
            { "key": id, "weight": 1, "type": "ed25519_public_key" },
            { "key": account_key(2), "weight": 1, "type": "ed25519_public_key" },
            { "key": account_key(2), "weight": 2, "type": "ed25519_public_key" },
        ]);
        assert_eq!(
            map(&record(&id, signers)),
            Err(MapError::BadField("signers.key"))
        );
    }

    #[test]
    fn twenty_additional_signers_map_and_twenty_one_are_refused() {
        let id = account_key(1);
        let with = |extra: u8| {
            let mut signers = vec![json!({ "key": id, "weight": 1, "type": "ed25519_public_key" })];
            for n in 0..extra {
                signers.push(
                    json!({ "key": account_key(n + 2), "weight": 1, "type": "ed25519_public_key" }),
                );
            }
            record(&id, json!(signers))
        };
        let text = map(&with(20)).expect("20 additional signers map");
        assert!(parse_snapshot(&text).is_ok());
        assert_eq!(map(&with(21)), Err(MapError::BadField("signers")));
    }

    #[test]
    fn a_signer_the_checker_does_not_read_keeps_its_key_as_sent() {
        let id = account_key(1);
        let signers = json!([
            { "key": id, "weight": 1, "type": "ed25519_public_key" },
            { "key": "not an address", "weight": 1, "type": "sha256_hash" },
        ]);
        let text = map(&record(&id, signers)).expect("maps");
        assert_eq!(json_of(&text)["signers"][0]["key"], "not an address");
    }

    #[test]
    fn error_messages_never_repeat_what_the_server_sent() {
        let marker = "SECRETMARKER0123456789";
        let mut value = json_of(&record(marker, json!([])));
        let first = map(&value.to_string()).expect_err("not an account");
        value["id"] = json!(account_key(1));
        value["signers"] = json!([{ "key": marker, "weight": 1, "type": marker }]);
        let second = map(&value.to_string()).expect_err("unknown type");
        for error in [first, second] {
            assert!(!error.to_string().contains(marker), "{error}");
        }
    }

    #[test]
    fn extra_fields_in_the_response_are_ignored() {
        let id = account_key(1);
        let mut value = json_of(&record(
            &id,
            json!([{ "key": id, "weight": 1, "type": "ed25519_public_key", "future": 1 }]),
        ));
        value["a_new_field"] = json!([1, 2, 3]);
        assert!(map(&value.to_string()).is_ok());
    }

    // ---- the `reserve` object ----

    fn funded(network: Network) -> Context<'static> {
        Context {
            base_reserve: Some(5_000_000),
            ..context(network)
        }
    }

    fn map_funded(account: &str) -> Result<String, MapError> {
        snapshot_from_account(account, &funded(Network::Testnet))
    }

    #[test]
    fn without_a_base_reserve_the_snapshot_has_no_reserve_object() {
        let text = map(THREE_SIGNERS).expect("maps");
        assert!(json_of(&text).get("reserve").is_none());
        assert!(parse_snapshot(&text).unwrap().reserve().is_none());
    }

    #[test]
    fn with_a_base_reserve_the_balance_is_written_in_whole_stroops() {
        let text = map_funded(THREE_SIGNERS).expect("maps");
        assert_eq!(
            json_of(&text)["reserve"],
            json!({
                "balance": 99_999_999_400u64,
                "selling_liabilities": 0,
                "subentry_count": 2,
                "num_sponsoring": 0,
                "num_sponsored": 0,
                "base_reserve": 5_000_000
            })
        );
        let reserve = parse_snapshot(&text).expect("accepted").reserve().unwrap();
        assert_eq!(reserve.available(), 99_999_999_400 - 20_000_000);
    }

    #[test]
    fn every_recorded_account_maps_to_a_snapshot_with_reserve_the_checker_accepts() {
        // A ledger later than every record was last modified.
        let context = Context {
            as_of_ledger: 100_000_000,
            ..funded(Network::Testnet)
        };
        for record in [THREE_SIGNERS, SINGLE_KEY, ZERO_MASTER] {
            let text = snapshot_from_account(record, &context).expect("maps");
            assert!(parse_snapshot(&text).expect("accepted").reserve().is_some());
        }
    }

    #[test]
    fn selling_liabilities_and_sponsorship_counts_are_carried() {
        let mut record = json_of(THREE_SIGNERS);
        record["balances"][0]["selling_liabilities"] = json!("12.5000001");
        record["num_sponsoring"] = json!(3);
        record["num_sponsored"] = json!(1);
        let text = map_funded(&record.to_string()).expect("maps");
        let reserve = parse_snapshot(&text).unwrap().reserve().unwrap();
        assert_eq!(reserve.selling_liabilities(), 125_000_001);
        assert_eq!((reserve.num_sponsoring(), reserve.num_sponsored()), (3, 1));
    }

    #[test]
    fn amounts_are_read_exactly() {
        for (text, expected) in [
            ("0.0000000", 0),
            ("0.0000001", 1),
            ("1.0000000", 10_000_000),
            ("10791.0428944", 107_910_428_944),
            ("922337203685.4775807", i64::MAX),
        ] {
            assert_eq!(stroops(text, "x"), Ok(expected), "{text}");
        }
    }

    #[test]
    fn amounts_in_any_other_shape_are_refused() {
        for bad in [
            "",
            ".5000000",
            "1.",
            "1",
            "1.000000",
            "1.00000000",
            "01.0000000",
            "-1.0000000",
            "+1.0000000",
            "1e3.0000000",
            "1,0000000",
            " 1.0000000",
            "1.0000000 ",
            "1.00000a0",
            "922337203685.4775808",
            "9999999999999.0000000",
            "٣.0000000",
        ] {
            assert_eq!(stroops(bad, "x"), Err(MapError::BadField("x")), "{bad:?}");
        }
    }

    #[test]
    fn an_account_the_reserve_cannot_describe_is_refused_not_guessed() {
        let mutate = |change: &dyn Fn(&mut Value)| {
            let mut record = json_of(THREE_SIGNERS);
            change(&mut record);
            map_funded(&record.to_string())
        };
        assert_eq!(
            mutate(&|r| {
                r.as_object_mut().unwrap().remove("balances");
            }),
            Err(MapError::MissingField("balances"))
        );
        assert_eq!(
            mutate(&|r| r["balances"][0]["asset_type"] = json!("credit_alphanum4")),
            Err(MapError::MissingField("balances"))
        );
        assert_eq!(
            mutate(&|r| {
                let first = r["balances"][0].clone();
                r["balances"].as_array_mut().unwrap().push(first);
            }),
            Err(MapError::BadField("balances"))
        );
        assert_eq!(
            mutate(&|r| r["balances"][0]["balance"] = json!(10)),
            Err(MapError::BadField("balance"))
        );
        assert_eq!(
            mutate(&|r| r["balances"][0]["balance"] = json!("1")),
            Err(MapError::BadField("balances.balance"))
        );
        assert_eq!(
            mutate(&|r| {
                r["balances"][0]["selling_liabilities"] = json!("999999.0000000");
            }),
            Err(MapError::BadField("reserve"))
        );
        assert_eq!(
            mutate(&|r| r["subentry_count"] = json!(1)),
            Err(MapError::BadField("subentry_count"))
        );
        assert_eq!(
            mutate(&|r| r["subentry_count"] = json!(1001)),
            Err(MapError::BadField("reserve"))
        );
        assert_eq!(
            mutate(&|r| r["num_sponsored"] = json!(99)),
            Err(MapError::BadField("reserve"))
        );
        assert_eq!(
            mutate(&|r| {
                r.as_object_mut().unwrap().remove("num_sponsoring");
            }),
            Err(MapError::MissingField("num_sponsoring"))
        );
    }

    #[test]
    fn a_base_reserve_the_checker_would_refuse_is_refused_here_first() {
        for base_reserve in [0, -1] {
            let context = Context {
                base_reserve: Some(base_reserve),
                ..context(Network::Testnet)
            };
            assert_eq!(
                snapshot_from_account(THREE_SIGNERS, &context),
                Err(MapError::BadField("reserve"))
            );
        }
    }

    #[test]
    fn the_figures_are_checked_for_signers_of_every_kind() {
        // A pre-auth signer is a sub-entry too, so one sub-entry cannot cover
        // the account's own two signers plus it.
        let mut record = json_of(THREE_SIGNERS);
        record["signers"].as_array_mut().unwrap().push(json!({
            "weight": 1,
            "key": "TBU2RRGLXH3E5CQHTD3ODLDF2BWDCYUSSBLLZ5GNW7JXHDIYKXZWHXL7",
            "type": "preauth_tx"
        }));
        record["subentry_count"] = json!(2);
        assert_eq!(
            map_funded(&record.to_string()),
            Err(MapError::BadField("subentry_count"))
        );
        record["subentry_count"] = json!(3);
        assert!(map_funded(&record.to_string()).is_ok());
    }
}
