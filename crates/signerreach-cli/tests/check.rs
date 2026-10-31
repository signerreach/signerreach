//! End-to-end tests: run the real binary on files written for each test.

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};

use signerreach_core::SignerKey;

/// A scratch directory that is removed when the test ends.
struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "signerreach-cli-test-{}-{name}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create the scratch directory");
        Self(dir)
    }

    fn write(&self, name: &str, contents: &str) -> String {
        let path = self.0.join(name);
        fs::write(&path, contents).expect("write a test file");
        path.to_string_lossy().into_owned()
    }

    fn write_bytes(&self, name: &str, contents: &[u8]) -> String {
        let path = self.0.join(name);
        fs::write(&path, contents).expect("write a test file");
        path.to_string_lossy().into_owned()
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn key(n: u8) -> String {
    SignerKey::from_bytes([n; 32]).to_string()
}

/// A snapshot of account 1 with the given master weight, extra signers and thresholds.
fn snapshot(master: u32, signers: &[(u8, u32)], thresholds: (u32, u32, u32)) -> String {
    let signers: Vec<String> = signers
        .iter()
        .map(|&(n, w)| format!(r#"{{"key":"{}","type":"ed25519","weight":{w}}}"#, key(n)))
        .collect();
    format!(
        r#"{{"schema":"signerreach.snapshot/1","network":"testnet","account":"{}",
        "provenance":{{"as_of_ledger":100,"captured_at":"2026-10-14T09:00:00Z","source":"file","protocol_version":29}},
        "master_weight":{master},
        "thresholds":{{"low":{},"medium":{},"high":{}}},
        "signers":[{}]}}"#,
        key(1),
        thresholds.0,
        thresholds.1,
        thresholds.2,
        signers.join(",")
    )
}

fn plan(transactions: &str) -> String {
    format!(r#"{{"schema":"signerreach.plan/1","transactions":[{transactions}]}}"#)
}

fn availability(statuses: &[(u8, &str)]) -> String {
    let entries: Vec<String> = statuses
        .iter()
        .map(|&(n, status)| format!(r#""{}":"{status}""#, key(n)))
        .collect();
    format!(
        r#"{{"schema":"signerreach.availability/1","signers":{{{}}}}}"#,
        entries.join(",")
    )
}

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_signerreach"))
        .args(args)
        .output()
        .expect("the binary runs")
}

fn check(
    scratch: &Scratch,
    snapshot: &str,
    plan: &str,
    availability: &str,
    extra: &[&str],
) -> Output {
    let snapshot = scratch.write("snapshot.json", snapshot);
    let plan = scratch.write("plan.json", plan);
    let availability = scratch.write("availability.json", availability);
    let mut args = vec![
        "check",
        "--snapshot",
        &snapshot,
        "--plan",
        &plan,
        "--availability",
        &availability,
    ];
    args.extend_from_slice(extra);
    run(&args)
}

fn code(output: &Output) -> i32 {
    output
        .status
        .code()
        .expect("the process exited with a code")
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("utf-8 on stdout")
}

fn stderr(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).expect("utf-8 on stderr")
}

/// FX-01: raising HIGH with a lost signer.
fn unsafe_plan() -> String {
    plan(&format!(
        r#"{{"id":"tx1","signed_by":["{}"],"operations":[{{"op":"set_options","high":2}}]}}"#,
        key(1)
    ))
}

#[test]
fn an_unsafe_plan_exits_1_and_prints_the_text_report() {
    let scratch = Scratch::new("unsafe-text");
    let output = check(
        &scratch,
        &snapshot(1, &[(2, 1)], (1, 1, 1)),
        &unsafe_plan(),
        &availability(&[(1, "available"), (2, "lost")]),
        &[],
    );
    assert_eq!(code(&output), 1);
    let text = stdout(&output);
    assert!(text.starts_with("SIGNERREACH CHECK\n"));
    assert!(text.contains("VERDICT: UNSAFE\n"));
    assert!(text.contains("SR-LOSS-01"));
    assert!(stderr(&output).is_empty());
}

#[test]
fn a_safe_plan_exits_0() {
    let scratch = Scratch::new("safe");
    let output = check(
        &scratch,
        &snapshot(1, &[(2, 1)], (1, 1, 1)),
        &plan(""),
        &availability(&[(1, "available"), (2, "available")]),
        &[],
    );
    assert_eq!(code(&output), 0);
    assert!(stdout(&output).contains("VERDICT: SAFE WITHIN SCOPE\n"));
}

#[test]
fn an_inconclusive_result_exits_2() {
    let scratch = Scratch::new("inconclusive");
    let output = check(
        &scratch,
        &snapshot(1, &[(2, 1), (3, 1)], (1, 2, 2)),
        &plan(""),
        &availability(&[(1, "available")]),
        &[],
    );
    assert_eq!(code(&output), 2);
    assert!(stdout(&output).contains("VERDICT: INCONCLUSIVE\n"));
}

#[test]
fn the_json_format_prints_the_json_report() {
    let scratch = Scratch::new("json");
    let output = check(
        &scratch,
        &snapshot(1, &[(2, 1)], (1, 1, 1)),
        &unsafe_plan(),
        &availability(&[(1, "available"), (2, "lost")]),
        &["--format", "json"],
    );
    assert_eq!(code(&output), 1);
    let text = stdout(&output);
    assert!(text.starts_with("{\n"));
    assert!(text.ends_with("}\n"));
    assert!(text.contains(r#""schema": "signerreach.report/1""#));
    assert!(text.contains(r#""verdict": "UNSAFE""#));
    assert!(text.contains(r#""code": "SR-LOSS-01""#));
}

#[test]
fn rejected_input_is_inconclusive_and_lists_every_problem() {
    let scratch = Scratch::new("rejected");
    let output = check(&scratch, "{}", "[", &availability(&[]), &[]);
    assert_eq!(code(&output), 2);
    let text = stdout(&output);
    assert!(text.contains("VERDICT: INCONCLUSIVE\n"));
    assert!(text.contains("Input problems"));
    assert!(text.contains("  snapshot SR-"));
    assert!(text.contains("  plan SR-INPUT-01 at (document): not valid JSON"));
    assert!(!text.contains("  availability "));
}

#[test]
fn rejected_input_in_json_keeps_the_report_envelope() {
    let scratch = Scratch::new("rejected-json");
    let output = check(
        &scratch,
        "{}",
        &plan(""),
        &availability(&[]),
        &["--format", "json"],
    );
    assert_eq!(code(&output), 2);
    let text = stdout(&output);
    assert!(text.contains(r#""schema": "signerreach.report/1""#));
    assert!(text.contains(r#""verdict": "INCONCLUSIVE""#));
    assert!(text.contains(r#""input_issues": ["#));
    assert!(text.contains(r#""document": "snapshot""#));
}

#[test]
fn a_missing_file_exits_64_and_says_which() {
    let scratch = Scratch::new("missing");
    let plan_file = scratch.write("plan.json", &plan(""));
    let availability_file = scratch.write("availability.json", &availability(&[]));
    let output = run(&[
        "check",
        "--snapshot",
        "definitely-not-a-file.json",
        "--plan",
        &plan_file,
        "--availability",
        &availability_file,
    ]);
    assert_eq!(code(&output), 64);
    assert!(stdout(&output).is_empty());
    let message = stderr(&output);
    assert!(message.contains("cannot read the snapshot file"));
}

#[test]
fn a_file_name_is_not_echoed_in_an_error() {
    let scratch = Scratch::new("path-error");
    let plan_file = scratch.write("plan.json", &plan(""));
    let availability_file = scratch.write("availability.json", &availability(&[]));
    let output = run(&[
        "check",
        "--snapshot",
        "SECRET_MARKER\nnot-a-file",
        "--plan",
        &plan_file,
        "--availability",
        &availability_file,
    ]);
    assert_eq!(code(&output), 64);
    assert!(!stderr(&output).contains("SECRET_MARKER"));
    assert!(!stderr(&output).contains("not-a-file"));
}

#[test]
fn a_file_that_is_not_utf8_exits_64() {
    let scratch = Scratch::new("not-utf8");
    let snapshot_file = scratch.write_bytes("snapshot.json", &[0xff, 0xfe, 0x00]);
    let plan_file = scratch.write("plan.json", &plan(""));
    let availability_file = scratch.write("availability.json", &availability(&[]));
    let output = run(&[
        "check",
        "--snapshot",
        &snapshot_file,
        "--plan",
        &plan_file,
        "--availability",
        &availability_file,
    ]);
    assert_eq!(code(&output), 64);
    assert!(stderr(&output).contains("is not valid UTF-8"));
}

#[test]
fn a_file_over_the_size_limit_is_rejected_as_input_not_read_whole() {
    let scratch = Scratch::new("too-large");
    let big = vec![b' '; 1024 * 1024 + 1];
    let snapshot_file = scratch.write_bytes("snapshot.json", &big);
    let plan_file = scratch.write("plan.json", &plan(""));
    let availability_file = scratch.write("availability.json", &availability(&[]));
    let output = run(&[
        "check",
        "--snapshot",
        &snapshot_file,
        "--plan",
        &plan_file,
        "--availability",
        &availability_file,
    ]);
    assert_eq!(code(&output), 2);
    assert!(stdout(&output).contains("larger than the 1 MiB limit"));
}

#[test]
fn usage_errors_exit_64_not_2() {
    for args in [
        vec![],
        vec!["check"],
        vec!["check", "--snapshot", "a.json"],
        vec!["check", "--no-such-flag"],
        vec!["no-such-command"],
    ] {
        let output = run(&args);
        assert_eq!(code(&output), 64, "{args:?}");
        assert!(stdout(&output).is_empty(), "{args:?}");
        assert!(!stderr(&output).is_empty(), "{args:?}");
    }
}

#[test]
fn an_unknown_format_is_a_usage_error() {
    let scratch = Scratch::new("bad-format");
    let output = check(
        &scratch,
        &snapshot(1, &[], (1, 1, 1)),
        &plan(""),
        &availability(&[(1, "available")]),
        &["--format", "xml"],
    );
    assert_eq!(code(&output), 64);
}

#[test]
fn help_and_version_exit_0() {
    for args in [vec!["--help"], vec!["check", "--help"], vec!["--version"]] {
        let output = run(&args);
        assert_eq!(code(&output), 0, "{args:?}");
        assert!(!stdout(&output).is_empty(), "{args:?}");
    }
    let help = stdout(&run(&["check", "--help"]));
    for word in ["--snapshot", "--plan", "--availability", "--format"] {
        assert!(help.contains(word), "{word}");
    }
}

#[test]
fn the_same_input_gives_the_same_output() {
    let scratch = Scratch::new("repeat");
    let make = || {
        check(
            &scratch,
            &snapshot(1, &[(2, 1)], (1, 1, 1)),
            &unsafe_plan(),
            &availability(&[(1, "available"), (2, "lost")]),
            &["--format", "json"],
        )
    };
    let first = make();
    let second = make();
    assert_eq!(first.stdout, second.stdout);
    assert_eq!(code(&first), code(&second));
}

#[test]
fn input_files_are_never_modified() {
    let scratch = Scratch::new("read-only");
    let snapshot_text = snapshot(1, &[(2, 1)], (1, 1, 1));
    let plan_text = unsafe_plan();
    let availability_text = availability(&[(1, "available"), (2, "lost")]);
    let _ = check(
        &scratch,
        &snapshot_text,
        &plan_text,
        &availability_text,
        &[],
    );
    assert_eq!(
        fs::read_to_string(scratch.0.join("snapshot.json")).unwrap(),
        snapshot_text
    );
    assert_eq!(
        fs::read_to_string(scratch.0.join("plan.json")).unwrap(),
        plan_text
    );
    assert_eq!(
        fs::read_to_string(scratch.0.join("availability.json")).unwrap(),
        availability_text
    );
}
