//! The plain-text report, `docs/SPEC.md`, section 8.
//!
//! The report is built only from validated values and fixed text, so it never
//! repeats anything taken from an input document that the tool did not parse.
//! It uses ASCII only and contains no timestamps of its own, so the same input
//! always gives the same bytes.

mod json;
mod rejected;

pub use json::{REPORT_SCHEMA, json_report};
pub use rejected::{RejectedDocument, rejected_json, rejected_text};

use crate::account::{AccountState, SignerKey};
use crate::authorize::{CheckResult, Trace};
use crate::diagnostic::Effect;
use crate::evaluate::{Evaluation, Finding, Reach, ScenarioResult, Step, StepResult, Verdict};
use crate::input::availability::{Availability, Intent};
use crate::input::plan::{Plan, SetOptions};
use crate::input::snapshot::Snapshot;

/// What the tool models.
pub(crate) const MODELLED: [&str; 7] = [
    "one classic account (a G address) per run",
    "ed25519 signers, including the account's own key",
    "SetOptions operations",
    "the authorization rules of protocol 29, checked against the state before each transaction",
    "the extra-signature rule, where it can be decided",
    "LOW, MEDIUM and HIGH reachability under the declared availability",
    "up to 20 additional signers, weights and thresholds from 0 to 255",
];

/// What the tool does not model. Each of these gives INCONCLUSIVE, not SAFE.
pub(crate) const NOT_MODELLED: [&str; 9] = [
    "whether anyone holds, or can use, any key",
    "fee-bump transactions and transaction preconditions",
    "source accounts other than the analysed account",
    "sponsorship and reserve requirements",
    "pre_auth_tx, hash(x) and signed-payload signers",
    "operations other than SetOptions",
    "the order of signatures, where it decides the outcome",
    "protocol versions other than 29",
    "changes to the account after the snapshot was taken",
];

fn put(out: &mut String, text: impl AsRef<str>) {
    out.push_str(text.as_ref());
    out.push('\n');
}

fn heading(out: &mut String, title: &str) {
    out.push('\n');
    put(out, title);
    put(out, "-".repeat(title.len()));
}

/// Renders the plain-text report.
pub fn text_report(
    snapshot: &Snapshot,
    plan: &Plan,
    availability: &Availability,
    evaluation: &Evaluation,
) -> String {
    let mut out = String::new();
    let provenance = snapshot.provenance();

    put(&mut out, "SIGNERREACH CHECK");
    put(&mut out, "=================");
    put(&mut out, "");
    for line in header_lines(snapshot) {
        put(&mut out, line);
    }

    out.push('\n');
    put(
        &mut out,
        format!("VERDICT: {}", evaluation.verdict.as_str()),
    );
    put(&mut out, format!("Reason: {}", reason(evaluation)));

    heading(&mut out, "Provenance");
    put(
        &mut out,
        format!("  network       {}", snapshot.network().as_str()),
    );
    put(
        &mut out,
        format!("  account       {}", snapshot.state().account()),
    );
    put(
        &mut out,
        format!("  as of ledger  {}", provenance.as_of_ledger()),
    );
    put(
        &mut out,
        format!("  captured at   {}", provenance.captured_at()),
    );
    put(
        &mut out,
        format!("  source        {}", provenance.source().as_str()),
    );
    put(
        &mut out,
        format!("  protocol      {}", provenance.protocol_version()),
    );

    heading(&mut out, "Starting state");
    state_block(&mut out, snapshot.state());

    heading(&mut out, "Assumptions");
    assumptions(&mut out, availability);

    heading(&mut out, "Plan");
    if plan.transactions().is_empty() {
        put(
            &mut out,
            "  No transactions: this is a check of the snapshot itself.",
        );
    }
    for (transaction, step) in plan.transactions().iter().zip(&evaluation.steps) {
        out.push('\n');
        put(&mut out, format!("  Transaction {}", step.id));
        for (index, operation) in transaction.operations().iter().enumerate() {
            put(
                &mut out,
                format!("    operation {index}: {}", describe(operation)),
            );
        }
        step_block(&mut out, transaction.signed_by(), step);
    }
    if !plan.transactions().is_empty() {
        out.push('\n');
        put(&mut out, "  State after the plan");
        state_block_indented(&mut out, &evaluation.final_state, "    ");
    }

    heading(&mut out, "Reachability");
    if evaluation.scenarios.is_empty() {
        put(
            &mut out,
            "  Not evaluated: a transaction could not be decided, so the state after the plan is \
             not known.",
        );
    }
    for scenario in &evaluation.scenarios {
        out.push('\n');
        scenario_block(&mut out, scenario);
    }

    heading(&mut out, "Diagnostics");
    diagnostics(&mut out, evaluation);

    heading(&mut out, "Coverage");
    put(&mut out, "  Modelled:");
    for item in MODELLED {
        put(&mut out, format!("    - {item}"));
    }
    put(
        &mut out,
        "  Not modelled (any of these gives INCONCLUSIVE):",
    );
    for item in NOT_MODELLED {
        put(&mut out, format!("    - {item}"));
    }
    out
}

