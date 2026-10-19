//! End-to-end tests for `signerreach fetch`: run the real binary against a fake
//! Horizon server on this machine. No test talks to the real network.

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener};
use std::process::{Command, Output};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;

use signerreach_core::{Network, parse_snapshot};

const ACCOUNT_JSON: &str =
    include_str!("../../signerreach-stellar/tests/data/horizon-testnet-three-signers.json");
const TESTNET: &str = "Test SDF Network ; September 2015";

/// A fake Horizon server. It answers the root and the one account, and counts
/// every request it receives.
struct Fake {
    address: SocketAddr,
    requests: Arc<AtomicUsize>,
}

impl Fake {
    fn start(passphrase: &'static str) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind a local port");
        let address = listener.local_addr().expect("local address");
        let requests = Arc::new(AtomicUsize::new(0));
        let counter = requests.clone();
        let id = account_id();
        thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { continue };
                let (counter, id) = (counter.clone(), id.clone());
                thread::spawn(move || {
                    let mut buffer = [0u8; 4096];
                    let Ok(count) = stream.read(&mut buffer) else {
                        return;
                    };
                    let request = String::from_utf8_lossy(&buffer[..count]).into_owned();
                    let path = request.split(' ').nth(1).unwrap_or("").to_string();
                    counter.fetch_add(1, Ordering::SeqCst);

                    let (status, extra, body) = if path == "/" {
                        let root = serde_json::json!({
                            "network_passphrase": passphrase,
                            "current_protocol_version": 29,
                        });
                        (200, "", root.to_string())
                    } else if path == format!("/accounts/{id}") {
                        (
                            200,
                            "Latest-Ledger: 5240432\r\nDate: Sat, 17 Oct 2026 08:29:07 GMT\r\n",
                            ACCOUNT_JSON.to_string(),
                        )
                    } else {
                        (404, "", String::new())
                    };
                    let head = format!(
                        "HTTP/1.1 {status} Test\r\nContent-Length: {}\r\nConnection: close\r\n{extra}\r\n",
                        body.len()
                    );
                    let _ = stream.write_all(head.as_bytes());
                    let _ = stream.write_all(body.as_bytes());
                });
            }
        });
        Self { address, requests }
    }

    fn url(&self) -> String {
        format!("http://{}", self.address)
    }

    fn request_count(&self) -> usize {
        self.requests.load(Ordering::SeqCst)
    }
}

fn account_id() -> String {
    let record: serde_json::Value = serde_json::from_str(ACCOUNT_JSON).expect("recorded JSON");
    record["id"].as_str().expect("an id").to_string()
}

fn fetch(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_signerreach"))
        .arg("fetch")
        .args(args)
        .output()
        .expect("run the binary")
}

fn code(output: &Output) -> i32 {
    output.status.code().expect("an exit code")
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

#[test]
fn it_prints_a_snapshot_the_checker_accepts() {
    let server = Fake::start(TESTNET);
    let output = fetch(&[
        "--network",
        "testnet",
        "--account",
        &account_id(),
        "--horizon-url",
        &server.url(),
    ]);

    assert_eq!(code(&output), 0, "stderr: {}", stderr(&output));
    assert!(output.stderr.is_empty());
    let text = String::from_utf8(output.stdout).expect("UTF-8");
    let snapshot = parse_snapshot(&text).expect("the checker accepts it");
    assert_eq!(snapshot.network(), Network::Testnet);
    assert_eq!(snapshot.provenance().as_of_ledger(), 5_240_432);
    assert_eq!(server.request_count(), 3);
}

#[test]
fn an_invalid_account_is_a_usage_error_and_makes_no_request() {
    let server = Fake::start(TESTNET);
    let output = fetch(&[
        "--network",
        "testnet",
        "--account",
        "not-an-account",
        "--horizon-url",
        &server.url(),
    ]);

    assert_eq!(code(&output), 64);
    assert!(output.stdout.is_empty());
    assert_eq!(server.request_count(), 0);
}

#[test]
fn plain_http_to_another_machine_is_a_usage_error() {
    let output = fetch(&[
        "--network",
        "testnet",
        "--account",
        &account_id(),
        "--horizon-url",
        "http://horizon.example.com",
    ]);

    assert_eq!(code(&output), 64);
    assert!(output.stdout.is_empty());
    assert!(!stderr(&output).contains("example.com"));
}

#[test]
fn an_unknown_network_is_a_usage_error() {
    let output = fetch(&["--network", "mars", "--account", &account_id()]);
    assert_eq!(code(&output), 64);
    assert!(output.stdout.is_empty());
}

#[test]
fn a_server_on_the_other_network_is_refused_without_printing_anything() {
    let server = Fake::start(TESTNET);
    let output = fetch(&[
        "--network",
        "pubnet",
        "--account",
        &account_id(),
        "--horizon-url",
        &server.url(),
    ]);

    assert_eq!(code(&output), 69);
    assert!(output.stdout.is_empty());
    assert!(!stderr(&output).contains("September"));
    assert_eq!(server.request_count(), 1);
}

#[test]
fn a_server_that_is_not_there_is_unavailable() {
    let address = {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind a local port");
        listener.local_addr().expect("local address")
    };
    let output = fetch(&[
        "--network",
        "testnet",
        "--account",
        &account_id(),
        "--horizon-url",
        &format!("http://{address}"),
    ]);

    assert_eq!(code(&output), 69);
    assert!(output.stdout.is_empty());
}
