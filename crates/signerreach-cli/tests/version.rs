use std::process::Command;

#[test]
fn version_flag_prints_name_and_version() {
    let output = Command::new(env!("CARGO_BIN_EXE_signerreach"))
        .arg("--version")
        .output()
        .expect("the binary runs");
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).expect("utf-8 output");
    assert_eq!(
        text.trim(),
        format!("signerreach {}", env!("CARGO_PKG_VERSION"))
    );
}
