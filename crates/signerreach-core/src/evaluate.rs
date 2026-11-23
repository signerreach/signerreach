//! Evaluating a plan against a snapshot and an availability declaration.
//!
//! Sections 6.2 to 6.4 of `docs/SPEC.md`. The plan is walked transaction by
//! transaction, each authorized against the state the previous one left. The
//! state at the end is then checked, for the baseline declaration and for each
//! named scenario, for whether each threshold class can still be reached by
//! signers declared available.
//!
//! The verdict is the worst effect of any finding. A finding that is only
//! informational never changes it.

use crate::account::{AccountState, SignerKey, ThresholdClass};
use crate::apply::{ApplyFailure, apply};
use crate::authorize::{Authorization, Doubt, Failure, Trace, authorize, trace};
use crate::diagnostic::{Code, Effect};
use crate::input::availability::{Availability, Intent, Status};
use crate::input::plan::Plan;
use crate::input::snapshot::Snapshot;
use crate::reserve::Reserve;

/// The answer.
///
/// Ordered by severity: when several results are combined, the largest wins.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Verdict {
    /// A path to administer the account remains, under the stated assumptions.
    SafeWithinScope,
    /// The result cannot be established from what is declared and modelled.
    Inconclusive,
    /// The plan fails, or administrative access is lost.
    Unsafe,
}

impl Verdict {
    /// The text printed in reports.
    pub const fn as_str(self) -> &'static str {
        match self {
            Verdict::SafeWithinScope => "SAFE WITHIN SCOPE",
            Verdict::Inconclusive => "INCONCLUSIVE",
            Verdict::Unsafe => "UNSAFE",
        }
    }

    fn of(effect: Effect) -> Self {
        match effect {
            Effect::Info => Verdict::SafeWithinScope,
            Effect::Inconclusive => Verdict::Inconclusive,
            Effect::Unsafe => Verdict::Unsafe,
        }
    }
}

/// What happened to one transaction of the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StepResult {
    /// Authorized and applied.
    Accepted,
    /// Rejected by the network; the state is unchanged.
    Rejected(Vec<Failure>),
    /// Authorized but would fail when applied; the state is unchanged.
    FailedToApply(ApplyFailure),
    /// Cannot be decided. Later transactions are not evaluated, because the
    /// state they depend on is not known.
    Undecided(Vec<Doubt>),
    /// Not evaluated, because an earlier transaction was undecided.
    NotEvaluated,
}

/// The fee a transaction was charged in the model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FeeUsed {
    /// The fee, in stroops.
    pub amount: u32,
    /// True when the plan gave no fee and 100 stroops for each operation was
    /// assumed.
    pub assumed: bool,
}

/// One transaction of the plan and its result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step {
    /// The transaction's id.
    pub id: String,
    /// The state the transaction was checked against; `None` if it was not
    /// evaluated.
    pub before: Option<AccountState>,
    /// The balance figures the transaction was checked against; `None` if it
    /// was not evaluated or the snapshot has none.
    pub reserve_before: Option<Reserve>,
    /// The fee used for the balance checks. `None` unless the snapshot has
    /// balance figures and the transaction got as far as the fee check.
    pub fee: Option<FeeUsed>,
    /// Who signed and how each check came out; `None` if it was not evaluated.
    pub trace: Option<Trace>,
    /// What happened.
    pub result: StepResult,
}

/// Whether a threshold class can be reached.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reach {
    /// Signers declared available meet the threshold.
    Reachable,
    /// Only if signers whose availability is unknown turn out to be usable.
    DependsOnUnknown,
    /// All signers together could meet it, but not those declared available.
    Lost,
    /// All signers together cannot meet it, whoever holds the keys.
    Structural,
}

/// Reachability of one threshold class in one scenario.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassReach {
    /// The class.
    pub class: ThresholdClass,
    /// The threshold.
    pub threshold: u8,
    /// Combined weight of every signer.
    pub total: u32,
    /// Combined weight of signers declared available.
    pub available: u32,
    /// Combined weight of signers whose availability is unknown.
    pub unknown: u32,
    /// The conclusion.
    pub reach: Reach,
}

/// One diagnostic, with the scenario it belongs to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    /// The code.
    pub code: Code,
    /// The scenario name; `None` for the baseline or the plan as a whole.
    pub scenario: Option<String>,
    /// A short explanation.
    pub message: String,
}

/// The state at the end of the plan, as seen under one declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScenarioResult {
    /// The scenario name; `None` for the baseline declaration.
    pub name: Option<String>,
    /// Every signer that can sign, with its status in this scenario: the
    /// master key first (if its weight is above 0), then the others by key.
    pub signers: Vec<SignerStatus>,
    /// LOW, MEDIUM and HIGH, in that order.
    pub reach: Vec<ClassReach>,
    /// Findings specific to this scenario.
    pub findings: Vec<Finding>,
    /// The verdict for this scenario, including plan-level findings.
    pub verdict: Verdict,
}

