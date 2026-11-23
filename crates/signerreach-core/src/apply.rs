//! Applying an authorized transaction to an account state.
//!
//! Section 6.2 of `docs/SPEC.md`. This step runs only for a transaction that
//! [`authorize`](crate::authorize::authorize) accepted. Operations are applied
//! one after another, as on the network, so the order of operations inside a
//! transaction matters for the signer limit (though never for authorization,
//! which is decided before any operation is applied).
//!
//! A transaction is atomic: if any operation cannot be applied, none is.
//!
//! When the snapshot carries balance figures ([`Reserve`]), adding a signer is
//! also checked against them (R11), in the order the network checks: the
//! signer limit, then the sub-entry limit, then the reserve. The caller charges
//! the fee first (R13) and passes the figures as they stand after it, so the
//! operations see the balance the network would see.

use std::collections::BTreeMap;

use crate::account::{
    AccountState, MAX_ADDITIONAL_SIGNERS, SignerKey, ThresholdClass, Thresholds, Weight,
};
use crate::diagnostic::Code;
use crate::error::DomainError;
use crate::input::plan::PlannedTransaction;
use crate::reserve::{Refusal, Reserve, SUB_ENTRY_LIMIT};

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
    /// An operation would take the account above the sub-entry limit
    /// (`opTOO_MANY_SUBENTRIES`).
    TooManySubEntries {
        /// The position of the operation in the transaction.
        operation: usize,
        /// The signer that could not be added.
        key: SignerKey,
        /// The limit.
        max: u32,
    },
    /// An operation adds a signer that the available balance cannot cover
    /// (`op_low_reserve`).
    LowReserve {
        /// The position of the operation in the transaction.
        operation: usize,
        /// The signer that could not be added.
        key: SignerKey,
        /// What was available when the operation ran, in stroops. May be
        /// negative.
        available: i128,
        /// The base reserve that was needed, in stroops.
        needed: i128,
    },
    /// The resulting state broke an invariant of the account model. This is
    /// not expected to occur; it is reported rather than hidden.
    Model(DomainError),
}

impl ApplyFailure {
    /// The diagnostic code for this failure.
    pub const fn code(&self) -> Code {
        match self {
            ApplyFailure::LowReserve { .. } => Code::Reserve01,
            _ => Code::Apply01,
        }
    }
}

/// What a transaction leaves behind when it applies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Applied {
    /// The account state after the transaction.
    pub state: AccountState,
    /// The balance figures after the transaction, when there were any.
    pub reserve: Option<Reserve>,
    /// The operations that added a new signer while no balance figures were
    /// known, so whether the signer can be afforded was not checked. In order.
    pub unchecked_additions: Vec<usize>,
}

