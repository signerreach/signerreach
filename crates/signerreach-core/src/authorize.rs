//! Deciding whether a transaction is authorized by an account state.
//!
//! Rules R2, R3, R6 and R7 of `docs/SPEC.md`, section 4, evaluated as in
//! section 6.2. Every check uses the state **before** the transaction: an
//! earlier operation in the same transaction does not change who may authorize
//! a later one (R6). Applying the operations is a separate step.
//!
//! The outcome has three parts, because the network's own behaviour has three:
//!
//! - [`Authorization::Accepted`]: every check passes and every signature is
//!   necessary, so every signature is used.
//! - [`Authorization::Rejected`]: the network rejects the transaction; no state
//!   changes, no fee and no sequence number is consumed (R8).
//! - [`Authorization::Undecided`]: the outcome depends on something this tool
//!   does not model, such as the order of the signatures (R7).

use std::collections::BTreeSet;

use crate::account::{AccountState, SignerKey, ThresholdClass, Weight, sum_weights};
use crate::diagnostic::Code;
use crate::input::plan::PlannedTransaction;
use crate::rules::{ENVELOPE_CLASS, required_class};

/// A reason the network rejects a transaction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Failure {
    /// A signature matches no signer of the state (`SR-TX-01`). The network
    /// rejects a transaction with a signature nobody uses.
    ExtraSignature {
        /// The key that signed.
        key: SignerKey,
    },
    /// The signatures do not meet the envelope's LOW threshold (`SR-AUTH-01`).
    EnvelopeNotAuthorized {
        /// How many signatures matched a signer.
        matched: usize,
        /// The combined weight of the matched signers.
        have: u32,
        /// The threshold that was needed.
        need: u32,
    },
    /// The signatures do not meet an operation's threshold (`SR-AUTH-01`).
    OperationNotAuthorized {
        /// The position of the operation in the transaction.
        operation: usize,
        /// The threshold class the operation needs.
        class: ThresholdClass,
        /// How many signatures matched a signer.
        matched: usize,
        /// The combined weight of the matched signers.
        have: u32,
        /// The threshold that was needed.
        need: u32,
    },
}

impl Failure {
    /// The diagnostic code for this failure.
    pub const fn code(&self) -> Code {
        match self {
            Failure::ExtraSignature { .. } => Code::Tx01,
            Failure::EnvelopeNotAuthorized { .. } | Failure::OperationNotAuthorized { .. } => {
                Code::Auth01
            }
        }
    }
}

/// A reason the outcome cannot be decided.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Doubt {
    /// These signatures are not needed by any check, so whether the network
    /// rejects them as extra depends on the order of the signatures (`SR-TX-02`).
    UnnecessarySignatures {
        /// The keys that are not needed, in the order they were given.
        keys: Vec<SignerKey>,
    },
    /// The transaction or an operation names a source account other than the
    /// analysed account (`SR-TX-04`).
    ForeignSource {
        /// The operation, or `None` for the transaction itself.
        operation: Option<usize>,
    },
    /// Existing account flags are absent from the snapshot, so a flag change
    /// may fail even when signatures meet the required threshold.
    UnmodelledAccountFlags {
        /// The operation that changes flags.
        operation: usize,
    },
}

impl Doubt {
    /// The diagnostic code for this doubt.
    pub const fn code(&self) -> Code {
        match self {
            Doubt::UnnecessarySignatures { .. } => Code::Tx02,
            Doubt::ForeignSource { .. } => Code::Tx04,
            Doubt::UnmodelledAccountFlags { .. } => Code::State01,
        }
    }
}

/// The result of checking one transaction against one state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Authorization {
    /// Authorized, and every signature is used.
    Accepted,
    /// Rejected by the network. Every failing check is listed: extra
    /// signatures first, then the envelope, then the operations in order.
    Rejected(Vec<Failure>),
    /// Cannot be decided from what is modelled.
    Undecided(Vec<Doubt>),
}

/// Whether the signatures meet a threshold: at least one signature must match
/// a signer, even for a threshold of 0 (R2), and the combined weight must reach
/// the threshold (R3).
const fn passes(matched: usize, weight: u32, need: u32) -> bool {
    matched >= 1 && weight >= need
}

