//! Checks the claim that `signerreach check` never opens a network connection.
//!
//! The claim is not proved by watching sockets, which cannot be done the same
//! way on every platform. It is shown by two facts that can be checked: the code
//! `check` runs cannot reach any networking code, and a networking dependency
//! added to it would be noticed here.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

/// Crates that open connections, or that are used only for that.
const NETWORKING: [&str; 4] = ["signerreach-stellar", "ureq", "rustls", "ring"];

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Package name to the names of its dependencies, read from `Cargo.lock`.
fn lock_graph() -> HashMap<String, Vec<String>> {
    let text = fs::read_to_string(root().join("Cargo.lock")).expect("read Cargo.lock");
    let mut graph: HashMap<String, Vec<String>> = HashMap::new();
    let mut current = String::new();
    let mut in_dependencies = false;
    for line in text.lines() {
        if let Some(name) = line
            .strip_prefix("name = \"")
            .and_then(|rest| rest.strip_suffix('"'))
        {
            current = name.to_string();
            graph.entry(current.clone()).or_default();
        } else if line.starts_with("dependencies = [") {
            in_dependencies = true;
        } else if in_dependencies && line == "]" {
            in_dependencies = false;
        } else if in_dependencies {
            // Entries are `"name"` or `"name 1.2.3"` when two versions exist.
            let entry = line.trim().trim_end_matches(',').trim_matches('"');
            let name = entry.split(' ').next().unwrap_or_default();
            graph
                .entry(current.clone())
                .or_default()
                .push(name.to_string());
        }
    }
    graph
}

/// Every package reachable from `start`, including `start`.
fn reachable(graph: &HashMap<String, Vec<String>>, start: &str) -> HashSet<String> {
    let mut seen = HashSet::new();
    let mut pending = vec![start.to_string()];
    while let Some(name) = pending.pop() {
        if seen.insert(name.clone()) {
            pending.extend(graph.get(&name).cloned().unwrap_or_default());
        }
    }
    seen
}

#[test]
fn the_core_depends_on_no_networking_code() {
    let graph = lock_graph();

    // The whole binary does reach networking code (through `fetch`), so a
    // reading of the lock file that could not see it would prove nothing.
    let binary = reachable(&graph, "signerreach-cli");
    for name in NETWORKING {
        assert!(
            binary.contains(name),
            "the lock file reading cannot see {name}"
        );
    }

    let core = reachable(&graph, "signerreach-core");
    assert!(
        core.contains("serde_json"),
        "the lock file reading cannot see the core's dependencies"
    );
    for name in NETWORKING {
        assert!(!core.contains(name), "signerreach-core depends on {name}");
    }
}

fn source_files(directory: &Path, files: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(directory).expect("read a source directory") {
        let path = entry.expect("read a directory entry").path();
        if path.is_dir() {
            source_files(&path, files);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            files.push(path);
        }
    }
}

#[test]
fn the_code_check_runs_never_names_networking() {
    let mut files = Vec::new();
    source_files(&root().join("crates/signerreach-core/src"), &mut files);
    files.push(root().join("crates/signerreach-cli/src/check.rs"));
    assert!(
        files.len() > 10,
        "found too few source files to mean anything"
    );

    for file in files {
        let text = fs::read_to_string(&file).expect("read a source file");
        for word in ["std::net", "signerreach_stellar", "ureq"] {
            assert!(
                !text.contains(word),
                "{} mentions {word}",
                file.strip_prefix(root()).unwrap_or(&file).display()
            );
        }
    }
}