/// The full result of evaluating a plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Evaluation {
    /// Each transaction and what happened to it.
    pub steps: Vec<Step>,
    /// The state after the last transaction that was applied.
    pub final_state: AccountState,
    /// The balance figures after the last transaction, when the snapshot has
    /// any. A transaction that failed when applied has still paid its fee.
    pub final_reserve: Option<Reserve>,
    /// Findings about the plan as a whole.
    pub plan_findings: Vec<Finding>,
    /// The baseline and each named scenario. Empty if evaluation stopped
    /// because a transaction was undecided.
    pub scenarios: Vec<ScenarioResult>,
    /// The overall verdict: the worst across the plan and every scenario.
    pub verdict: Verdict,
}

/// Evaluates a plan.
pub fn evaluate(snapshot: &Snapshot, plan: &Plan, availability: &Availability) -> Evaluation {
    let mut state = snapshot.state().clone();
    let mut reserve = snapshot.reserve();
    let mut steps = Vec::new();
    let mut plan_findings = Vec::new();
    let mut stopped = false;

    for transaction in plan.transactions() {
        let id = transaction.id().to_string();
        if stopped {
            steps.push(Step {
                id,
                before: None,
                reserve_before: None,
                fee: None,
                trace: None,
                result: StepResult::NotEvaluated,
            });
            continue;
        }
        let before = state.clone();
        let reserve_before = reserve;
        let mut fee_used = None;
        let step_trace = trace(&state, transaction);
        let result = match authorize(&state, transaction) {
            Authorization::Accepted => {
                // The fee is charged before the first operation (R13), and kept
                // if an operation then fails (R12).
                let (amount, assumed) = transaction.fee_or_assumed();
                let after_fee = reserve.map(|figures| figures.after_fee(amount));
                let unaffordable = reserve
                    .map(|figures| figures.available())
                    .filter(|&available| available < i128::from(amount));
                if reserve.is_some() {
                    fee_used = Some(FeeUsed { amount, assumed });
                }
                if let Some(available) = unaffordable {
                    let failure = Failure::FeeUnaffordable {
                        fee: amount,
                        available,
                    };
                    plan_findings.push(finding(
                        failure.code(),
                        None,
                        failure_message(&id, &failure),
                    ));
                    StepResult::Rejected(vec![failure])
                } else {
                    match apply(&state, after_fee, transaction) {
                        Ok(applied) => {
                            for operation in &applied.unchecked_additions {
                                plan_findings.push(finding(
                                    Code::State01,
                                    None,
                                    format!(
                                        "transaction {id}, operation {operation}: adds a signer, \
                                         and the snapshot has no reserve data, so it is not known \
                                         whether the balance can cover it"
                                    ),
                                ));
                            }
                            state = applied.state;
                            reserve = applied.reserve;
                            StepResult::Accepted
                        }
                        Err(failure) => {
                            reserve = after_fee;
                            plan_findings.push(finding(
                                failure.code(),
                                None,
                                apply_message(&id, &failure),
                            ));
                            StepResult::FailedToApply(failure)
                        }
                    }
                }
            }
            Authorization::Rejected(failures) => {
                for failure in &failures {
                    plan_findings.push(finding(
                        failure.code(),
                        None,
                        failure_message(&id, failure),
                    ));
                }
                StepResult::Rejected(failures)
            }
            Authorization::Undecided(doubts) => {
                for doubt in &doubts {
                    plan_findings.push(finding(doubt.code(), None, doubt_message(&id, doubt)));
                }
                stopped = true;
                StepResult::Undecided(doubts)
            }
        };
        steps.push(Step {
            id,
            before: Some(before),
            reserve_before,
            fee: fee_used,
            trace: Some(step_trace),
            result,
        });
    }

    if availability.intent() == Intent::Irreversible {
        plan_findings.push(finding(
            Code::Intent01,
            None,
            "the plan is declared irreversible; the result is unchanged".to_string(),
        ));
    }

    let plan_level = worst(&plan_findings);

    let mut scenarios = Vec::new();
    if !stopped {
        scenarios.push(scenario_result(
            &state,
            availability,
            None,
            plan_level,
            |key| availability.status(key),
        ));
        for scenario in availability.scenarios() {
            scenarios.push(scenario_result(
                &state,
                availability,
                Some(scenario.name().to_string()),
                plan_level,
                |key| availability.status_in(scenario, key),
            ));
        }
    }

    let verdict = scenarios
        .iter()
        .map(|result| result.verdict)
        .fold(plan_level, Verdict::max);

    Evaluation {
        steps,
        final_state: state,
        final_reserve: reserve,
        plan_findings,
        scenarios,
        verdict,
    }
}

fn finding(code: Code, scenario: Option<&str>, message: String) -> Finding {
    Finding {
        code,
        scenario: scenario.map(str::to_string),
        message,
    }
}

fn worst(findings: &[Finding]) -> Verdict {
    findings
        .iter()
        .map(|f| Verdict::of(f.code.effect()))
        .max()
        .unwrap_or(Verdict::SafeWithinScope)
}

/// One signer that can contribute to a threshold, with its declared status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignerStatus {
    /// The key.
    pub key: SignerKey,
    /// The weight.
    pub weight: u32,
    /// The declared availability.
    pub status: Status,
}

