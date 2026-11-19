//! The three input readers must never panic, whatever they are given, and a
//! rejection must always say why.
//!
//! These are the functions behind `signerreach check`: they read files that a
//! person or another tool wrote.

#![no_main]

use libfuzzer_sys::fuzz_target;
use signerreach_core::{parse_availability, parse_plan, parse_snapshot};

fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };

    if let Err(issues) = parse_snapshot(text) {
        assert!(!issues.is_empty(), "snapshot rejected with no reason");
        for issue in &issues {
            let _ = issue.to_string();
        }
    }
    if let Err(issues) = parse_plan(text) {
        assert!(!issues.is_empty(), "plan rejected with no reason");
        for issue in &issues {
            let _ = issue.to_string();
        }
    }
    if let Err(issues) = parse_availability(text) {
        assert!(!issues.is_empty(), "availability rejected with no reason");
        for issue in &issues {
            let _ = issue.to_string();
        }
    }
});
