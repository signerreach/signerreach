//! A whole check, end to end: parse three documents, evaluate, render.
//!
//! The input is three documents separated by NUL bytes: snapshot, plan,
//! availability. The properties checked are the ones the reports promise:
//!
//! - no input makes the evaluator or a renderer panic;
//! - the JSON report is always valid JSON, and its verdict is the evaluation's;
//! - the same inputs always give byte-identical reports;
//! - a rejection report is valid JSON too.

#![no_main]

use libfuzzer_sys::fuzz_target;
use signerreach_core::{
    RejectedDocument, evaluate, json_report, parse_availability, parse_plan, parse_snapshot,
    rejected_json, rejected_text, text_report,
};

fuzz_target!(|data: &[u8]| {
    let mut parts = data.splitn(3, |byte| *byte == 0);
    let (Some(snapshot), Some(plan), Some(availability)) =
        (parts.next(), parts.next(), parts.next())
    else {
        return;
    };
    let (Ok(snapshot), Ok(plan), Ok(availability)) = (
        std::str::from_utf8(snapshot),
        std::str::from_utf8(plan),
        std::str::from_utf8(availability),
    ) else {
        return;
    };

    let snapshot = parse_snapshot(snapshot);
    let plan = parse_plan(plan);
    let availability = parse_availability(availability);

    match (&snapshot, &plan, &availability) {
        (Ok(snapshot), Ok(plan), Ok(availability)) => {
            let evaluation = evaluate(snapshot, plan, availability);

            let json = json_report(snapshot, plan, availability, &evaluation);
            let value: serde_json::Value =
                serde_json::from_str(&json).expect("the JSON report is valid JSON");
            assert_eq!(
                value["verdict"].as_str(),
                Some(evaluation.verdict.as_str()),
                "the report states the evaluation's verdict"
            );

            let text = text_report(snapshot, plan, availability, &evaluation);
            let again = evaluate(snapshot, plan, availability);
            assert_eq!(json, json_report(snapshot, plan, availability, &again));
            assert_eq!(text, text_report(snapshot, plan, availability, &again));
        }
        _ => {
            let empty = [];
            let documents = [
                RejectedDocument {
                    document: "snapshot",
                    issues: snapshot.as_ref().err().map_or(&empty[..], |v| &v[..]),
                },
                RejectedDocument {
                    document: "plan",
                    issues: plan.as_ref().err().map_or(&empty[..], |v| &v[..]),
                },
                RejectedDocument {
                    document: "availability",
                    issues: availability.as_ref().err().map_or(&empty[..], |v| &v[..]),
                },
            ];
            let _ = rejected_text(&documents);
            serde_json::from_str::<serde_json::Value>(&rejected_json(&documents))
                .expect("the rejection report is valid JSON");
        }
    }
});
