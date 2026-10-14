//! Applying an authorized transaction to an account state.
//!
//! Section 6.2 of `docs/SPEC.md`. This step runs only for a transaction that
//! [`authorize`](crate::authorize::authorize) accepted. Operations are applied
//! one after another, as on the network, so the order of operations inside a
//! transaction matters for the signer limit (though never for authorization,
//! which is decided before any operation is applied).
//!
//! A transaction is atomic: if any operation cannot be applied, none is.

use std::collections::BTreeMap;

use crate::account::{
    AccountState, MAX_ADDITIONAL_SIGNERS, SignerKey, ThresholdClass, Thresholds, Weight,
};
use crate::diagnostic::Code;
use crate::error::DomainError;
use crate::input::plan::PlannedTransaction;

/// Why an authorized transaction fails when it is applied (`SR-APPLY-01`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApplyFailure {
    /// An operation names the account's own key as a signer. The master key is
    /// set with the master weight, never as a signer.
    SignerIsAccount {
        /// The position of the operation in the transaction.
        operation: usize,
        /// The key.
        key: SignerKey,
    },
    /// An operation would take the account above the signer limit.
    TooManySigners {
        /// The position of the operation in the transaction.
        operation: usize,
        /// The signer that could not be added.
        key: SignerKey,
        /// The limit.
        max: usize,
    },
    /// The resulting state broke an invariant of the account model. This is
    /// not expected to occur; it is reported rather than hidden.
    Model(DomainError),
}

impl ApplyFailure {
    /// The diagnostic code for this failure.
    pub const fn code(&self) -> Code {
        Code::Apply01
    }
}