/// The three limitations printed at the top of every report (spec 8.1).
pub(crate) fn header_lines(snapshot: &Snapshot) -> [String; 3] {
    [
        "Availability is declared by the user. This tool does not and cannot verify that anyone \
         holds any key."
            .to_string(),
        format!(
            "The result applies to the account state at ledger {} on {}. Authorization happens \
             at apply time against the live state, which may have changed.",
            snapshot.provenance().as_of_ledger(),
            snapshot.network().as_str()
        ),
        "Only the features listed under \"Coverage\" are modelled. Anything else is reported \
         INCONCLUSIVE."
            .to_string(),
    ]
}

/// The single most important reason for the verdict.
pub(crate) fn reason(evaluation: &Evaluation) -> String {
    let wanted = match evaluation.verdict {
        Verdict::SafeWithinScope => {
            return "a path to administer the account remains under the declared assumptions, in \
                    every scenario checked"
                .to_string();
        }
        Verdict::Inconclusive => Effect::Inconclusive,
        Verdict::Unsafe => Effect::Unsafe,
    };
    let scenario_findings = evaluation
        .scenarios
        .iter()
        .flat_map(|scenario| scenario.findings.iter());
    evaluation
        .plan_findings
        .iter()
        .chain(scenario_findings)
        .find(|finding| finding.code.effect() == wanted)
        .map_or_else(
            || "see the diagnostics".to_string(),
            |finding| match &finding.scenario {
                Some(name) => format!("{} (scenario {name})", finding.message),
                None => finding.message.clone(),
            },
        )
}

fn thresholds_line(state: &AccountState) -> String {
    use crate::account::ThresholdClass::{High, Low, Medium};
    format!(
        "low {}, medium {}, high {}",
        state.threshold(Low).get(),
        state.threshold(Medium).get(),
        state.threshold(High).get()
    )
}

fn state_block(out: &mut String, state: &AccountState) {
    state_block_indented(out, state, "  ");
}

fn state_block_indented(out: &mut String, state: &AccountState, indent: &str) {
    put(
        out,
        format!("{indent}master weight  {}", state.master_weight().get()),
    );
    put(
        out,
        format!("{indent}thresholds     {}", thresholds_line(state)),
    );
    if state.additional_signer_count() == 0 {
        put(out, format!("{indent}other signers  none"));
    } else {
        for (key, weight) in state.signers() {
            put(
                out,
                format!("{indent}signer         {key} weight {}", weight.get()),
            );
        }
    }
}

fn assumptions(out: &mut String, availability: &Availability) {
    put(
        out,
        format!(
            "  default status           {}",
            availability.default_status().as_str()
        ),
    );
    let mut declared = availability.signers().peekable();
    if declared.peek().is_none() {
        put(out, "  declared signers         none");
    }
    for (key, status) in declared {
        put(
            out,
            format!("  declared                 {key} {}", status.as_str()),
        );
    }
    if availability.scenarios().is_empty() {
        put(out, "  scenarios                none");
    }
    for scenario in availability.scenarios() {
        let overrides: Vec<String> = scenario
            .overrides()
            .map(|(key, status)| format!("{key} {}", status.as_str()))
            .collect();
        put(
            out,
            format!(
                "  scenario {}: {}",
                scenario.name(),
                if overrides.is_empty() {
                    "no overrides".to_string()
                } else {
                    overrides.join(", ")
                }
            ),
        );
    }
    put(
        out,
        format!(
            "  survive loss of          {} signer(s)",
            availability.require_survive_loss_of()
        ),
    );
    put(
        out,
        format!(
            "  intent                   {}",
            availability.intent().as_str()
        ),
    );
    if availability.intent() == Intent::Irreversible {
        put(
            out,
            "  Irreversible administrative state, declared by the user. The result below is \
             unchanged by this declaration.",
        );
    }
}