/// Applies the operations of a transaction to a state.
///
/// `reserve` is the balance figures as they stand when the first operation
/// runs, that is after the fee. Returns the state after the transaction, or why
/// it cannot be applied. The inputs are never changed.
pub fn apply(
    state: &AccountState,
    reserve: Option<Reserve>,
    transaction: &PlannedTransaction,
) -> Result<Applied, ApplyFailure> {
    let account = state.account();
    let mut reserve = reserve;
    let mut unchecked_additions = Vec::new();
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
                // Removing a signer that is not there changes nothing (R10).
                if signers.remove(&key).is_some() {
                    reserve = reserve.map(|figures| figures.with_signer_removed());
                }
            } else if signers.contains_key(&key) {
                // Re-weighting a signer needs no reserve (R10).
                signers.insert(key, change.weight());
            } else {
                if signers.len() >= MAX_ADDITIONAL_SIGNERS {
                    return Err(ApplyFailure::TooManySigners {
                        operation,
                        key,
                        max: MAX_ADDITIONAL_SIGNERS,
                    });
                }
                match reserve {
                    Some(figures) => {
                        match figures.check_new_signer() {
                            Ok(()) => {}
                            Err(Refusal::TooManySubEntries) => {
                                return Err(ApplyFailure::TooManySubEntries {
                                    operation,
                                    key,
                                    max: SUB_ENTRY_LIMIT,
                                });
                            }
                            Err(Refusal::LowReserve { available, needed }) => {
                                return Err(ApplyFailure::LowReserve {
                                    operation,
                                    key,
                                    available,
                                    needed,
                                });
                            }
                        }
                        reserve = Some(figures.with_signer_added());
                    }
                    None => unchecked_additions.push(operation),
                }
                signers.insert(key, change.weight());
            }
        }
        // Flags and the home domain are not part of the model and change nothing.
    }

    let state = AccountState::new(
        account,
        master_weight,
        Thresholds::new(low, medium, high),
        signers,
    )
    .map_err(ApplyFailure::Model)?;
    Ok(Applied {
        state,
        reserve,
        unchecked_additions,
    })
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

    /// Applies with no balance figures, as in snapshots without `reserve`.
    fn apply_state(
        state: &AccountState,
        transaction: &PlannedTransaction,
    ) -> Result<AccountState, ApplyFailure> {
        apply(state, None, transaction).map(|applied| applied.state)
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
        let after = apply_state(&before, &transaction(vec![signer(4, 1), signer(3, 0)])).unwrap();
        assert_eq!(signers_of(&after), [(2, 1), (4, 1)]);
        assert_eq!(after.master_weight(), w(1));
        assert_eq!(after.thresholds(), before.thresholds());
    }

    // FX-03: master weight to 0.
    #[test]
    fn the_master_weight_can_be_set_to_zero() {
        let before = state(1, &[(2, 1)], (1, 1, 2));
        let after = apply_state(
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
        let after = apply_state(&before, &tx).unwrap();
        assert_eq!(after.master_weight(), w(2));
        assert!(signers_of(&after).is_empty());
    }

    #[test]
    fn only_the_thresholds_that_are_given_change() {
        let before = state(1, &[], (1, 2, 3));
        let tx = transaction(vec![json!({ "op": "set_options", "medium": 9 })]);
        let after = apply_state(&before, &tx).unwrap();
        assert_eq!(after.threshold(ThresholdClass::Low), t(1));
        assert_eq!(after.threshold(ThresholdClass::Medium), t(9));
        assert_eq!(after.threshold(ThresholdClass::High), t(3));
    }

    #[test]
    fn re_weighting_a_signer_changes_its_weight() {
        let before = state(1, &[(2, 1)], (0, 0, 0));
        let after = apply_state(&before, &transaction(vec![signer(2, 7)])).unwrap();
        assert_eq!(signers_of(&after), [(2, 7)]);
    }

    #[test]
    fn removing_a_signer_that_is_not_there_changes_nothing() {
        let before = state(1, &[(2, 1)], (0, 0, 0));
        let after = apply_state(&before, &transaction(vec![signer(9, 0)])).unwrap();
        assert_eq!(after, before);
    }

    #[test]
    fn flags_and_home_domain_change_nothing() {
        let before = state(1, &[(2, 1)], (1, 1, 1));
        let tx = transaction(vec![
            json!({ "op": "set_options", "home_domain": "example.org", "set_flags": 1 }),
            json!({ "op": "set_options" }),
        ]);
        assert_eq!(apply_state(&before, &tx).unwrap(), before);
    }

    #[test]
    fn the_input_state_is_not_modified() {
        let before = state(1, &[(2, 1)], (1, 1, 1));
        let copy = before.clone();
        let _ = apply_state(&before, &transaction(vec![signer(3, 1)])).unwrap();
        assert_eq!(before, copy);
    }

    #[test]
    fn the_accounts_own_key_cannot_be_a_signer_even_to_remove_it() {
        let before = state(1, &[], (0, 0, 0));
        for weight in [0, 1] {
            let failure = apply_state(&before, &transaction(vec![signer(1, weight)])).unwrap_err();
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
            apply_state(&before, &tx).unwrap_err(),
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
        let at_limit = apply_state(&before, &transaction(vec![signer(50, 1)])).unwrap();
        assert_eq!(at_limit.additional_signer_count(), 20);
        assert!(apply_state(&at_limit, &transaction(vec![signer(50, 9)])).is_ok());
    }

    #[test]
    fn at_the_limit_remove_then_add_fits_and_add_then_remove_does_not() {
        let twenty: Vec<(u8, u32)> = (2..22).map(|n| (n, 1)).collect();
        let before = state(1, &twenty, (0, 0, 0));

        let remove_first = transaction(vec![signer(2, 0), signer(50, 1)]);
        let after = apply_state(&before, &remove_first).unwrap();
        assert_eq!(after.additional_signer_count(), 20);

        let add_first = transaction(vec![signer(50, 1), signer(2, 0)]);
        assert!(matches!(
            apply_state(&before, &add_first).unwrap_err(),
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
        assert!(apply_state(&before, &tx).is_err());
        assert_eq!(before.master_weight(), w(1));
    }

    // ---- balance, reserve and fee (R10 to R14) ----

    fn figures(balance: i64, subentries: u32) -> Reserve {
        Reserve::new(balance, 0, subentries, 0, 0, 5_000_000).expect("valid")
    }

    #[test]
    fn a_new_signer_updates_the_figures() {
        let before = state(1, &[(2, 1)], (0, 0, 0));
        let applied = apply(
            &before,
            Some(figures(25_000_000, 1)),
            &transaction(vec![signer(3, 1)]),
        )
        .unwrap();
        assert_eq!(applied.reserve.unwrap().subentry_count(), 2);
        assert_eq!(applied.reserve.unwrap().balance(), 25_000_000);
        assert!(applied.unchecked_additions.is_empty());
    }

    // FX-10a / FX-10b: the boundary of R11. The caller has already taken the
    // fee, so these figures are the ones after it.
    #[test]
    fn a_new_signer_at_exactly_the_reserve_is_applied_and_one_stroop_less_is_not() {
        let before = state(1, &[(2, 1)], (0, 0, 0));
        let tx = transaction(vec![signer(3, 1)]);
        assert!(apply(&before, Some(figures(20_000_000, 1)), &tx).is_ok());
        assert_eq!(
            apply(&before, Some(figures(19_999_999, 1)), &tx).unwrap_err(),
            ApplyFailure::LowReserve {
                operation: 0,
                key: k(3),
                available: 4_999_999,
                needed: 5_000_000
            }
        );
    }

    #[test]
    fn a_low_reserve_failure_has_the_reserve_code() {
        let before = state(1, &[], (0, 0, 0));
        let failure = apply(
            &before,
            Some(figures(0, 0)),
            &transaction(vec![signer(3, 1)]),
        )
        .unwrap_err();
        assert_eq!(failure.code(), Code::Reserve01);
    }

    // FX-12 / R10: re-weighting needs no reserve, even with nothing to spare.
    #[test]
    fn re_weighting_a_signer_needs_no_reserve() {
        let before = state(1, &[(2, 1)], (0, 0, 0));
        let applied = apply(
            &before,
            Some(figures(15_000_000, 1)),
            &transaction(vec![signer(2, 9)]),
        )
        .unwrap();
        assert_eq!(applied.reserve.unwrap().subentry_count(), 1);
    }

    // FX-11a / FX-11b: R14.
    #[test]
    fn at_the_reserve_remove_then_add_fits_and_add_then_remove_does_not() {
        let before = state(1, &[(2, 1), (3, 1)], (0, 0, 0));
        let money = figures(20_000_000, 2);

        let remove_first = transaction(vec![signer(3, 0), signer(4, 1)]);
        let applied = apply(&before, Some(money), &remove_first).unwrap();
        assert_eq!(applied.reserve.unwrap().subentry_count(), 2);
        assert_eq!(signers_of(&applied.state), [(2, 1), (4, 1)]);

        let add_first = transaction(vec![signer(4, 1), signer(3, 0)]);
        assert!(matches!(
            apply(&before, Some(money), &add_first).unwrap_err(),
            ApplyFailure::LowReserve { operation: 0, .. }
        ));
    }

    #[test]
    fn removing_a_signer_that_is_not_there_frees_nothing() {
        let before = state(1, &[(2, 1)], (0, 0, 0));
        let applied = apply(
            &before,
            Some(figures(20_000_000, 1)),
            &transaction(vec![signer(9, 0)]),
        )
        .unwrap();
        assert_eq!(applied.reserve.unwrap().subentry_count(), 1);
    }

    // FX-16.
    #[test]
    fn the_sub_entry_limit_is_reported_before_the_reserve() {
        let before = state(1, &[(2, 1)], (0, 0, 0));
        let failure = apply(
            &before,
            Some(figures(i64::MAX, 1000)),
            &transaction(vec![signer(3, 1)]),
        )
        .unwrap_err();
        assert_eq!(
            failure,
            ApplyFailure::TooManySubEntries {
                operation: 0,
                key: k(3),
                max: 1000
            }
        );
        assert_eq!(failure.code(), Code::Apply01);
    }

    #[test]
    fn the_signer_limit_is_reported_before_the_sub_entry_limit() {
        let twenty: Vec<(u8, u32)> = (2..22).map(|n| (n, 1)).collect();
        let before = state(1, &twenty, (0, 0, 0));
        let failure = apply(
            &before,
            Some(figures(i64::MAX, 1000)),
            &transaction(vec![signer(50, 1)]),
        )
        .unwrap_err();
        assert!(matches!(failure, ApplyFailure::TooManySigners { .. }));
    }

    // FX-15: without figures the signer is applied but noted.
    #[test]
    fn without_figures_an_addition_is_applied_and_noted() {
        let before = state(1, &[(2, 1)], (0, 0, 0));
        let tx = transaction(vec![
            json!({ "op": "set_options", "high": 1 }),
            signer(3, 1),
            signer(2, 5),
            signer(4, 1),
        ]);
        let applied = apply(&before, None, &tx).unwrap();
        assert_eq!(applied.unchecked_additions, [1, 3]);
        assert!(applied.reserve.is_none());
        assert_eq!(applied.state.additional_signer_count(), 3);
    }

    #[test]
    fn a_failure_after_a_signer_was_added_leaves_nothing_applied() {
        let before = state(1, &[(2, 1)], (0, 0, 0));
        let money = figures(20_000_000, 1);
        let tx = transaction(vec![signer(3, 1), signer(4, 1)]);
        // The first addition uses the spare reserve, the second cannot be paid.
        let failure = apply(&before, Some(money), &tx).unwrap_err();
        assert!(matches!(
            failure,
            ApplyFailure::LowReserve { operation: 1, .. }
        ));
        assert_eq!(before.additional_signer_count(), 1);
    }
}
