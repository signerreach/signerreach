//! The JSON report, `signerreach.report/1`, `docs/SPEC.md`, section 8.3.
//!
//! The document is built only from validated values and fixed text. Object keys
//! are written in alphabetical order, arrays in the order documented below,
//! and the output holds no clock reading, so the same input always gives the
//! same bytes.
//!
//! Top-level fields: `schema`, `tool`, `verdict`, `reason`, `limitations`,
//! `provenance`, `starting_state`, `assumptions`, `transactions`, `state_after`,
//! `scenarios`, `diagnostics`, `coverage`.

use serde_json::{Map, Value, json};

use super::{MODELLED, NOT_MODELLED, header_lines, reason, scoped_findings};
use crate::account::{AccountState, SignerKey};
use crate::authorize::Trace;
use crate::diagnostic::Effect;
use crate::evaluate::{Evaluation, Reach, ScenarioResult, Step, StepResult};
use crate::input::availability::{Availability, Status};
use crate::input::plan::{Plan, SetOptions};
use crate::input::snapshot::Snapshot;
use crate::rules::required_class;

/// The schema identifier of the JSON report.
pub const REPORT_SCHEMA: &str = "signerreach.report/1";

/// Renders the JSON report: indented, with a final newline.
pub fn json_report(
    snapshot: &Snapshot,
    plan: &Plan,
    availability: &Availability,
    evaluation: &Evaluation,
) -> String {
    let provenance = snapshot.provenance();
    let transactions: Vec<Value> = plan
        .transactions()
        .iter()
        .zip(&evaluation.steps)
        .map(|(transaction, step)| {
            transaction_json(transaction.signed_by(), transaction.operations(), step)
        })
        .collect();

    let value = json!({
        "schema": REPORT_SCHEMA,
        "tool": { "name": "signerreach", "version": env!("CARGO_PKG_VERSION") },
        "verdict": evaluation.verdict.as_str(),
        "reason": reason(evaluation),
        "limitations": header_lines(snapshot),
        "provenance": {
            "network": snapshot.network().as_str(),
            "account": snapshot.state().account().to_string(),
            "as_of_ledger": provenance.as_of_ledger(),
            "captured_at": provenance.captured_at(),
            "source": provenance.source().as_str(),
            "protocol_version": provenance.protocol_version(),
        },
        "starting_state": state_json(snapshot.state()),
        "assumptions": assumptions_json(availability),
        "transactions": transactions,
        "state_after": state_json(&evaluation.final_state),
        "scenarios": evaluation.scenarios.iter().map(scenario_json).collect::<Vec<_>>(),
        "diagnostics": diagnostics_json(evaluation),
        "coverage": { "modelled": MODELLED, "not_modelled": NOT_MODELLED },
    });

    format!("{value:#}\n")
}

fn state_json(state: &AccountState) -> Value {
    use crate::account::ThresholdClass::{High, Low, Medium};
    json!({
        "account": state.account().to_string(),
        "master_weight": state.master_weight().get(),
        "thresholds": {
            "low": state.threshold(Low).get(),
            "medium": state.threshold(Medium).get(),
            "high": state.threshold(High).get(),
        },
        "signers": state
            .signers()
            .map(|(key, weight)| json!({ "key": key.to_string(), "weight": weight.get() }))
            .collect::<Vec<_>>(),
    })
}

fn status_json(key: SignerKey, status: Status) -> Value {
    json!({ "key": key.to_string(), "status": status.as_str() })
}

fn assumptions_json(availability: &Availability) -> Value {
    json!({
        "default": availability.default_status().as_str(),
        "signers": availability
            .signers()
            .map(|(key, status)| status_json(key, status))
            .collect::<Vec<_>>(),
        "scenarios": availability
            .scenarios()
            .iter()
            .map(|scenario| json!({
                "name": scenario.name(),
                "overrides": scenario
                    .overrides()
                    .map(|(key, status)| status_json(key, status))
                    .collect::<Vec<_>>(),
            }))
            .collect::<Vec<_>>(),
        "require_survive_loss_of": availability.require_survive_loss_of(),
        "intent": availability.intent().as_str(),
    })
}