fn describe(operation: &SetOptions) -> String {
    let mut parts = Vec::new();
    if let Some(weight) = operation.master_weight() {
        parts.push(format!("set master weight to {}", weight.get()));
    }
    for (name, value) in [
        ("low", operation.low()),
        ("medium", operation.medium()),
        ("high", operation.high()),
    ] {
        if let Some(threshold) = value {
            parts.push(format!("set {name} threshold to {}", threshold.get()));
        }
    }
    if let Some(change) = operation.signer() {
        if change.weight().is_zero() {
            parts.push(format!("remove signer {}", change.key()));
        } else {
            parts.push(format!(
                "set signer {} to weight {}",
                change.key(),
                change.weight().get()
            ));
        }
    }
    if let Some(domain) = operation.home_domain() {
        parts.push(format!("set home domain {domain}"));
    }
    if let Some(flags) = operation.set_flags() {
        parts.push(format!("set account flags {flags}"));
    }
    if let Some(flags) = operation.clear_flags() {
        parts.push(format!("clear account flags {flags}"));
    }
    if parts.is_empty() {
        "SetOptions with no changes".to_string()
    } else {
        parts.join("; ")
    }
}

fn step_block(out: &mut String, signed_by: &[SignerKey], step: &Step) {
    if let Some(before) = &step.before {
        put(out, "    state before:");
        state_block_indented(out, before, "      ");
    }
    if let Some(trace) = &step.trace {
        trace_block(out, signed_by, trace);
    }
    let result = match &step.result {
        StepResult::Accepted => "accepted".to_string(),
        StepResult::Rejected(failures) => {
            let codes: Vec<&str> = failures.iter().map(|f| f.code().as_str()).collect();
            format!(
                "REJECTED by the network, nothing changes ({})",
                codes.join(", ")
            )
        }
        StepResult::FailedToApply(failure) => format!(
            "authorized but fails when applied, nothing changes ({})",
            failure.code()
        ),
        StepResult::Undecided(doubts) => {
            let codes: Vec<&str> = doubts.iter().map(|d| d.code().as_str()).collect();
            format!(
                "UNDECIDED ({}); later transactions are not evaluated",
                codes.join(", ")
            )
        }
        StepResult::NotEvaluated => {
            "not evaluated: an earlier transaction was undecided".to_string()
        }
    };
    put(out, format!("    result: {result}"));
}

fn trace_block(out: &mut String, signed_by: &[SignerKey], trace: &Trace) {
    if signed_by.is_empty() {
        put(out, "    signed by: nobody");
    }
    for key in signed_by {
        match trace.matched.iter().find(|(matched, _)| matched == key) {
            Some((_, weight)) => put(
                out,
                format!("    signed by: {key} (weight {})", weight.get()),
            ),
            None => put(
                out,
                format!("    signed by: {key} (not a signer of this state)"),
            ),
        }
    }
    for check in &trace.checks {
        put(out, format!("    {}", check_line(check)));
    }
}

fn check_line(check: &CheckResult) -> String {
    let what = match check.operation {
        None => format!("envelope [{}]", check.class),
        Some(index) => format!("operation {index} [{}]", check.class),
    };
    format!(
        "check {what}: needs {}, signatures give {}: {}",
        check.need,
        check.have,
        if check.passed { "met" } else { "NOT MET" }
    )
}

fn reach_words(reach: Reach) -> &'static str {
    match reach {
        Reach::Reachable => "reachable",
        Reach::DependsOnUnknown => "depends on signers whose availability is unknown",
        Reach::Lost => "NOT reachable: the signers that could meet it are declared lost",
        Reach::Structural => "NOT reachable: all signers together are too light",
    }
}

fn scenario_block(out: &mut String, scenario: &ScenarioResult) {
    let title = match &scenario.name {
        None => "Baseline declaration".to_string(),
        Some(name) => format!("Scenario {name}"),
    };
    put(out, format!("  {title}: {}", scenario.verdict.as_str()));
    for signer in &scenario.signers {
        put(
            out,
            format!(
                "    signer {} weight {} {}",
                signer.key,
                signer.weight,
                signer.status.as_str()
            ),
        );
    }
    if scenario.signers.is_empty() {
        put(out, "    no signer has a weight above 0");
    }
    for reach in &scenario.reach {
        put(
            out,
            format!(
                "    {:<6} threshold {:>3}  total {}  available {}  unknown {}: {}",
                reach.class.to_string(),
                reach.threshold,
                reach.total,
                reach.available,
                reach.unknown,
                reach_words(reach.reach)
            ),
        );
    }
}