fn holders(state: &AccountState, status: impl Fn(SignerKey) -> Status) -> Vec<SignerStatus> {
    let mut holders = Vec::new();
    if !state.master_weight().is_zero() {
        let key = state.account();
        holders.push(SignerStatus {
            key,
            weight: u32::from(state.master_weight().get()),
            status: status(key),
        });
    }
    for (key, weight) in state.signers() {
        holders.push(SignerStatus {
            key,
            weight: u32::from(weight.get()),
            status: status(key),
        });
    }
    holders
}

fn weight_where(holders: &[SignerStatus], wanted: Status) -> (u32, usize) {
    holders
        .iter()
        .filter(|h| h.status == wanted)
        .fold((0, 0), |(sum, count), h| (sum + h.weight, count + 1))
}

fn class_reach(
    state: &AccountState,
    holders: &[SignerStatus],
    class: ThresholdClass,
) -> ClassReach {
    let threshold = state.threshold(class).get();
    let need = u32::from(threshold);
    let total: u32 = holders.iter().map(|h| h.weight).sum();
    let (available, available_count) = weight_where(holders, Status::Available);
    let (unknown, unknown_count) = weight_where(holders, Status::Unknown);

    // A signature is always needed, even for a threshold of 0 (R2).
    let reach = if available_count >= 1 && available >= need {
        Reach::Reachable
    } else if holders.is_empty() || total < need {
        Reach::Structural
    } else if available_count + unknown_count >= 1 && available + unknown >= need {
        Reach::DependsOnUnknown
    } else {
        Reach::Lost
    };
    ClassReach {
        class,
        threshold,
        total,
        available,
        unknown,
        reach,
    }
}

fn scenario_result(
    state: &AccountState,
    availability: &Availability,
    name: Option<String>,
    plan_level: Verdict,
    status: impl Fn(SignerKey) -> Status,
) -> ScenarioResult {
    let holders = holders(state, status);
    let reach: Vec<ClassReach> = ThresholdClass::ALL
        .iter()
        .map(|&class| class_reach(state, &holders, class))
        .collect();
    let by_class = |class: ThresholdClass| {
        reach
            .iter()
            .find(|r| r.class == class)
            .expect("every class is present")
    };

    let scenario = name.as_deref();
    let mut findings = Vec::new();
    let high = by_class(ThresholdClass::High);
    match high.reach {
        Reach::Structural => findings.push(finding(
            Code::Lock01,
            scenario,
            format!(
                "the total weight of all signers is {}, below the HIGH threshold {}",
                high.total, high.threshold
            ),
        )),
        Reach::Lost => findings.push(finding(
            Code::Loss01,
            scenario,
            format!(
                "signers declared available weigh {}, below the HIGH threshold {}",
                high.available, high.threshold
            ),
        )),
        Reach::DependsOnUnknown => findings.push(finding(
            Code::Avail01,
            scenario,
            format!(
                "reaching the HIGH threshold {} needs signers whose availability is unknown",
                high.threshold
            ),
        )),
        Reach::Reachable => {
            let low = by_class(ThresholdClass::Low);
            if low.reach != Reach::Reachable {
                findings.push(finding(
                    Code::Tx03,
                    scenario,
                    format!(
                        "the LOW threshold {} cannot be met by available signers, so the account \
                         cannot be a transaction source and administering it needs another funded \
                         source account",
                        low.threshold
                    ),
                ));
            }
            let medium = by_class(ThresholdClass::Medium);
            if medium.reach != Reach::Reachable {
                findings.push(finding(
                    Code::Lock02,
                    scenario,
                    format!(
                        "the MEDIUM threshold {} cannot be met by available signers, so payments \
                         are blocked until it is lowered",
                        medium.threshold
                    ),
                ));
            }
            if availability.require_survive_loss_of() >= 1 {
                let mut fragile = holders
                    .iter()
                    .filter(|h| h.status == Status::Available)
                    .filter(|h| {
                        let (available, count) = weight_where(&holders, Status::Available);
                        count < 2 || available - h.weight < u32::from(high.threshold)
                    })
                    .map(|h| h.key);
                if let Some(first) = fragile.next() {
                    let more = fragile.count();
                    findings.push(finding(
                        Code::Loss01,
                        scenario,
                        format!(
                            "the HIGH threshold would not stay reachable after losing {first}{}",
                            if more == 0 {
                                String::new()
                            } else {
                                format!(" or {more} other available signer(s)")
                            }
                        ),
                    ));
                }
            }
        }
    }

    let verdict = Verdict::max(plan_level, worst(&findings));
    ScenarioResult {
        name,
        signers: holders,
        reach,
        findings,
        verdict,
    }
}

fn failure_message(id: &str, failure: &Failure) -> String {
    match failure {
        Failure::ExtraSignature { key } => format!(
            "transaction {id}: the signature of {key} matches no signer, so the network rejects it \
             as an extra signature"
        ),
        Failure::EnvelopeNotAuthorized { have, need, .. } => format!(
            "transaction {id}: the envelope needs LOW {need} and the signatures give {have}"
        ),
        Failure::OperationNotAuthorized {
            operation,
            class,
            have,
            need,
            ..
        } => format!(
            "transaction {id}, operation {operation}: needs {class} {need} and the signatures give \
             {have}"
        ),
        Failure::FeeUnaffordable { fee, available } => format!(
            "transaction {id}: the fee is {fee} stroops and {available} are available (the balance \
             less the minimum balance and selling liabilities), so the network rejects it before \
             applying it"
        ),
    }
}

