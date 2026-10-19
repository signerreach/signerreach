//! The `check` command: read three documents, evaluate, print a report.
//!
//! This command reads files and prints to the terminal. It makes no network
//! connection and never writes a file.

use std::fs::File;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Args as ClapArgs, ValueEnum};
use signerreach_core::{
    Code, InputIssue, MAX_INPUT_BYTES, RejectedDocument, Verdict, evaluate, json_report,
    parse_availability, parse_plan, parse_snapshot, rejected_json, rejected_text, text_report,
};

/// Exit code for a usage error or a file that cannot be read.
pub const USAGE_ERROR: u8 = 64;

/// Arguments of `signerreach check`.
#[derive(Debug, ClapArgs)]
pub struct Args {
    /// The account snapshot (signerreach.snapshot/1)
    #[arg(long, value_name = "FILE")]
    snapshot: PathBuf,

    /// The plan (signerreach.plan/1); use an empty transaction list to check the snapshot alone
    #[arg(long, value_name = "FILE")]
    plan: PathBuf,

    /// The availability declaration (signerreach.availability/1)
    #[arg(long, value_name = "FILE")]
    availability: PathBuf,

    /// The report format
    #[arg(long, value_enum, default_value_t = Format::Text)]
    format: Format,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum Format {
    /// Human-readable text
    Text,
    /// Machine-readable JSON (signerreach.report/1)
    Json,
}

/// A document that was read, or the fact that it was too large to read.
enum Document {
    Text(String),
    TooLarge,
}

/// Runs the command and returns the process exit code.
pub fn run(args: &Args) -> ExitCode {
    let snapshot = read(&args.snapshot, "snapshot");
    let plan = read(&args.plan, "plan");
    let availability = read(&args.availability, "availability");
    let (Some(snapshot), Some(plan), Some(availability)) = (snapshot, plan, availability) else {
        return ExitCode::from(USAGE_ERROR);
    };

    let snapshot = parse(snapshot, parse_snapshot);
    let plan = parse(plan, parse_plan);
    let availability = parse(availability, parse_availability);

    match (snapshot, plan, availability) {
        (Ok(snapshot), Ok(plan), Ok(availability)) => {
            let evaluation = evaluate(&snapshot, &plan, &availability);
            let report = match args.format {
                Format::Text => text_report(&snapshot, &plan, &availability, &evaluation),
                Format::Json => json_report(&snapshot, &plan, &availability, &evaluation),
            };
            emit(&report, exit_code(evaluation.verdict))
        }
        (snapshot, plan, availability) => {
            let snapshot = snapshot.err().unwrap_or_default();
            let plan = plan.err().unwrap_or_default();
            let availability = availability.err().unwrap_or_default();
            let documents = [
                RejectedDocument {
                    document: "snapshot",
                    issues: &snapshot,
                },
                RejectedDocument {
                    document: "plan",
                    issues: &plan,
                },
                RejectedDocument {
                    document: "availability",
                    issues: &availability,
                },
            ];
            let report = match args.format {
                Format::Text => rejected_text(&documents),
                Format::Json => rejected_json(&documents),
            };
            emit(&report, exit_code(Verdict::Inconclusive))
        }
    }
}

const fn exit_code(verdict: Verdict) -> u8 {
    match verdict {
        Verdict::SafeWithinScope => 0,
        Verdict::Unsafe => 1,
        Verdict::Inconclusive => 2,
    }
}

/// Reads a file, at most one byte past the size limit. A file that cannot be
/// read is reported on stderr and gives `None`.
fn read(path: &Path, label: &str) -> Option<Document> {
    let limit = u64::try_from(MAX_INPUT_BYTES)
        .unwrap_or(u64::MAX)
        .saturating_add(1);
    let mut bytes = Vec::new();
    let result = File::open(path).and_then(|file| file.take(limit).read_to_end(&mut bytes));
    if let Err(error) = result {
        eprintln!(
            "signerreach: cannot read the {label} file {}: {error}",
            path.display()
        );
        return None;
    }
    if bytes.len() > MAX_INPUT_BYTES {
        return Some(Document::TooLarge);
    }
    match String::from_utf8(bytes) {
        Ok(text) => Some(Document::Text(text)),
        Err(_) => {
            eprintln!(
                "signerreach: the {label} file {} is not valid UTF-8",
                path.display()
            );
            None
        }
    }
}

fn parse<T>(
    document: Document,
    parser: fn(&str) -> Result<T, Vec<InputIssue>>,
) -> Result<T, Vec<InputIssue>> {
    match document {
        Document::Text(text) => parser(&text),
        Document::TooLarge => Err(vec![InputIssue {
            code: Code::Input01,
            location: String::new(),
            message: "the document is larger than the 1 MiB limit".to_string(),
        }]),
    }
}

/// Prints the text on stdout and returns `code`. A closed pipe, as in
/// `signerreach check ... | head`, is not an error.
pub fn emit(report: &str, code: u8) -> ExitCode {
    let mut stdout = io::stdout().lock();
    match stdout
        .write_all(report.as_bytes())
        .and_then(|()| stdout.flush())
    {
        Ok(()) => ExitCode::from(code),
        Err(error) if error.kind() == io::ErrorKind::BrokenPipe => ExitCode::from(code),
        Err(error) => {
            eprintln!("signerreach: cannot write the report: {error}");
            ExitCode::from(USAGE_ERROR)
        }
    }
}