/// Applies the operations of a transaction to a state.
///
/// Returns the state after the transaction, or why it cannot be applied. The
/// input state is never changed.
pub fn apply(
    state: &AccountState,
    transaction: &PlannedTransaction,
) -> Result<AccountState, ApplyFailure> {
    let account = state.account();
    let mut master_weight = state.master_weight();
    let mut low = state.threshold(ThresholdClass::Low);
    let mut medium = state.threshold(ThresholdClass::Medium);
    let mut high = state.threshold(ThresholdClass::High);
    let mut signers: BTreeMap<SignerKey, Weight> = state.signers().collect();

    for (operation, set_options) in transaction.operations().iter().enumerate() {
        if let Some(weight) = set_options.master_weight() {
            master_weight = weight;
        }
        if let Some(threshold) = set_options.low() {
            low = threshold;
        }
        if let Some(threshold) = set_options.medium() {
            medium = threshold;
        }
        if let Some(threshold) = set_options.high() {
            high = threshold;
        }

        if let Some(change) = set_options.signer() {
            let key = change.key();
            if key == account {
                return Err(ApplyFailure::SignerIsAccount { operation, key });
            }
            if change.weight().is_zero() {
                // Removing a signer that is not there changes nothing.
                signers.remove(&key);
            } else if !signers.contains_key(&key) && signers.len() >= MAX_ADDITIONAL_SIGNERS {
                return Err(ApplyFailure::TooManySigners {
                    operation,
                    key,
                    max: MAX_ADDITIONAL_SIGNERS,
                });
            } else {
                signers.insert(key, change.weight());
            }
        }
        // Flags and the home domain are not part of the model and change nothing.
    }

    AccountState::new(
        account,
        master_weight,
        Thresholds::new(low, medium, high),
        signers,
    )
    .map_err(ApplyFailure::Model)
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::*;
    use crate::account::Threshold;
    use crate::input::plan::parse_plan;

    fn k(n: u8) -> SignerKey {
        SignerKey::from_bytes([n; 32])
    }

    fn w(n: u32) -> Weight {
        Weight::new(n).unwrap()
    }

    fn t(n: u32) -> Threshold {
        Threshold::new(n).unwrap()
    }

    fn state(master: u32, signers: &[(u8, u32)], thresholds: (u32, u32, u32)) -> AccountState {
        AccountState::new(
            k(1),
            w(master),
            Thresholds::new(t(thresholds.0), t(thresholds.1), t(thresholds.2)),
            signers.iter().map(|&(n, weight)| (k(n), w(weight))),
        )
        .expect("valid state")
    }

    fn signer(n: u8, weight: u32) -> Value {
        json!({ "op": "set_options", "signer": { "key": k(n).to_string(), "weight": weight } })
    }

    fn transaction(operations: Vec<Value>) -> PlannedTransaction {
        let doc = json!({
            "schema": "signerreach.plan/1",
            "transactions": [{ "id": "t", "signed_by": [], "operations": operations }]
        });
        parse_plan(&doc.to_string())
            .expect("valid plan")
            .transactions()[0]
            .clone()
    }

    fn signers_of(state: &AccountState) -> Vec<(u8, u8)> {
        state
            .signers()
            .map(|(key, weight)| (key.as_bytes()[0], weight.get()))
            .collect()
    }

    // FX-02: [add D, remove C].
    #[test]
    fn a_rotation_adds_and_removes_signers() {
        let before = state(1, &[(2, 1), (3, 1)], (1, 2, 2));
        let after = apply(&before, &transaction(vec![signer(4, 1), signer(3, 0)])).unwrap();
        assert_eq!(signers_of(&after), [(2, 1), (4, 1)]);
        assert_eq!(after.master_weight(), w(1));
        assert_eq!(after.thresholds(), before.thresholds());
    }

    // FX-03: master weight to 0.
    #[test]
    fn the_master_weight_can_be_set_to_zero() {
        let before = state(1, &[(2, 1)], (1, 1, 2));
        let after = apply(
            &before,
            &transaction(vec![json!({ "op": "set_options", "master_weight": 0 })]),
        )
        .unwrap();
        assert_eq!(after.master_weight(), Weight::ZERO);
        assert_eq!(after.total_weight(), 1);
    }

    // FX-05a: [remove B, master weight 2].
    #[test]
    fn two_operations_in_one_transaction_both_take_effect() {
        let before = state(1, &[(2, 1)], (1, 2, 2));
        let tx = transaction(vec![
            signer(2, 0),
            json!({ "op": "set_options", "master_weight": 2 }),
        ]);
        let after = apply(&before, &tx).unwrap();
        assert_eq!(after.master_weight(), w(2));
        assert!(signers_of(&after).is_empty());
    }

    #[test]
    fn only_the_thresholds_that_are_given_change() {
        let before = state(1, &[], (1, 2, 3));
        let tx = transaction(vec![json!({ "op": "set_options", "medium": 9 })]);
        let after = apply(&before, &tx).unwrap();
        assert_eq!(after.threshold(ThresholdClass::Low), t(1));
        assert_eq!(after.threshold(ThresholdClass::Medium), t(9));
        assert_eq!(after.threshold(ThresholdClass::High), t(3));
    }

    #[test]
    fn re_weighting_a_signer_changes_its_weight() {
        let before = state(1, &[(2, 1)], (0, 0, 0));
        let after = apply(&before, &transaction(vec![signer(2, 7)])).unwrap();
        assert_eq!(signers_of(&after), [(2, 7)]);
    }

    #[test]
    fn removing_a_signer_that_is_not_there_changes_nothing() {
        let before = state(1, &[(2, 1)], (0, 0, 0));
        let after = apply(&before, &transaction(vec![signer(9, 0)])).unwrap();
        assert_eq!(after, before);
    }

    #[test]
    fn flags_and_home_domain_change_nothing() {
        let before = state(1, &[(2, 1)], (1, 1, 1));
        let tx = transaction(vec![
            json!({ "op": "set_options", "home_domain": "example.org", "set_flags": 1 }),
            json!({ "op": "set_options" }),
        ]);
        assert_eq!(apply(&before, &tx).unwrap(), before);
    }

    #[test]
    fn the_input_state_is_not_modified() {
        let before = state(1, &[(2, 1)], (1, 1, 1));
        let copy = before.clone();
        let _ = apply(&before, &transaction(vec![signer(3, 1)])).unwrap();
        assert_eq!(before, copy);
    }

    #[test]
    fn the_accounts_own_key_cannot_be_a_signer_even_to_remove_it() {
        let before = state(1, &[], (0, 0, 0));
        for weight in [0, 1] {
            let failure = apply(&before, &transaction(vec![signer(1, weight)])).unwrap_err();
            assert_eq!(
                failure,
                ApplyFailure::SignerIsAccount {
                    operation: 0,
                    key: k(1)
                }
            );
            assert_eq!(failure.code(), Code::Apply01);
        }
    }

    #[test]
    fn a_twenty_first_signer_is_refused_at_its_operation() {
        let twenty: Vec<(u8, u32)> = (2..22).map(|n| (n, 1)).collect();
        let before = state(1, &twenty, (0, 0, 0));
        assert_eq!(before.additional_signer_count(), 20);
        let tx = transaction(vec![
            json!({ "op": "set_options", "high": 1 }),
            signer(50, 1),
        ]);
        assert_eq!(
            apply(&before, &tx).unwrap_err(),
            ApplyFailure::TooManySigners {
                operation: 1,
                key: k(50),
                max: 20
            }
        );
    }

    #[test]
    fn a_twentieth_signer_is_accepted_and_a_reweight_at_the_limit_is_too() {
        let nineteen: Vec<(u8, u32)> = (2..21).map(|n| (n, 1)).collect();
        let before = state(1, &nineteen, (0, 0, 0));
        let at_limit = apply(&before, &transaction(vec![signer(50, 1)])).unwrap();
        assert_eq!(at_limit.additional_signer_count(), 20);
        assert!(apply(&at_limit, &transaction(vec![signer(50, 9)])).is_ok());
    }

    #[test]
    fn at_the_limit_remove_then_add_fits_and_add_then_remove_does_not() {
        let twenty: Vec<(u8, u32)> = (2..22).map(|n| (n, 1)).collect();
        let before = state(1, &twenty, (0, 0, 0));

        let remove_first = transaction(vec![signer(2, 0), signer(50, 1)]);
        let after = apply(&before, &remove_first).unwrap();
        assert_eq!(after.additional_signer_count(), 20);

        let add_first = transaction(vec![signer(50, 1), signer(2, 0)]);
        assert!(matches!(
            apply(&before, &add_first).unwrap_err(),
            ApplyFailure::TooManySigners { operation: 0, .. }
        ));
    }

    #[test]
    fn a_failing_operation_leaves_nothing_applied() {
        let before = state(1, &[(2, 1)], (1, 1, 1));
        let tx = transaction(vec![
            json!({ "op": "set_options", "master_weight": 9 }),
            signer(1, 1),
        ]);
        assert!(apply(&before, &tx).is_err());
        assert_eq!(before.master_weight(), w(1));
    }
}
