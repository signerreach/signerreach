//! Which threshold an operation or an envelope needs.
//!
//! Rules R4 and R5 of `docs/SPEC.md`, section 4. They were read in
//! stellar-core (`SetOptionsOpFrame::getThresholdLevel`,
//! `TransactionFrame::checkAllTransactionSignatures`) and observed on Testnet;
//! see `docs/evidence/testnet-2026-10-13/`.

use crate::account::ThresholdClass;
use crate::input::plan::SetOptions;

/// The threshold the transaction envelope itself needs: the source account's
/// LOW threshold (R5).
pub const ENVELOPE_CLASS: ThresholdClass = ThresholdClass::Low;

/// Whether a `SetOptions` operation changes the master weight, a threshold or
/// a signer.
///
/// The test is whether the field is present, not whether its value differs
/// from the current one: the network decides by presence.
pub fn is_administrative(operation: &SetOptions) -> bool {
    operation.master_weight().is_some()
        || operation.low().is_some()
        || operation.medium().is_some()
        || operation.high().is_some()
        || operation.signer().is_some()
}

/// The threshold class a `SetOptions` operation needs (R4).
///
/// HIGH when it sets the master weight, a threshold or a signer; MEDIUM when it
/// sets none of those, including an operation with no fields at all.
pub fn required_class(operation: &SetOptions) -> ThresholdClass {
    if is_administrative(operation) {
        ThresholdClass::High
    } else {
        ThresholdClass::Medium
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::*;
    use crate::account::SignerKey;
    use crate::input::plan::parse_plan;

    fn key() -> String {
        SignerKey::from_bytes([3; 32]).to_string()
    }

    /// The operation fields, each with a valid value.
    fn fields() -> Vec<(&'static str, Value, bool)> {
        vec![
            ("master_weight", json!(1), true),
            ("low", json!(1), true),
            ("medium", json!(1), true),
            ("high", json!(1), true),
            ("signer", json!({ "key": key(), "weight": 1 }), true),
            ("home_domain", json!("example.org"), false),
            ("set_flags", json!(1), false),
            ("clear_flags", json!(2), false),
            ("source", json!(key()), false),
        ]
    }

    fn operation(present: &[(&'static str, Value, bool)]) -> SetOptions {
        let mut op = json!({ "op": "set_options" });
        for (name, value, _) in present {
            op[*name] = value.clone();
        }
        let doc = json!({
            "schema": "signerreach.plan/1",
            "transactions": [{ "id": "t", "signed_by": [], "operations": [op] }]
        });
        let plan = parse_plan(&doc.to_string()).expect("valid plan");
        plan.transactions()[0].operations()[0].clone()
    }

    #[test]
    fn an_operation_with_no_fields_needs_medium() {
        assert_eq!(required_class(&operation(&[])), ThresholdClass::Medium);
    }

    #[test]
    fn each_administrative_field_alone_needs_high() {
        for field in fields().into_iter().filter(|(_, _, admin)| *admin) {
            let name = field.0;
            assert_eq!(
                required_class(&operation(&[field])),
                ThresholdClass::High,
                "{name}"
            );
        }
    }

    #[test]
    fn each_non_administrative_field_alone_needs_medium() {
        for field in fields().into_iter().filter(|(_, _, admin)| !*admin) {
            let name = field.0;
            assert_eq!(
                required_class(&operation(&[field])),
                ThresholdClass::Medium,
                "{name}"
            );
        }
    }

    #[test]
    fn every_combination_is_high_exactly_when_an_administrative_field_is_present() {
        let all = fields();
        for mask in 0u32..(1 << all.len()) {
            let present: Vec<_> = all
                .iter()
                .enumerate()
                .filter(|(i, _)| mask & (1 << i) != 0)
                .map(|(_, field)| field.clone())
                .collect();
            // Setting and clearing the same flag is refused by the plan reader.
            if present.iter().any(|f| f.0 == "set_flags")
                && present.iter().any(|f| f.0 == "clear_flags")
            {
                continue;
            }
            let expected = if present.iter().any(|(_, _, admin)| *admin) {
                ThresholdClass::High
            } else {
                ThresholdClass::Medium
            };
            assert_eq!(
                required_class(&operation(&present)),
                expected,
                "mask {mask}"
            );
        }
    }

    #[test]
    fn removing_a_signer_and_a_zero_master_weight_still_need_high() {
        let remove = ("signer", json!({ "key": key(), "weight": 0 }), true);
        assert_eq!(required_class(&operation(&[remove])), ThresholdClass::High);
        let zero = ("master_weight", json!(0), true);
        assert_eq!(required_class(&operation(&[zero])), ThresholdClass::High);
    }

    #[test]
    fn the_envelope_needs_low() {
        assert_eq!(ENVELOPE_CLASS, ThresholdClass::Low);
    }
}