fn doubt_message(id: &str, doubt: &Doubt) -> String {
    match doubt {
        Doubt::UnnecessarySignatures { keys } => format!(
            "transaction {id}: {} signature(s) are not needed by any check, so whether the network \
             rejects them depends on the order of the signatures",
            keys.len()
        ),
        Doubt::ForeignSource { operation: None } => {
            format!("transaction {id}: names a source account other than the analysed account")
        }
        Doubt::ForeignSource {
            operation: Some(operation),
        } => format!(
            "transaction {id}, operation {operation}: names a source account other than the \
             analysed account"
        ),
        Doubt::UnmodelledAccountFlags { operation } => format!(
            "transaction {id}, operation {operation}: changes account flags, but the snapshot \
             does not record their current state"
        ),
    }
}

fn apply_message(id: &str, failure: &ApplyFailure) -> String {
    match failure {
        ApplyFailure::SignerIsAccount { operation, .. } => format!(
            "transaction {id}, operation {operation}: names the account's own key as a signer"
        ),
        ApplyFailure::TooManySigners { operation, max, .. } => format!(
            "transaction {id}, operation {operation}: would take the account above {max} \
             additional signers"
        ),
        ApplyFailure::TooManySubEntries { operation, max, .. } => format!(
            "transaction {id}, operation {operation}: would take the account above {max} \
             sub-entries"
        ),
        ApplyFailure::LowReserve {
            operation,
            key,
            available,
            needed,
        } => format!(
            "transaction {id}, operation {operation}: adding signer {key} needs {needed} stroops \
             available and {available} are, a shortfall of {}",
            needed - available
        ),
        ApplyFailure::Model(error) => format!("transaction {id}: {error}"),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::testing::{
        availability, availability_with, k, plan, signer_op, snapshot, snapshot_funded, tx,
        tx_with_fee,
    };

    fn codes(findings: &[Finding]) -> Vec<Code> {
        findings.iter().map(|f| f.code).collect()
    }

    fn all_codes(evaluation: &Evaluation) -> Vec<Code> {
        let mut found = codes(&evaluation.plan_findings);
        for scenario in &evaluation.scenarios {
            found.extend(codes(&scenario.findings));
        }
        found
    }

    // FX-01 (Testnet 1a to 1c).
    #[test]
    fn raising_high_with_a_lost_signer_is_unsafe() {
        let result = evaluate(
            &snapshot(1, &[(2, 1)], (1, 1, 1)),
            &plan(vec![tx(
                "tx1",
                &[1],
                vec![json!({ "op": "set_options", "high": 2 })],
            )]),
            &availability(&[(1, "available"), (2, "lost")]),
        );
        assert_eq!(result.verdict, Verdict::Unsafe);
        assert_eq!(result.steps[0].result, StepResult::Accepted);
        assert_eq!(all_codes(&result), [Code::Loss01]);
        let high = &result.scenarios[0].reach[2];
        assert_eq!(
            (
                high.class,
                high.threshold,
                high.total,
                high.available,
                high.reach
            ),
            (ThresholdClass::High, 2, 2, 1, Reach::Lost)
        );
    }

    // FX-02 (Testnet 2a, 2b).
    #[test]
    fn a_rotation_with_enough_available_signers_is_safe() {
        let result = evaluate(
            &snapshot_funded(1, &[(2, 1), (3, 1)], (1, 2, 2), (100_000_000_000, 0, 2)),
            &plan(vec![tx(
                "rotate",
                &[1, 2],
                vec![signer_op(4, 1), signer_op(3, 0)],
            )]),
            &availability(&[
                (1, "available"),
                (2, "available"),
                (4, "available"),
                (3, "lost"),
            ]),
        );
        assert_eq!(result.verdict, Verdict::SafeWithinScope);
        assert!(all_codes(&result).is_empty());
        assert_eq!(result.final_state.additional_signer_count(), 2);
    }

    // FX-03 (Testnet 3a, 3b).
    #[test]
    fn zeroing_the_master_when_total_weight_is_too_low_is_a_structural_lock() {
        let result = evaluate(
            &snapshot(1, &[(2, 1)], (1, 1, 2)),
            &plan(vec![tx(
                "zero",
                &[1, 2],
                vec![json!({ "op": "set_options", "master_weight": 0 })],
            )]),
            &availability(&[(1, "available"), (2, "available")]),
        );
        assert_eq!(result.verdict, Verdict::Unsafe);
        assert_eq!(all_codes(&result), [Code::Lock01]);
        assert_eq!(result.scenarios[0].reach[2].reach, Reach::Structural);
    }

    // FX-04.
    #[test]
    fn a_result_that_depends_on_unknown_signers_is_inconclusive() {
        let result = evaluate(
            &snapshot(1, &[(2, 1), (3, 1)], (1, 2, 2)),
            &plan(vec![]),
            &availability(&[(1, "available")]),
        );
        assert_eq!(result.verdict, Verdict::Inconclusive);
        assert_eq!(all_codes(&result), [Code::Avail01]);
        assert_eq!(result.scenarios[0].reach[2].reach, Reach::DependsOnUnknown);
    }

    // FX-05a and FX-05b (Testnet 5a, 5b-1, 5b-2).
    #[test]
    fn the_same_changes_are_safe_as_one_transaction_and_unsafe_as_two() {
        let before = snapshot(1, &[(2, 1)], (1, 2, 2));
        let everyone = availability(&[(1, "available"), (2, "available")]);

        let one = evaluate(
            &before,
            &plan(vec![tx(
                "one",
                &[1, 2],
                vec![
                    signer_op(2, 0),
                    json!({ "op": "set_options", "master_weight": 2 }),
                ],
            )]),
            &everyone,
        );
        assert_eq!(one.verdict, Verdict::SafeWithinScope);

        let two = evaluate(
            &before,
            &plan(vec![
                tx("first", &[1, 2], vec![signer_op(2, 0)]),
                tx(
                    "second",
                    &[1],
                    vec![json!({ "op": "set_options", "master_weight": 2 })],
                ),
            ]),
            &everyone,
        );
        assert_eq!(two.verdict, Verdict::Unsafe);
        assert_eq!(two.steps[0].result, StepResult::Accepted);
        assert!(matches!(two.steps[1].result, StepResult::Rejected(_)));
        assert!(codes(&two.plan_findings).contains(&Code::Auth01));
    }

    // FX-05c (Testnet 5c).
    #[test]
    fn an_extra_signature_makes_the_plan_unsafe() {
        let result = evaluate(
            &snapshot(1, &[], (0, 0, 0)),
            &plan(vec![tx(
                "t",
                &[1, 2],
                vec![
                    json!({ "op": "set_options", "high": 2, "signer": { "key": k(2), "weight": 1 } }),
                ],
            )]),
            &availability(&[(1, "available"), (2, "available")]),
        );
        assert_eq!(result.verdict, Verdict::Unsafe);
        assert_eq!(codes(&result.plan_findings), [Code::Tx01]);
    }

    // FX-06g.
    #[test]
    fn an_unneeded_signature_is_inconclusive_and_stops_the_walk() {
        let result = evaluate(
            &snapshot(2, &[(2, 1)], (1, 2, 2)),
            &plan(vec![
                tx(
                    "first",
                    &[1, 2],
                    vec![json!({ "op": "set_options", "home_domain": "example.org" })],
                ),
                // Would be rejected if evaluated against the unchanged state.
                tx(
                    "second",
                    &[],
                    vec![json!({ "op": "set_options", "high": 1 })],
                ),
            ]),
            &availability(&[(1, "available"), (2, "available")]),
        );
        assert_eq!(result.verdict, Verdict::Inconclusive);
        assert!(matches!(result.steps[0].result, StepResult::Undecided(_)));
        assert_eq!(result.steps[1].result, StepResult::NotEvaluated);
        assert!(result.scenarios.is_empty());
        assert_eq!(codes(&result.plan_findings), [Code::Tx02]);
    }

    #[test]
    fn a_rejected_transaction_does_not_stop_later_ones_being_reported() {
        let result = evaluate(
            &snapshot(1, &[], (1, 1, 1)),
            &plan(vec![
                tx("bad", &[], vec![json!({ "op": "set_options", "high": 5 })]),
                tx(
                    "also-bad",
                    &[],
                    vec![json!({ "op": "set_options", "low": 5 })],
                ),
            ]),
            &availability(&[(1, "available")]),
        );
        assert!(matches!(result.steps[0].result, StepResult::Rejected(_)));
        assert!(matches!(result.steps[1].result, StepResult::Rejected(_)));
        assert_eq!(result.verdict, Verdict::Unsafe);
    }

    // FX-07 (Testnet 7a to 7c).
    #[test]
    fn an_unreachable_low_threshold_is_inconclusive() {
        let result = evaluate(
            &snapshot(1, &[], (5, 0, 0)),
            &plan(vec![]),
            &availability(&[(1, "available")]),
        );
        assert_eq!(result.verdict, Verdict::Inconclusive);
        assert_eq!(all_codes(&result), [Code::Tx03]);
    }

    // FX-08.
    #[test]
    fn requiring_survival_of_a_loss_fails_when_one_key_is_critical() {
        let result = evaluate(
            &snapshot(1, &[(2, 1)], (1, 1, 2)),
            &plan(vec![]),
            &availability_with(
                &[(1, "available"), (2, "available")],
                json!([]),
                1,
                "recoverable",
            ),
        );
        assert_eq!(result.verdict, Verdict::Unsafe);
        assert_eq!(all_codes(&result), [Code::Loss01]);
    }

    #[test]
    fn survival_of_a_loss_holds_with_enough_spare_weight() {
        let result = evaluate(
            &snapshot(1, &[(2, 1), (3, 1)], (1, 1, 1)),
            &plan(vec![]),
            &availability_with(
                &[(1, "available"), (2, "available"), (3, "available")],
                json!([]),
                1,
                "recoverable",
            ),
        );
        assert_eq!(result.verdict, Verdict::SafeWithinScope);
    }

    #[test]
    fn the_worst_scenario_decides_and_is_named() {
        let scenarios = json!([
            { "name": "A-lost", "overrides": { k(1): "lost" } },
            { "name": "A-and-B-lost", "overrides": { k(1): "lost", k(2): "lost" } }
        ]);
        let result = evaluate(
            &snapshot(1, &[(2, 1), (3, 1)], (1, 2, 2)),
            &plan(vec![]),
            &availability_with(
                &[(1, "available"), (2, "available"), (3, "available")],
                scenarios,
                0,
                "recoverable",
            ),
        );
        assert_eq!(result.scenarios.len(), 3);
        assert_eq!(result.scenarios[0].verdict, Verdict::SafeWithinScope);
        assert_eq!(result.scenarios[1].verdict, Verdict::SafeWithinScope);
        assert_eq!(result.scenarios[2].verdict, Verdict::Unsafe);
        assert_eq!(result.scenarios[2].name.as_deref(), Some("A-and-B-lost"));
        assert_eq!(
            result.scenarios[2].findings[0].scenario.as_deref(),
            Some("A-and-B-lost")
        );
        assert_eq!(result.verdict, Verdict::Unsafe);
    }

    #[test]
    fn a_plan_level_failure_applies_to_every_scenario() {
        let scenarios = json!([{ "name": "s", "overrides": {} }]);
        let result = evaluate(
            &snapshot(1, &[], (1, 1, 1)),
            &plan(vec![tx(
                "bad",
                &[],
                vec![json!({ "op": "set_options", "high": 5 })],
            )]),
            &availability_with(&[(1, "available")], scenarios, 0, "recoverable"),
        );
        assert!(
            result
                .scenarios
                .iter()
                .all(|s| s.verdict == Verdict::Unsafe)
        );
    }

    #[test]
    fn a_declared_irreversible_intent_never_changes_the_verdict() {
        let make = |intent: &str| {
            evaluate(
                &snapshot(1, &[(2, 1)], (1, 1, 2)),
                &plan(vec![]),
                &availability_with(&[(1, "available"), (2, "lost")], json!([]), 0, intent),
            )
        };
        let recoverable = make("recoverable");
        let irreversible = make("irreversible");
        assert_eq!(recoverable.verdict, Verdict::Unsafe);
        assert_eq!(irreversible.verdict, Verdict::Unsafe);
        assert!(codes(&irreversible.plan_findings).contains(&Code::Intent01));
        assert!(!codes(&recoverable.plan_findings).contains(&Code::Intent01));

        let safe = evaluate(
            &snapshot(1, &[], (1, 1, 1)),
            &plan(vec![]),
            &availability_with(&[(1, "available")], json!([]), 0, "irreversible"),
        );
        assert_eq!(safe.verdict, Verdict::SafeWithinScope);
    }

    #[test]
    fn an_empty_plan_checks_the_snapshot_itself() {
        let result = evaluate(
            &snapshot(1, &[], (1, 1, 1)),
            &plan(vec![]),
            &availability(&[(1, "available")]),
        );
        assert!(result.steps.is_empty());
        assert_eq!(result.verdict, Verdict::SafeWithinScope);
        assert_eq!(result.scenarios.len(), 1);
        assert_eq!(result.scenarios[0].name, None);
    }

    #[test]
    fn a_blocked_medium_threshold_is_only_information() {
        let result = evaluate(
            &snapshot(1, &[], (0, 5, 1)),
            &plan(vec![]),
            &availability(&[(1, "available")]),
        );
        assert_eq!(result.verdict, Verdict::SafeWithinScope);
        assert_eq!(all_codes(&result), [Code::Lock02]);
    }

    #[test]
    fn a_threshold_of_zero_still_needs_an_available_signer() {
        let result = evaluate(
            &snapshot(1, &[], (0, 0, 0)),
            &plan(vec![]),
            &availability(&[(1, "lost")]),
        );
        assert_eq!(result.verdict, Verdict::Unsafe);
        assert_eq!(result.scenarios[0].reach[2].reach, Reach::Lost);
    }

    #[test]
    fn an_account_with_no_usable_signer_at_all_is_a_structural_lock() {
        let result = evaluate(
            &snapshot(0, &[], (0, 0, 0)),
            &plan(vec![]),
            &availability(&[]),
        );
        assert_eq!(result.verdict, Verdict::Unsafe);
        assert_eq!(all_codes(&result), [Code::Lock01]);
    }

    #[test]
    fn verdicts_are_ordered_by_severity_and_named() {
        assert!(Verdict::Unsafe > Verdict::Inconclusive);
        assert!(Verdict::Inconclusive > Verdict::SafeWithinScope);
        assert_eq!(Verdict::SafeWithinScope.as_str(), "SAFE WITHIN SCOPE");
        assert_eq!(Verdict::Inconclusive.as_str(), "INCONCLUSIVE");
        assert_eq!(Verdict::Unsafe.as_str(), "UNSAFE");
    }

    #[test]
    fn the_same_input_gives_the_same_result() {
        let run = || {
            evaluate(
                &snapshot(1, &[(2, 1), (3, 1)], (1, 2, 2)),
                &plan(vec![tx("t", &[1, 2], vec![signer_op(4, 1)])]),
                &availability(&[(1, "available"), (2, "lost")]),
            )
        };
        assert_eq!(run(), run());
    }

    // ---- balance, reserve and fee (R10 to R14, FX-10 to FX-17) ----

    /// Account 1 with signer 2 (weight 1), thresholds 1/2/2, one sub-entry,
    /// and the given balance and selling liabilities.
    fn funded(balance: i64, liabilities: i64) -> Snapshot {
        snapshot_funded(1, &[(2, 1)], (1, 2, 2), (balance, liabilities, 1))
    }

    fn everyone() -> Availability {
        availability(&[
            (1, "available"),
            (2, "available"),
            (3, "available"),
            (4, "available"),
        ])
    }

    fn add_d() -> Plan {
        plan(vec![tx("tx1", &[1, 2], vec![signer_op(4, 1)])])
    }

    fn shortfall_of(result: &Evaluation) -> String {
        result
            .plan_findings
            .iter()
            .find(|f| f.code == Code::Reserve01)
            .map(|f| f.message.clone())
            .expect("a reserve finding")
    }

    // FX-10a.
    #[test]
    fn a_signer_is_safe_at_exactly_the_minimum_balance_plus_reserve_plus_fee() {
        let result = evaluate(&funded(20_000_100, 0), &add_d(), &everyone());
        assert_eq!(result.verdict, Verdict::SafeWithinScope);
        assert_eq!(result.steps[0].result, StepResult::Accepted);
        assert_eq!(
            result.steps[0].fee,
            Some(FeeUsed {
                amount: 100,
                assumed: true
            })
        );
        let after = result.final_reserve.expect("figures after");
        assert_eq!((after.balance(), after.subentry_count()), (20_000_000, 2));
        assert_eq!(after.available(), 0);
    }

    // FX-10b.
    #[test]
    fn one_stroop_less_is_unsafe_with_the_shortfall_named() {
        let result = evaluate(&funded(20_000_099, 0), &add_d(), &everyone());
        assert_eq!(result.verdict, Verdict::Unsafe);
        assert!(matches!(
            result.steps[0].result,
            StepResult::FailedToApply(ApplyFailure::LowReserve { .. })
        ));
        assert_eq!(all_codes(&result), [Code::Reserve01]);
        assert!(shortfall_of(&result).ends_with("a shortfall of 1"));
        // The signer was not added, but the fee was paid.
        assert_eq!(result.final_state.additional_signer_count(), 1);
        assert_eq!(result.final_reserve.unwrap().balance(), 19_999_999);
        assert_eq!(result.final_reserve.unwrap().subentry_count(), 1);
    }

    // FX-10c, the Testnet failure at ledger 5831809.
    #[test]
    fn the_testnet_failure_is_reported_unsafe() {
        let result = evaluate(
            &snapshot_funded(1, &[], (0, 0, 0), (10_010_000, 0, 0)),
            &plan(vec![tx("tx1", &[1], vec![signer_op(2, 1)])]),
            &availability(&[(1, "available"), (2, "available")]),
        );
        assert_eq!(result.verdict, Verdict::Unsafe);
        assert_eq!(all_codes(&result), [Code::Reserve01]);
        assert!(shortfall_of(&result).ends_with("a shortfall of 4990100"));
    }

    // FX-11a and FX-11b (R14).
    #[test]
    fn at_the_boundary_remove_then_add_is_safe_and_add_then_remove_is_not() {
        let snapshot = snapshot_funded(1, &[(2, 1), (3, 1)], (1, 2, 2), (20_000_200, 0, 2));
        let remove_first = plan(vec![tx_with_fee(
            "tx1",
            &[1, 2],
            200,
            vec![signer_op(3, 0), signer_op(4, 1)],
        )]);
        let safe = evaluate(&snapshot, &remove_first, &everyone());
        assert_eq!(safe.verdict, Verdict::SafeWithinScope);
        assert_eq!(safe.steps[0].fee.map(|fee| fee.assumed), Some(false));

        let add_first = plan(vec![tx_with_fee(
            "tx1",
            &[1, 2],
            200,
            vec![signer_op(4, 1), signer_op(3, 0)],
        )]);
        let refused = evaluate(&snapshot, &add_first, &everyone());
        assert_eq!(refused.verdict, Verdict::Unsafe);
        assert!(matches!(
            refused.steps[0].result,
            StepResult::FailedToApply(ApplyFailure::LowReserve { operation: 0, .. })
        ));
    }

    // FX-12 (R10).
    #[test]
    fn re_weighting_needs_no_reserve() {
        let result = evaluate(
            &funded(15_000_100, 0),
            &plan(vec![tx("tx1", &[1, 2], vec![signer_op(2, 2)])]),
            &everyone(),
        );
        assert_eq!(result.verdict, Verdict::SafeWithinScope);
    }

    // FX-13.
    #[test]
    fn selling_liabilities_can_make_a_signer_unaffordable() {
        let result = evaluate(&funded(20_000_100, 1), &add_d(), &everyone());
        assert_eq!(result.verdict, Verdict::Unsafe);
        assert!(shortfall_of(&result).ends_with("a shortfall of 1"));
    }

    // FX-14 (R13).
    #[test]
    fn a_fee_that_cannot_be_paid_rejects_the_transaction_before_it_is_applied() {
        let reweight = plan(vec![tx("tx1", &[1, 2], vec![signer_op(2, 2)])]);
        let result = evaluate(&funded(15_000_099, 0), &reweight, &everyone());
        assert_eq!(result.verdict, Verdict::Unsafe);
        assert_eq!(
            result.steps[0].result,
            StepResult::Rejected(vec![Failure::FeeUnaffordable {
                fee: 100,
                available: 99
            }])
        );
        // Nothing was charged.
        assert_eq!(result.final_reserve.unwrap().balance(), 15_000_099);

        let exactly = evaluate(&funded(15_000_100, 0), &reweight, &everyone());
        assert_eq!(exactly.verdict, Verdict::SafeWithinScope);
    }

    // FX-15.
    #[test]
    fn a_signer_added_without_reserve_data_is_inconclusive_not_safe() {
        let result = evaluate(&snapshot(1, &[(2, 1)], (1, 2, 2)), &add_d(), &everyone());
        assert_eq!(result.verdict, Verdict::Inconclusive);
        assert_eq!(result.steps[0].result, StepResult::Accepted);
        assert_eq!(all_codes(&result), [Code::State01]);
        assert!(result.final_reserve.is_none());
        assert!(result.steps[0].fee.is_none());
    }

    #[test]
    fn without_reserve_data_a_loss_of_access_is_still_unsafe() {
        // The signer cannot be checked for affordability, but the same plan
        // raises HIGH above the total weight, and that is reported.
        let result = evaluate(
            &snapshot(1, &[(2, 1)], (1, 2, 2)),
            &plan(vec![tx(
                "tx1",
                &[1, 2],
                vec![signer_op(4, 1), json!({ "op": "set_options", "high": 9 })],
            )]),
            &everyone(),
        );
        assert_eq!(result.verdict, Verdict::Unsafe);
        assert!(all_codes(&result).contains(&Code::State01));
        assert!(all_codes(&result).contains(&Code::Lock01));
    }

    #[test]
    fn without_reserve_data_a_plan_that_adds_no_signer_is_unchanged() {
        let result = evaluate(
            &snapshot(1, &[(2, 1)], (1, 2, 2)),
            &plan(vec![tx("tx1", &[1, 2], vec![signer_op(2, 2)])]),
            &everyone(),
        );
        assert_eq!(result.verdict, Verdict::SafeWithinScope);
        assert!(all_codes(&result).is_empty());
    }

    // FX-16.
    #[test]
    fn the_sub_entry_limit_is_unsafe_with_the_apply_code() {
        let result = evaluate(
            &snapshot_funded(1, &[(2, 1)], (1, 2, 2), (100_000_000_000, 0, 1000)),
            &add_d(),
            &everyone(),
        );
        assert_eq!(result.verdict, Verdict::Unsafe);
        assert_eq!(all_codes(&result), [Code::Apply01]);
    }

    // FX-17 (R12): a transaction that fails when applied still pays its fee.
    #[test]
    fn a_failed_transaction_keeps_its_fee_and_can_starve_the_next_one() {
        let result = evaluate(
            &funded(15_000_150, 0),
            &plan(vec![
                tx("tx1", &[1, 2], vec![signer_op(4, 1)]),
                tx("tx2", &[1, 2], vec![signer_op(2, 2)]),
            ]),
            &everyone(),
        );
        assert_eq!(result.verdict, Verdict::Unsafe);
        assert!(matches!(
            result.steps[0].result,
            StepResult::FailedToApply(ApplyFailure::LowReserve { .. })
        ));
        assert_eq!(
            result.steps[1].result,
            StepResult::Rejected(vec![Failure::FeeUnaffordable {
                fee: 100,
                available: 50
            }])
        );
        assert_eq!(result.steps[1].reserve_before.unwrap().available(), 50);
    }

    #[test]
    fn a_rejected_transaction_costs_nothing_for_the_next_one() {
        // tx1 is signed by a key that is not a signer: rejected at submission,
        // so no fee is taken and tx2 sees the full balance.
        let result = evaluate(
            &funded(15_000_100, 0),
            &plan(vec![
                tx("tx1", &[3], vec![signer_op(2, 2)]),
                tx("tx2", &[1, 2], vec![signer_op(2, 2)]),
            ]),
            &everyone(),
        );
        assert!(matches!(result.steps[0].result, StepResult::Rejected(_)));
        assert_eq!(result.steps[1].result, StepResult::Accepted);
        assert!(result.steps[0].fee.is_none());
    }

    #[test]
    fn the_assumed_fee_is_100_stroops_for_each_operation() {
        let result = evaluate(
            &funded(30_000_000, 0),
            &plan(vec![tx(
                "tx1",
                &[1, 2],
                vec![signer_op(2, 2), signer_op(2, 3), signer_op(2, 4)],
            )]),
            &everyone(),
        );
        assert_eq!(
            result.steps[0].fee,
            Some(FeeUsed {
                amount: 300,
                assumed: true
            })
        );
        assert_eq!(result.final_reserve.unwrap().balance(), 30_000_000 - 300);
    }
}