fn operation_json(index: usize, operation: &SetOptions) -> Value {
    let mut fields = Map::new();
    fields.insert("index".to_string(), json!(index));
    fields.insert(
        "required_class".to_string(),
        json!(required_class(operation).to_string()),
    );
    if let Some(weight) = operation.master_weight() {
        fields.insert("master_weight".to_string(), json!(weight.get()));
    }
    for (name, value) in [
        ("low", operation.low()),
        ("medium", operation.medium()),
        ("high", operation.high()),
    ] {
        if let Some(threshold) = value {
            fields.insert(name.to_string(), json!(threshold.get()));
        }
    }
    if let Some(change) = operation.signer() {
        fields.insert(
            "signer".to_string(),
            json!({ "key": change.key().to_string(), "weight": change.weight().get() }),
        );
    }
    if let Some(domain) = operation.home_domain() {
        fields.insert("home_domain".to_string(), json!(domain));
    }
    if let Some(flags) = operation.set_flags() {
        fields.insert("set_flags".to_string(), json!(flags));
    }
    if let Some(flags) = operation.clear_flags() {
        fields.insert("clear_flags".to_string(), json!(flags));
    }
    Value::Object(fields)
}

fn signed_by_json(signed_by: &[SignerKey], trace: Option<&Trace>) -> Vec<Value> {
    signed_by
        .iter()
        .map(|key| {
            let weight = trace.and_then(|trace| {
                trace
                    .matched
                    .iter()
                    .find(|(matched, _)| matched == key)
                    .map(|(_, weight)| weight.get())
            });
            // `matched` and `weight` are null when the transaction was not evaluated.
            let matched = trace.map(|_| weight.is_some());
            json!({ "key": key.to_string(), "matched": matched, "weight": weight })
        })
        .collect()
}

fn transaction_json(signed_by: &[SignerKey], operations: &[SetOptions], step: &Step) -> Value {
    let (result, codes): (&str, Vec<&str>) = match &step.result {
        StepResult::Accepted => ("accepted", vec![]),
        StepResult::Rejected(failures) => (
            "rejected",
            failures.iter().map(|f| f.code().as_str()).collect(),
        ),
        StepResult::FailedToApply(failure) => ("failed_to_apply", vec![failure.code().as_str()]),
        StepResult::Undecided(doubts) => (
            "undecided",
            doubts.iter().map(|d| d.code().as_str()).collect(),
        ),
        StepResult::NotEvaluated => ("not_evaluated", vec![]),
    };
    let checks: Vec<Value> = step.trace.as_ref().map_or_else(Vec::new, |trace| {
        trace
            .checks
            .iter()
            .map(|check| {
                json!({
                    "operation": check.operation,
                    "class": check.class.to_string(),
                    "need": check.need,
                    "have": check.have,
                    "passed": check.passed,
                })
            })
            .collect()
    });
    json!({
        "id": step.id,
        "operations": operations
            .iter()
            .enumerate()
            .map(|(index, operation)| operation_json(index, operation))
            .collect::<Vec<_>>(),
        "state_before": step.before.as_ref().map(state_json),
        "signed_by": signed_by_json(signed_by, step.trace.as_ref()),
        "checks": checks,
        "result": result,
        "codes": codes,
    })
}

fn reach_json(reach: Reach) -> &'static str {
    match reach {
        Reach::Reachable => "reachable",
        Reach::DependsOnUnknown => "depends_on_unknown",
        Reach::Lost => "lost",
        Reach::Structural => "structural",
    }
}

fn scenario_json(scenario: &ScenarioResult) -> Value {
    json!({
        "name": scenario.name,
        "verdict": scenario.verdict.as_str(),
        "signers": scenario
            .signers
            .iter()
            .map(|signer| json!({
                "key": signer.key.to_string(),
                "weight": signer.weight,
                "status": signer.status.as_str(),
            }))
            .collect::<Vec<_>>(),
        "reachability": scenario
            .reach
            .iter()
            .map(|reach| json!({
                "class": reach.class.to_string(),
                "threshold": reach.threshold,
                "total": reach.total,
                "available": reach.available,
                "unknown": reach.unknown,
                "reach": reach_json(reach.reach),
            }))
            .collect::<Vec<_>>(),
    })
}

