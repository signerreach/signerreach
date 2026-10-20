//! Keeps `docs/walkthrough/README.md` true: the two runs it shows give the
//! verdicts it quotes.

use std::path::{Path, PathBuf};
use std::process::Command;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Runs `check` on the walkthrough's account and availability with `plan`, and
/// returns the exit code and the text report.
fn check(plan: &Path) -> (i32, String) {
    let fixture = root().join("fixtures/fx-01-raise-high-with-lost-signer");
    let output = Command::new(env!("CARGO_BIN_EXE_signerreach"))
        .arg("check")
        .arg("--snapshot")
        .arg(fixture.join("snapshot.json"))
        .arg("--plan")
        .arg(plan)
        .arg("--availability")
        .arg(fixture.join("availability.json"))
        .output()
        .expect("the binary runs");
    (
        output.status.code().expect("an exit code"),
        String::from_utf8(output.stdout).expect("UTF-8"),
    )
}

#[test]
fn the_plan_in_the_walkthrough_is_unsafe() {
    let (code, report) =
        check(&root().join("fixtures/fx-01-raise-high-with-lost-signer/plan.json"));
    assert_eq!(code, 1);
    assert!(report.contains("VERDICT: UNSAFE"));
    assert!(report.contains("SR-LOSS-01"));
}

#[test]
fn the_corrected_plan_in_the_walkthrough_is_safe_within_scope() {
    let (code, report) = check(&root().join("docs/walkthrough/corrected-plan.json"));
    assert_eq!(code, 0);
    assert!(report.contains("VERDICT: SAFE WITHIN SCOPE"));
    assert!(report.contains("HIGH   threshold   2  total 3  available 2  unknown 0: reachable"));
}