/// One check a transaction must pass: the envelope or one operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckResult {
    /// The operation, or `None` for the envelope.
    pub operation: Option<usize>,
    /// The threshold class the check needs.
    pub class: ThresholdClass,
    /// The threshold that was needed.
    pub need: u32,
    /// The combined weight of the signers that signed.
    pub have: u32,
    /// Whether the signatures meet the threshold.
    pub passed: bool,
}

/// The working behind an [`Authorization`]: who signed, with what weight, and
/// how each check came out. Source accounts are not examined here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Trace {
    /// Keys that signed and are signers of the state, with their weights.
    pub matched: Vec<(SignerKey, Weight)>,
    /// Keys that signed but are not signers of the state.
    pub unmatched: Vec<SignerKey>,
    /// The envelope check, then one check per operation, in order.
    pub checks: Vec<CheckResult>,
}

/// Works out who signed and how each check comes out, against the state before
/// the transaction (R6).
pub fn trace(state: &AccountState, transaction: &PlannedTransaction) -> Trace {
    // A key signs only if it is the account's own key with a weight above 0 or
    // an additional signer. A zero master weight is not a signer (R2).
    let mut seen = BTreeSet::new();
    let mut matched: Vec<(SignerKey, Weight)> = Vec::new();
    let mut unmatched = Vec::new();
    for &key in transaction.signed_by() {
        if !seen.insert(key) {
            continue;
        }
        let weight = state.weight_of(key);
        if weight.is_zero() {
            unmatched.push(key);
        } else {
            matched.push((key, weight));
        }
    }
    let count = matched.len();
    let have = sum_weights(matched.iter().map(|(_, weight)| *weight));

    let check = |operation: Option<usize>, class: ThresholdClass| {
        let need = u32::from(state.threshold(class).get());
        CheckResult {
            operation,
            class,
            need,
            have,
            passed: passes(count, have, need),
        }
    };
    let mut checks = vec![check(None, ENVELOPE_CLASS)];
    for (index, operation) in transaction.operations().iter().enumerate() {
        checks.push(check(Some(index), required_class(operation)));
    }
    Trace {
        matched,
        unmatched,
        checks,
    }
}

/// Checks a transaction against the state before it.
pub fn authorize(state: &AccountState, transaction: &PlannedTransaction) -> Authorization {
    let doubts = unmodelled_conditions(state, transaction);
    if !doubts.is_empty() {
        return Authorization::Undecided(doubts);
    }

    let trace = trace(state, transaction);
    let count = trace.matched.len();

    let mut failures: Vec<Failure> = trace
        .unmatched
        .iter()
        .map(|&key| Failure::ExtraSignature { key })
        .collect();
    for check in trace.checks.iter().filter(|check| !check.passed) {
        failures.push(match check.operation {
            None => Failure::EnvelopeNotAuthorized {
                matched: count,
                have: check.have,
                need: check.need,
            },
            Some(operation) => Failure::OperationNotAuthorized {
                operation,
                class: check.class,
                matched: count,
                have: check.have,
                need: check.need,
            },
        });
    }
    if !failures.is_empty() {
        return Authorization::Rejected(failures);
    }

    // Every signature that some check cannot do without is certainly used. A
    // signature no check needs may or may not be used, depending on order (R7).
    let have = trace.checks.first().map_or(0, |check| check.have);
    let unnecessary: Vec<SignerKey> = trace
        .matched
        .iter()
        .filter(|(_, weight)| {
            let without = have - u32::from(weight.get());
            trace
                .checks
                .iter()
                .all(|check| passes(count - 1, without, check.need))
        })
        .map(|(key, _)| *key)
        .collect();
    if unnecessary.is_empty() {
        Authorization::Accepted
    } else {
        Authorization::Undecided(vec![Doubt::UnnecessarySignatures { keys: unnecessary }])
    }
}

