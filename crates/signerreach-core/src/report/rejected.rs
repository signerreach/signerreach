//! The report for input that was rejected before anything was evaluated.
//!
//! Spec section 6.1: an input problem makes the result INCONCLUSIVE. The
//! problems are listed with their codes and locations. Like the other reports,
//! this one never repeats a value taken from a document.

use serde_json::{Value, json};

use super::REPORT_SCHEMA;
use crate::evaluate::Verdict;
use crate::input::InputIssue;

const REASON: &str = "the input was rejected, so nothing was evaluated";

/// The problems found in one input document.
#[derive(Debug, Clone, Copy)]
pub struct RejectedDocument<'a> {
    /// Which document: `snapshot`, `plan` or `availability`.
    pub document: &'a str,
    /// What is wrong with it.
    pub issues: &'a [InputIssue],
}

/// Renders the rejection as plain text.
pub fn rejected_text(documents: &[RejectedDocument<'_>]) -> String {
    let mut out = String::new();
    out.push_str("SIGNERREACH CHECK\n=================\n\n");
    out.push_str(&format!("VERDICT: {}\n", Verdict::Inconclusive.as_str()));
    out.push_str(&format!("Reason: {REASON}\n\n"));
    out.push_str("Input problems\n--------------\n");
    for document in documents {
        for issue in document.issues {
            out.push_str(&format!("  {} {issue}\n", document.document));
        }
    }
    out.push_str("\nFix the problems above and run the check again.\n");
    out
}

/// Renders the rejection as JSON, in the same envelope as the full report.
pub fn rejected_json(documents: &[RejectedDocument<'_>]) -> String {
    let issues: Vec<Value> = documents
        .iter()
        .flat_map(|document| {
            document.issues.iter().map(move |issue| {
                json!({
                    "document": document.document,
                    "code": issue.code.as_str(),
                    "location": issue.location,
                    "message": issue.message,
                })
            })
        })
        .collect();
    let value = json!({
        "schema": REPORT_SCHEMA,
        "tool": { "name": "signerreach", "version": env!("CARGO_PKG_VERSION") },
        "verdict": Verdict::Inconclusive.as_str(),
        "reason": REASON,
        "input_issues": issues,
    });
    format!("{value:#}\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::availability::parse_availability;
    use crate::input::plan::parse_plan;
    use crate::input::snapshot::parse_snapshot;

    fn problems() -> (Vec<InputIssue>, Vec<InputIssue>) {
        let snapshot = parse_snapshot("{}").expect_err("empty snapshot");
        let plan = parse_plan("[").expect_err("broken plan");
        (snapshot, plan)
    }

    #[test]
    fn the_text_lists_every_problem_under_its_document() {
        let (snapshot, plan) = problems();
        let text = rejected_text(&[
            RejectedDocument {
                document: "snapshot",
                issues: &snapshot,
            },
            RejectedDocument {
                document: "plan",
                issues: &plan,
            },
        ]);
        assert!(text.contains("VERDICT: INCONCLUSIVE\n"));
        assert!(text.contains("Reason: the input was rejected, so nothing was evaluated"));
        assert!(text.contains("  snapshot SR-"));
        assert!(text.contains("  plan SR-INPUT-01 at (document): not valid JSON at line 1"));
        assert!(text.ends_with("run the check again.\n"));
        assert!(text.is_ascii());
    }

    #[test]
    fn the_json_has_the_report_envelope_and_the_issues() {
        let (snapshot, plan) = problems();
        let text = rejected_json(&[
            RejectedDocument {
                document: "snapshot",
                issues: &snapshot,
            },
            RejectedDocument {
                document: "plan",
                issues: &plan,
            },
        ]);
        let value: Value = serde_json::from_str(&text).expect("valid JSON");
        assert_eq!(value["schema"], "signerreach.report/1");
        assert_eq!(value["verdict"], "INCONCLUSIVE");
        assert_eq!(value["reason"], REASON);
        let issues = value["input_issues"].as_array().expect("an array");
        assert_eq!(issues.len(), snapshot.len() + plan.len());
        assert_eq!(issues[0]["document"], "snapshot");
        assert_eq!(issues[0]["code"], snapshot[0].code.as_str());
        assert_eq!(issues.last().unwrap()["document"], "plan");
        assert_eq!(issues.last().unwrap()["location"], "");
        assert!(text.ends_with("}\n"));
    }

    #[test]
    fn a_document_with_no_problems_adds_nothing() {
        let availability = parse_availability("{}").expect_err("no schema");
        let text = rejected_text(&[
            RejectedDocument {
                document: "snapshot",
                issues: &[],
            },
            RejectedDocument {
                document: "availability",
                issues: &availability,
            },
        ]);
        assert!(!text.contains("  snapshot "));
        assert!(text.contains("  availability SR-INPUT-01 at /schema"));
    }

    #[test]
    fn the_output_is_reproducible() {
        let (snapshot, _) = problems();
        let documents = [RejectedDocument {
            document: "snapshot",
            issues: &snapshot,
        }];
        assert_eq!(rejected_json(&documents), rejected_json(&documents));
        assert_eq!(rejected_text(&documents), rejected_text(&documents));
    }
}