pub(crate) fn effect_label(effect: Effect) -> &'static str {
    match effect {
        Effect::Unsafe => "UNSAFE",
        Effect::Inconclusive => "INCONCLUSIVE",
        Effect::Info => "info",
    }
}

/// Every finding with the scope it applies to: `plan`, `baseline`, or a
/// scenario name. Plan findings come first, then each scenario in order.
pub(crate) fn scoped_findings(evaluation: &Evaluation) -> Vec<(String, &Finding)> {
    let mut lines: Vec<(String, &Finding)> = Vec::new();
    for finding in &evaluation.plan_findings {
        lines.push(("plan".to_string(), finding));
    }
    for scenario in &evaluation.scenarios {
        let scope = scenario
            .name
            .clone()
            .unwrap_or_else(|| "baseline".to_string());
        for finding in &scenario.findings {
            lines.push((scope.clone(), finding));
        }
    }
    lines
}

fn diagnostics(out: &mut String, evaluation: &Evaluation) {
    let lines = scoped_findings(evaluation);
    if lines.is_empty() {
        put(out, "  none");
    }
    for (scope, finding) in lines {
        put(
            out,
            format!(
                "  {} [{}] ({scope}) {}",
                finding.code,
                effect_label(finding.code.effect()),
                finding.message
            ),
        );
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::evaluate::evaluate;
    use crate::testing::{availability, availability_with, k, plan, signer_op, snapshot, tx};

    fn report(snapshot: &Snapshot, plan: &Plan, availability: &Availability) -> String {
        let evaluation = evaluate(snapshot, plan, availability);
        text_report(snapshot, plan, availability, &evaluation)
    }

    fn has_line(text: &str, wanted: &str) -> bool {
        text.lines().any(|line| line.trim() == wanted)
    }

    fn contains(text: &str, wanted: &str) -> bool {
        text.lines().any(|line| line.contains(wanted))
    }

    // FX-01.
    fn unsafe_report() -> String {
        report(
            &snapshot(1, &[(2, 1)], (1, 1, 1)),
            &plan(vec![tx(
                "tx1",
                &[1],
                vec![json!({ "op": "set_options", "high": 2 })],
            )]),
            &availability(&[(1, "available"), (2, "lost")]),
        )
    }

    #[test]
    fn the_fixed_header_comes_first_with_ledger_and_network_filled_in() {
        let text = unsafe_report();
        let all: Vec<&str> = text.lines().collect();
        assert_eq!(all[0], "SIGNERREACH CHECK");
        assert_eq!(
            all[3],
            "Availability is declared by the user. This tool does not and cannot verify that \
             anyone holds any key."
        );
        assert_eq!(
            all[4],
            "The result applies to the account state at ledger 100 on testnet. Authorization \
             happens at apply time against the live state, which may have changed."
        );
        assert_eq!(
            all[5],
            "Only the features listed under \"Coverage\" are modelled. Anything else is \
             reported INCONCLUSIVE."
        );
    }

    #[test]
    fn the_verdict_and_reason_follow_the_header() {
        let text = unsafe_report();
        assert!(has_line(&text, "VERDICT: UNSAFE"));
        assert!(contains(
            &text,
            "Reason: signers declared available weigh 1, below the HIGH threshold 2"
        ));
    }

    #[test]
    fn the_trace_shows_weights_thresholds_and_each_result() {
        let text = unsafe_report();
        assert!(contains(&text, "signed by: "));
        assert!(contains(&text, "(weight 1)"));
        assert!(contains(
            &text,
            "check envelope [LOW]: needs 1, signatures give 1: met"
        ));
        assert!(contains(
            &text,
            "check operation 0 [HIGH]: needs 1, signatures give 1: met"
        ));
        assert!(has_line(&text, "result: accepted"));
        assert!(contains(&text, "operation 0: set high threshold to 2"));
    }

    #[test]
    fn reachability_lists_the_signers_and_each_class() {
        let text = unsafe_report();
        assert!(has_line(&text, "Baseline declaration: UNSAFE"));
        assert!(contains(&text, "weight 1 available"));
        assert!(contains(&text, "weight 1 lost"));
        assert!(contains(
            &text,
            "HIGH   threshold   2  total 2  available 1  unknown 0: NOT reachable: the signers \
             that could meet it are declared lost"
        ));
    }

    #[test]
    fn diagnostics_and_coverage_are_always_present() {
        let text = unsafe_report();
        assert!(contains(&text, "SR-LOSS-01 [UNSAFE] (baseline)"));
        assert!(has_line(&text, "Modelled:"));
        assert!(has_line(
            &text,
            "Not modelled (any of these gives INCONCLUSIVE):"
        ));
        assert!(contains(&text, "whether anyone holds, or can use, any key"));
    }

    #[test]
    fn a_safe_result_says_which_assumptions_it_rests_on() {
        let text = report(
            &snapshot(1, &[(2, 1)], (1, 1, 1)),
            &plan(vec![]),
            &availability(&[(1, "available"), (2, "available")]),
        );
        assert!(has_line(&text, "VERDICT: SAFE WITHIN SCOPE"));
        assert!(contains(
            &text,
            "Reason: a path to administer the account remains"
        ));
        assert!(has_line(
            &text,
            "No transactions: this is a check of the snapshot itself."
        ));
        assert!(has_line(&text, "none"));
        assert!(contains(&text, "declared "));
    }

    #[test]
    fn a_rejected_transaction_names_the_failing_check_and_the_extra_signature() {
        let text = report(
            &snapshot(1, &[(2, 1)], (1, 2, 2)),
            &plan(vec![tx(
                "bad",
                &[1, 9],
                vec![json!({ "op": "set_options", "high": 0 })],
            )]),
            &availability(&[(1, "available"), (2, "available")]),
        );
        assert!(contains(&text, "(not a signer of this state)"));
        assert!(contains(
            &text,
            "check operation 0 [HIGH]: needs 2, signatures give 1: NOT MET"
        ));
        assert!(contains(
            &text,
            "result: REJECTED by the network, nothing changes (SR-TX-01, SR-AUTH-01)"
        ));
    }

    #[test]
    fn a_two_transaction_plan_shows_both_and_the_state_after() {
        let text = report(
            &snapshot(1, &[(2, 1)], (1, 2, 2)),
            &plan(vec![
                tx("first", &[1, 2], vec![signer_op(2, 0)]),
                tx(
                    "second",
                    &[1],
                    vec![json!({ "op": "set_options", "master_weight": 2 })],
                ),
            ]),
            &availability(&[(1, "available"), (2, "available")]),
        );
        assert!(has_line(&text, "Transaction first"));
        assert!(has_line(&text, "Transaction second"));
        assert!(contains(
            &text,
            &format!("operation 0: remove signer {}", k(2))
        ));
        assert!(has_line(&text, "State after the plan"));
        assert!(contains(&text, "SR-AUTH-01 [UNSAFE] (plan)"));
    }

    #[test]
    fn an_undecided_transaction_stops_the_report_honestly() {
        let text = report(
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
        );
        assert!(has_line(&text, "VERDICT: INCONCLUSIVE"));
        assert!(contains(
            &text,
            "UNDECIDED (SR-TX-02); later transactions are not evaluated"
        ));
        assert!(contains(
            &text,
            "result: not evaluated: an earlier transaction was undecided"
        ));
        assert!(contains(
            &text,
            "Not evaluated: a transaction could not be decided"
        ));
    }

    #[test]
    fn scenarios_and_assumptions_are_listed() {
        let scenarios = json!([{ "name": "A-lost", "overrides": { k(1): "lost" } }]);
        let text = report(
            &snapshot(1, &[(2, 1), (3, 1)], (1, 2, 2)),
            &plan(vec![]),
            &availability_with(
                &[(1, "available"), (2, "available"), (3, "available")],
                scenarios,
                1,
                "irreversible",
            ),
        );
        assert!(contains(&text, "scenario A-lost: "));
        assert!(has_line(&text, "Scenario A-lost: UNSAFE"));
        assert!(contains(&text, "survive loss of          1 signer(s)"));
        assert!(contains(&text, "intent                   irreversible"));
        assert!(contains(
            &text,
            "Irreversible administrative state, declared by the user"
        ));
        assert!(contains(&text, "SR-INTENT-01 [info] (plan)"));
    }

    #[test]
    fn the_report_is_plain_ascii_without_tabs_and_ends_with_a_newline() {
        let text = unsafe_report();
        assert!(text.is_ascii());
        assert!(!text.contains('\t'));
        assert!(text.ends_with('\n'));
        assert!(!text.contains("\n\n\n"));
    }

    #[test]
    fn the_same_input_gives_the_same_bytes() {
        assert_eq!(unsafe_report(), unsafe_report());
    }
}