fn unmodelled_conditions(state: &AccountState, transaction: &PlannedTransaction) -> Vec<Doubt> {
    let account = state.account();
    let foreign = |source: Option<SignerKey>| source.is_some_and(|key| key != account);

    let mut doubts = Vec::new();
    if foreign(transaction.source()) {
        doubts.push(Doubt::ForeignSource { operation: None });
    }
    for (index, operation) in transaction.operations().iter().enumerate() {
        if foreign(operation.source()) {
            doubts.push(Doubt::ForeignSource {
                operation: Some(index),
            });
        }
        if operation.set_flags().is_some() || operation.clear_flags().is_some() {
            doubts.push(Doubt::UnmodelledAccountFlags { operation: index });
        }
    }
    doubts
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::*;
    use crate::account::{Threshold, Thresholds};
    use crate::input::plan::parse_plan;

    fn k(n: u8) -> SignerKey {
        SignerKey::from_bytes([n; 32])
    }

    fn state(master: u32, signers: &[(u8, u32)], thresholds: (u32, u32, u32)) -> AccountState {
        AccountState::new(
            k(1),
            Weight::new(master).unwrap(),
            Thresholds::new(
                Threshold::new(thresholds.0).unwrap(),
                Threshold::new(thresholds.1).unwrap(),
                Threshold::new(thresholds.2).unwrap(),
            ),
            signers
                .iter()
                .map(|&(n, w)| (k(n), Weight::new(w).unwrap())),
        )
        .expect("valid state")
    }

    fn signer(n: u8, weight: u32) -> Value {
        json!({ "op": "set_options", "signer": { "key": k(n).to_string(), "weight": weight } })
    }

    fn transaction(signed_by: &[u8], operations: Vec<Value>) -> PlannedTransaction {
        transaction_with(signed_by, operations, None)
    }

    fn transaction_with(
        signed_by: &[u8],
        operations: Vec<Value>,
        source: Option<u8>,
    ) -> PlannedTransaction {
        let keys: Vec<String> = signed_by.iter().map(|&n| k(n).to_string()).collect();
        let mut tx = json!({ "id": "t", "signed_by": keys, "operations": operations });
        if let Some(n) = source {
            tx["source"] = json!(k(n).to_string());
        }
        let doc = json!({ "schema": "signerreach.plan/1", "transactions": [tx] });
        parse_plan(&doc.to_string())
            .expect("valid plan")
            .transactions()[0]
            .clone()
    }

    fn rejected(outcome: Authorization) -> Vec<Failure> {
        match outcome {
            Authorization::Rejected(failures) => failures,
            other => panic!("expected a rejection, got {other:?}"),
        }
    }

    // Case 1 (fixture FX-01, Testnet 1a to 1c).
    #[test]
    fn raising_high_is_authorized_when_the_master_meets_high() {
        let before = state(1, &[(2, 1)], (1, 1, 1));
        let tx = transaction(&[1], vec![json!({ "op": "set_options", "high": 2 })]);
        assert_eq!(authorize(&before, &tx), Authorization::Accepted);
    }

    #[test]
    fn flag_changes_cannot_be_decided_without_the_existing_account_flags() {
        let before = state(1, &[], (1, 1, 1));
        let tx = transaction(&[1], vec![json!({ "op": "set_options", "set_flags": 1 })]);
        assert!(matches!(
            authorize(&before, &tx),
            Authorization::Undecided(_)
        ));
    }

    #[test]
    fn after_high_is_raised_the_master_alone_cannot_change_signers() {
        let after = state(1, &[(2, 1)], (1, 1, 2));
        let tx = transaction(&[1], vec![json!({ "op": "set_options", "high": 0 })]);
        let failures = rejected(authorize(&after, &tx));
        assert_eq!(
            failures,
            [Failure::OperationNotAuthorized {
                operation: 0,
                class: ThresholdClass::High,
                matched: 1,
                have: 1,
                need: 2
            }]
        );
        assert_eq!(failures[0].code(), Code::Auth01);
    }

    #[test]
    fn with_both_signers_the_same_change_is_authorized() {
        let after = state(1, &[(2, 1)], (1, 1, 2));
        let tx = transaction(&[1, 2], vec![json!({ "op": "set_options", "high": 0 })]);
        assert_eq!(authorize(&after, &tx), Authorization::Accepted);
    }

    // Case 2 (FX-02, Testnet 2a to 2c).
    #[test]
    fn a_rotation_in_one_transaction_is_authorized_against_the_state_before() {
        let before = state(1, &[(2, 1), (3, 1)], (1, 2, 2));
        let tx = transaction(&[1, 2], vec![signer(4, 1), signer(3, 0)]);
        assert_eq!(authorize(&before, &tx), Authorization::Accepted);
    }

    #[test]
    fn a_removed_signer_does_not_help_and_is_reported_as_extra() {
        let after = state(1, &[(2, 1), (4, 1)], (1, 2, 2));
        let tx = transaction(&[1, 3], vec![json!({ "op": "set_options", "high": 2 })]);
        let failures = rejected(authorize(&after, &tx));
        assert_eq!(
            failures,
            [
                Failure::ExtraSignature { key: k(3) },
                Failure::OperationNotAuthorized {
                    operation: 0,
                    class: ThresholdClass::High,
                    matched: 1,
                    have: 1,
                    need: 2
                }
            ]
        );
        assert_eq!(failures[0].code(), Code::Tx01);
    }

    // Case 3 (FX-03, Testnet 3a and 3b).
    #[test]
    fn zeroing_the_master_weight_is_authorized_and_then_b_alone_is_not_enough() {
        let before = state(1, &[(2, 1)], (1, 1, 2));
        let zero = transaction(
            &[1, 2],
            vec![json!({ "op": "set_options", "master_weight": 0 })],
        );
        assert_eq!(authorize(&before, &zero), Authorization::Accepted);

        let after = state(0, &[(2, 1)], (1, 1, 2));
        let tx = transaction(&[2], vec![json!({ "op": "set_options", "high": 0 })]);
        let failures = rejected(authorize(&after, &tx));
        assert!(matches!(
            failures[..],
            [Failure::OperationNotAuthorized {
                have: 1,
                need: 2,
                ..
            }]
        ));
    }

    // Case 5 (FX-05a and FX-05b, Testnet 5a, 5b-1, 5b-2).
    #[test]
    fn the_same_two_changes_pass_as_one_transaction_and_fail_as_two() {
        let before = state(1, &[(2, 1)], (1, 2, 2));

        let one = transaction(
            &[1, 2],
            vec![
                signer(2, 0),
                json!({ "op": "set_options", "master_weight": 2 }),
            ],
        );
        assert_eq!(authorize(&before, &one), Authorization::Accepted);

        let first = transaction(&[1, 2], vec![signer(2, 0)]);
        assert_eq!(authorize(&before, &first), Authorization::Accepted);
        let between = state(1, &[], (1, 2, 2));
        let second = transaction(
            &[1],
            vec![json!({ "op": "set_options", "master_weight": 2 })],
        );
        let failures = rejected(authorize(&between, &second));
        assert!(matches!(
            failures[..],
            [Failure::OperationNotAuthorized {
                operation: 0,
                have: 1,
                need: 2,
                ..
            }]
        ));
    }

    // Case 5c (FX-05c, Testnet 5c).
    #[test]
    fn a_signature_by_a_key_that_is_not_yet_a_signer_is_extra() {
        let before = state(1, &[], (0, 0, 0));
        let tx = transaction(
            &[1, 2],
            vec![
                json!({ "op": "set_options", "low": 2, "medium": 2, "high": 2,
                        "signer": { "key": k(2).to_string(), "weight": 1 } }),
                json!({ "op": "set_options", "home_domain": "example.org" }),
            ],
        );
        assert_eq!(
            rejected(authorize(&before, &tx)),
            [Failure::ExtraSignature { key: k(2) }]
        );
    }

    // FX-06g.
    #[test]
    fn a_signature_no_check_needs_is_order_dependent() {
        let before = state(2, &[(2, 1)], (1, 2, 2));
        let tx = transaction(
            &[1, 2],
            vec![json!({ "op": "set_options", "home_domain": "example.org" })],
        );
        match authorize(&before, &tx) {
            Authorization::Undecided(doubts) => {
                assert_eq!(doubts, [Doubt::UnnecessarySignatures { keys: vec![k(2)] }]);
                assert_eq!(doubts[0].code(), Code::Tx02);
            }
            other => panic!("expected a doubt, got {other:?}"),
        }
    }

    #[test]
    fn with_all_thresholds_zero_two_signatures_are_both_unnecessary() {
        let before = state(1, &[(2, 1)], (0, 0, 0));
        let tx = transaction(&[1, 2], vec![json!({ "op": "set_options" })]);
        assert_eq!(
            authorize(&before, &tx),
            Authorization::Undecided(vec![Doubt::UnnecessarySignatures {
                keys: vec![k(1), k(2)]
            }])
        );
    }

    #[test]
    fn a_threshold_of_zero_still_needs_one_matching_signature() {
        let before = state(1, &[], (0, 0, 0));
        let tx = transaction(&[], vec![json!({ "op": "set_options" })]);
        let failures = rejected(authorize(&before, &tx));
        assert_eq!(
            failures,
            [
                Failure::EnvelopeNotAuthorized {
                    matched: 0,
                    have: 0,
                    need: 0
                },
                Failure::OperationNotAuthorized {
                    operation: 0,
                    class: ThresholdClass::Medium,
                    matched: 0,
                    have: 0,
                    need: 0
                }
            ]
        );
    }

    #[test]
    fn a_master_key_with_weight_zero_is_not_a_signer() {
        let before = state(0, &[(2, 1)], (0, 0, 0));
        let tx = transaction(&[1], vec![json!({ "op": "set_options" })]);
        assert_eq!(
            rejected(authorize(&before, &tx))[0],
            Failure::ExtraSignature { key: k(1) }
        );
    }

    #[test]
    fn failures_are_listed_in_a_fixed_order() {
        let before = state(1, &[], (5, 5, 5));
        let tx = transaction(
            &[1, 9],
            vec![
                json!({ "op": "set_options", "high": 1 }),
                json!({ "op": "set_options" }),
            ],
        );
        let failures = rejected(authorize(&before, &tx));
        assert!(matches!(failures[0], Failure::ExtraSignature { .. }));
        assert!(matches!(failures[1], Failure::EnvelopeNotAuthorized { .. }));
        assert!(matches!(
            failures[2],
            Failure::OperationNotAuthorized { operation: 0, .. }
        ));
        assert!(matches!(
            failures[3],
            Failure::OperationNotAuthorized { operation: 1, .. }
        ));
    }

    #[test]
    fn the_accounts_own_key_as_source_is_fine_and_another_is_not_decided() {
        let before = state(1, &[(2, 1)], (1, 1, 1));
        let own = transaction_with(&[1], vec![json!({ "op": "set_options" })], Some(1));
        assert_eq!(authorize(&before, &own), Authorization::Accepted);

        let other = transaction_with(&[1], vec![json!({ "op": "set_options" })], Some(7));
        assert_eq!(
            authorize(&before, &other),
            Authorization::Undecided(vec![Doubt::ForeignSource { operation: None }])
        );

        let mut op = json!({ "op": "set_options" });
        op["source"] = json!(k(8).to_string());
        let foreign_op = transaction(&[1], vec![op]);
        match authorize(&before, &foreign_op) {
            Authorization::Undecided(doubts) => {
                assert_eq!(doubts, [Doubt::ForeignSource { operation: Some(0) }]);
                assert_eq!(doubts[0].code(), Code::Tx04);
            }
            other => panic!("expected a doubt, got {other:?}"),
        }
    }

    #[test]
    fn a_state_with_large_weights_does_not_overflow() {
        let signers: Vec<(u8, u32)> = (2..=21).map(|n| (n, 255)).collect();
        let before = state(255, &signers, (255, 255, 255));
        let keys: Vec<u8> = (1..=20).collect();
        let tx = transaction(&keys, vec![json!({ "op": "set_options", "high": 255 })]);
        let outcome = authorize(&before, &tx);
        // One signature already reaches 255, so the rest are unnecessary.
        assert!(matches!(outcome, Authorization::Undecided(_)));
    }
}