fn diagnostics_json(evaluation: &Evaluation) -> Vec<Value> {
    scoped_findings(evaluation)
        .into_iter()
        .map(|(scope, finding)| {
            let effect = match finding.code.effect() {
                Effect::Unsafe => "unsafe",
                Effect::Inconclusive => "inconclusive",
                Effect::Info => "info",
            };
            json!({
                "code": finding.code.as_str(),
                "effect": effect,
                "scope": scope,
                "message": finding.message,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::evaluate::evaluate;
    use crate::input::{Json, MAX_INPUT_BYTES};
    use crate::report::text_report;
    use crate::testing::{availability, availability_with, k, plan, snapshot, tx};

    fn render(snapshot: &Snapshot, plan: &Plan, availability: &Availability) -> String {
        let evaluation = evaluate(snapshot, plan, availability);
        json_report(snapshot, plan, availability, &evaluation)
    }

    fn parsed(text: &str) -> Value {
        serde_json::from_str(text).expect("the report is valid JSON")
    }

    // FX-01.
    fn unsafe_text() -> String {
        render(
            &snapshot(1, &[(2, 1)], (1, 1, 1)),
            &plan(vec![tx(
                "tx1",
                &[1],
                vec![json!({ "op": "set_options", "high": 2 })],
            )]),
            &availability(&[(1, "available"), (2, "lost")]),
        )
    }

    /// Every object's keys, in the order they appear in the text.
    fn assert_keys_sorted(value: &Json) {
        match value {
            Json::Object(pairs) => {
                let keys: Vec<&String> = pairs.iter().map(|(key, _)| key).collect();
                let mut sorted = keys.clone();
                sorted.sort();
                assert_eq!(keys, sorted, "keys are not alphabetical");
                pairs
                    .iter()
                    .for_each(|(_, child)| assert_keys_sorted(child));
            }
            Json::Array(items) => items.iter().for_each(assert_keys_sorted),
            _ => {}
        }
    }

    #[test]
    fn the_report_is_valid_json_with_the_documented_top_level_fields() {
        let report = parsed(&unsafe_text());
        let mut names: Vec<&str> = report
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        names.sort_unstable();
        assert_eq!(
            names,
            [
                "assumptions",
                "coverage",
                "diagnostics",
                "limitations",
                "provenance",
                "reason",
                "scenarios",
                "schema",
                "starting_state",
                "state_after",
                "tool",
                "transactions",
                "verdict",
            ]
        );
        assert_eq!(report["schema"], "signerreach.report/1");
        assert_eq!(report["tool"]["name"], "signerreach");
        assert_eq!(report["verdict"], "UNSAFE");
        assert_eq!(
            report["reason"],
            "signers declared available weigh 1, below the HIGH threshold 2"
        );
    }

    #[test]
    fn provenance_and_states_are_reported() {
        let report = parsed(&unsafe_text());
        assert_eq!(report["provenance"]["network"], "testnet");
        assert_eq!(report["provenance"]["as_of_ledger"], 100);
        assert_eq!(report["provenance"]["protocol_version"], 29);
        assert_eq!(report["provenance"]["account"], k(1));
        assert_eq!(report["starting_state"]["thresholds"]["high"], 1);
        assert_eq!(report["state_after"]["thresholds"]["high"], 2);
        assert_eq!(report["starting_state"]["signers"][0]["key"], k(2));
    }

    #[test]
    fn a_transaction_carries_its_operations_trace_and_result() {
        let report = parsed(&unsafe_text());
        let tx = &report["transactions"][0];
        assert_eq!(tx["id"], "tx1");
        assert_eq!(tx["result"], "accepted");
        assert_eq!(tx["codes"], json!([]));
        assert_eq!(tx["operations"][0]["index"], 0);
        assert_eq!(tx["operations"][0]["required_class"], "HIGH");
        assert_eq!(tx["operations"][0]["high"], 2);
        assert_eq!(tx["signed_by"][0]["key"], k(1));
        assert_eq!(tx["signed_by"][0]["matched"], true);
        assert_eq!(tx["signed_by"][0]["weight"], 1);
        assert_eq!(tx["checks"][0]["operation"], Value::Null);
        assert_eq!(tx["checks"][0]["class"], "LOW");
        assert_eq!(tx["checks"][1]["operation"], 0);
        assert_eq!(
            tx["checks"][1],
            json!({ "operation": 0, "class": "HIGH", "need": 1, "have": 1, "passed": true })
        );
        assert_eq!(tx["state_before"]["thresholds"]["high"], 1);
    }

    #[test]
    fn reachability_and_scenarios_are_reported() {
        let report = parsed(&unsafe_text());
        let scenario = &report["scenarios"][0];
        assert_eq!(scenario["name"], Value::Null);
        assert_eq!(scenario["verdict"], "UNSAFE");
        assert_eq!(scenario["signers"][1]["status"], "lost");
        assert_eq!(
            scenario["reachability"][2],
            json!({
                "class": "HIGH", "threshold": 2, "total": 2,
                "available": 1, "unknown": 0, "reach": "lost"
            })
        );
    }

    #[test]
    fn diagnostics_carry_code_effect_scope_and_message() {
        let report = parsed(&unsafe_text());
        assert_eq!(
            report["diagnostics"],
            json!([{
                "code": "SR-LOSS-01",
                "effect": "unsafe",
                "scope": "baseline",
                "message": "signers declared available weigh 1, below the HIGH threshold 2"
            }])
        );
    }

    #[test]
    fn a_rejected_transaction_marks_the_extra_signature_and_lists_its_codes() {
        let report = parsed(&render(
            &snapshot(1, &[(2, 1)], (1, 2, 2)),
            &plan(vec![tx(
                "bad",
                &[1, 9],
                vec![json!({ "op": "set_options", "high": 0 })],
            )]),
            &availability(&[(1, "available"), (2, "available")]),
        ));
        let tx = &report["transactions"][0];
        assert_eq!(tx["result"], "rejected");
        assert_eq!(tx["codes"], json!(["SR-TX-01", "SR-AUTH-01"]));
        assert_eq!(tx["signed_by"][1]["matched"], false);
        assert_eq!(tx["signed_by"][1]["weight"], Value::Null);
        assert_eq!(tx["checks"][1]["passed"], false);
        assert_eq!(report["verdict"], "UNSAFE");
    }

    #[test]
    fn an_undecided_transaction_leaves_later_ones_unevaluated_and_scenarios_empty() {
        let report = parsed(&render(
            &snapshot(2, &[(2, 1)], (1, 2, 2)),
            &plan(vec![
                tx(
                    "first",
                    &[1, 2],
                    vec![json!({ "op": "set_options", "home_domain": "example.org" })],
                ),
                tx("second", &[1], vec![json!({ "op": "set_options" })]),
            ]),
            &availability(&[(1, "available"), (2, "available")]),
        ));
        assert_eq!(report["verdict"], "INCONCLUSIVE");
        assert_eq!(report["transactions"][0]["result"], "undecided");
        let second = &report["transactions"][1];
        assert_eq!(second["result"], "not_evaluated");
        assert_eq!(second["state_before"], Value::Null);
        assert_eq!(second["checks"], json!([]));
        assert_eq!(second["signed_by"][0]["matched"], Value::Null);
        assert_eq!(report["scenarios"], json!([]));
    }

    #[test]
    fn an_empty_plan_has_no_transactions_and_state_after_equals_the_start() {
        let report = parsed(&render(
            &snapshot(1, &[], (1, 1, 1)),
            &plan(vec![]),
            &availability(&[(1, "available")]),
        ));
        assert_eq!(report["transactions"], json!([]));
        assert_eq!(report["state_after"], report["starting_state"]);
        assert_eq!(report["verdict"], "SAFE WITHIN SCOPE");
    }

    #[test]
    fn assumptions_list_the_declaration_as_given() {
        let scenarios = json!([{ "name": "A-lost", "overrides": { k(1): "lost" } }]);
        let report = parsed(&render(
            &snapshot(1, &[(2, 1)], (1, 1, 1)),
            &plan(vec![]),
            &availability_with(
                &[(1, "available"), (2, "unknown")],
                scenarios,
                1,
                "irreversible",
            ),
        ));
        let assumptions = &report["assumptions"];
        assert_eq!(assumptions["default"], "unknown");
        assert_eq!(assumptions["intent"], "irreversible");
        assert_eq!(assumptions["require_survive_loss_of"], 1);
        assert_eq!(assumptions["scenarios"][0]["name"], "A-lost");
        assert_eq!(
            assumptions["scenarios"][0]["overrides"][0],
            json!({ "key": k(1), "status": "lost" })
        );
        assert_eq!(report["diagnostics"][0]["code"], "SR-INTENT-01");
        assert_eq!(report["diagnostics"][0]["effect"], "info");
        assert_eq!(report["diagnostics"][0]["scope"], "plan");
    }

    #[test]
    fn the_limitations_and_coverage_match_the_text_report() {
        let snap = snapshot(1, &[(2, 1)], (1, 1, 1));
        let pl = plan(vec![]);
        let av = availability(&[(1, "available"), (2, "available")]);
        let evaluation = evaluate(&snap, &pl, &av);
        let report = parsed(&json_report(&snap, &pl, &av, &evaluation));
        let text = text_report(&snap, &pl, &av, &evaluation);
        for line in report["limitations"].as_array().unwrap() {
            assert!(text.contains(line.as_str().unwrap()));
        }
        for item in report["coverage"]["modelled"].as_array().unwrap() {
            assert!(text.contains(item.as_str().unwrap()));
        }
        for item in report["coverage"]["not_modelled"].as_array().unwrap() {
            assert!(text.contains(item.as_str().unwrap()));
        }
        assert_eq!(report["limitations"].as_array().unwrap().len(), 3);
    }

    #[test]
    fn keys_are_alphabetical_at_every_level() {
        let text = unsafe_text();
        assert!(text.len() < MAX_INPUT_BYTES);
        let tree: Json = serde_json::from_str(&text).expect("valid JSON");
        assert_keys_sorted(&tree);
    }

    #[test]
    fn the_output_is_ascii_ends_with_a_newline_and_is_reproducible() {
        let text = unsafe_text();
        assert!(text.is_ascii());
        assert!(text.ends_with("}\n"));
        assert_eq!(text, unsafe_text());
    }
}
