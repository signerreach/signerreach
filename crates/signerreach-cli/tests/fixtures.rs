//! Runs every directory under `fixtures/` through the real binary.
//!
//! See `fixtures/README.md` for the layout and the two kinds of fixture.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::Value;

const FILES: [&str; 4] = [
    "snapshot.json",
    "plan.json",
    "availability.json",
    "expected.json",
];

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn fixture_dirs() -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = fs::read_dir(workspace_root().join("fixtures"))
        .expect("the fixtures directory exists")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect();
    dirs.sort();
    dirs
}

fn name(dir: &Path) -> String {
    dir.file_name()
        .expect("a directory name")
        .to_string_lossy()
        .into_owned()
}

fn read_json(path: &Path) -> Value {
    let text = fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
    serde_json::from_str(&text)
        .unwrap_or_else(|error| panic!("{} is not valid JSON: {error}", path.display()))
}

fn run(dir: &Path, format: &str) -> Output {
    Command::new(env!("CARGO_BIN_EXE_signerreach"))
        .arg("check")
        .arg("--snapshot")
        .arg(dir.join("snapshot.json"))
        .arg("--plan")
        .arg(dir.join("plan.json"))
        .arg("--availability")
        .arg(dir.join("availability.json"))
        .arg("--format")
        .arg(format)
        .output()
        .expect("the binary runs")
}

fn strings(value: &Value) -> Vec<String> {
    value
        .as_array()
        .unwrap_or_else(|| panic!("expected an array, got {value}"))
        .iter()
        .map(|item| item.as_str().expect("a string").to_string())
        .collect()
}

fn exit_code_of(verdict: &str) -> i64 {
    match verdict {
        "SAFE WITHIN SCOPE" => 0,
        "UNSAFE" => 1,
        "INCONCLUSIVE" => 2,
        other => panic!("unknown verdict {other}"),
    }
}

#[test]
fn the_corpus_has_fixtures_and_chain_fixtures() {
    let dirs = fixture_dirs();
    assert!(!dirs.is_empty());
    let chain = dirs
        .iter()
        .filter(|dir| read_json(&dir.join("expected.json"))["source"] == "chain")
        .count();
    assert!(chain >= 1, "there is no chain fixture");
}

#[test]
fn every_fixture_directory_is_complete_and_well_formed() {
    let evidence =
        fs::read_to_string(workspace_root().join("docs/evidence/testnet-2026-10-13/README.md"))
            .expect("the evidence summary exists");

    let mut ids = BTreeSet::new();
    for dir in fixture_dirs() {
        let label = name(&dir);
        for file in FILES {
            assert!(dir.join(file).is_file(), "{label}: missing {file}");
        }
        let expected = read_json(&dir.join("expected.json"));

        let id = expected["id"].as_str().expect("id is a string");
        assert!(id.starts_with("FX-"), "{label}: id {id}");
        assert!(ids.insert(id.to_string()), "{label}: id {id} is used twice");
        assert!(
            label.starts_with(&id.to_lowercase()),
            "{label}: the directory name must start with {}",
            id.to_lowercase()
        );
        assert!(
            expected["summary"].as_str().is_some_and(|s| !s.is_empty()),
            "{label}: summary"
        );

        let verdict = expected["verdict"].as_str().expect("verdict is a string");
        assert_eq!(
            expected["exit_code"].as_i64(),
            Some(exit_code_of(verdict)),
            "{label}: exit_code does not match the verdict"
        );
        strings(&expected["codes"]);
        strings(&expected["transactions"]);

        match expected["source"].as_str() {
            Some("chain") => {
                let cases = strings(&expected["evidence_cases"]);
                assert!(!cases.is_empty(), "{label}: a chain fixture needs evidence");
                for case in cases {
                    assert!(
                        evidence.contains(&format!("| {case} |")),
                        "{label}: case {case} is not in the evidence table"
                    );
                }
            }
            Some("synthetic") => assert!(
                expected.get("evidence_cases").is_none(),
                "{label}: a synthetic fixture cannot claim chain evidence"
            ),
            other => panic!("{label}: source must be chain or synthetic, not {other:?}"),
        }
    }
}

#[test]
fn every_fixture_gives_its_expected_result() {
    for dir in fixture_dirs() {
        let label = name(&dir);
        let expected = read_json(&dir.join("expected.json"));
        let output = run(&dir, "json");

        assert_eq!(
            output.status.code().map(i64::from),
            expected["exit_code"].as_i64(),
            "{label}: exit code"
        );
        let report: Value = serde_json::from_slice(&output.stdout)
            .unwrap_or_else(|error| panic!("{label}: the report is not JSON: {error}"));
        assert_eq!(report["verdict"], expected["verdict"], "{label}: verdict");

        // A full report lists diagnostics; a report on rejected input lists
        // input problems. Either way the codes are compared as a set.
        let list = if report.get("diagnostics").is_some() {
            &report["diagnostics"]
        } else {
            &report["input_issues"]
        };
        let actual_codes: BTreeSet<String> = list
            .as_array()
            .expect("a list of findings")
            .iter()
            .map(|item| item["code"].as_str().expect("a code").to_string())
            .collect();
        let expected_codes: BTreeSet<String> = strings(&expected["codes"]).into_iter().collect();
        assert_eq!(actual_codes, expected_codes, "{label}: codes");

        let actual_results: Vec<String> = report
            .get("transactions")
            .map(|transactions| {
                transactions
                    .as_array()
                    .expect("an array")
                    .iter()
                    .map(|t| t["result"].as_str().expect("a result").to_string())
                    .collect()
            })
            .unwrap_or_default();
        assert_eq!(
            actual_results,
            strings(&expected["transactions"]),
            "{label}: transaction results"
        );
    }
}

#[test]
fn the_text_report_agrees_with_the_json_report() {
    for dir in fixture_dirs() {
        let label = name(&dir);
        let expected = read_json(&dir.join("expected.json"));
        let output = run(&dir, "text");
        assert_eq!(
            output.status.code().map(i64::from),
            expected["exit_code"].as_i64(),
            "{label}: exit code in text mode"
        );
        let text = String::from_utf8(output.stdout).expect("utf-8");
        let verdict = expected["verdict"].as_str().expect("a verdict");
        assert!(
            text.contains(&format!("VERDICT: {verdict}\n")),
            "{label}: the text report does not state {verdict}"
        );
    }
}
