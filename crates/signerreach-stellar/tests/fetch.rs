//! Tests the client against a fake server on this machine.
//!
//! No test talks to the real network. The fake server records every request it
//! receives, so the tests can check what the client sent as well as what it did
//! with the answers.

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use signerreach_core::{Network, parse_snapshot};
use signerreach_stellar::{FetchError, HorizonUrl, Limits, MapError, UrlError, fetch_snapshot};

const ACCOUNT_JSON: &str = include_str!("data/horizon-testnet-three-signers.json");
const DATE: &str = "Sat, 17 Oct 2026 08:29:07 GMT";

#[derive(Debug, Clone)]
struct Request {
    method: String,
    path: String,
    headers: Vec<(String, String)>,
}

impl Request {
    fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
    }
}

#[derive(Debug, Clone)]
struct Reply {
    status: u16,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
    delay: Duration,
    claimed_length: Option<usize>,
}

impl Reply {
    fn ok(body: &str) -> Self {
        Self {
            status: 200,
            headers: Vec::new(),
            body: body.as_bytes().to_vec(),
            delay: Duration::ZERO,
            claimed_length: None,
        }
    }

    fn status(code: u16, body: &str) -> Self {
        Self {
            status: code,
            ..Self::ok(body)
        }
    }

    fn header(mut self, name: &str, value: &str) -> Self {
        self.headers.push((name.to_string(), value.to_string()));
        self
    }

    fn delayed(mut self, delay: Duration) -> Self {
        self.delay = delay;
        self
    }

    fn claiming(mut self, length: usize) -> Self {
        self.claimed_length = Some(length);
        self
    }
}

struct Server {
    address: SocketAddr,
    seen: Arc<Mutex<Vec<Request>>>,
    stop: Arc<AtomicBool>,
    accept: Option<JoinHandle<()>>,
}

impl Server {
    fn start<F>(handler: F) -> Self
    where
        F: Fn(&Request) -> Reply + Send + Sync + 'static,
    {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind a local port");
        let address = listener.local_addr().expect("local address");
        let seen = Arc::new(Mutex::new(Vec::new()));
        let stop = Arc::new(AtomicBool::new(false));
        let handler = Arc::new(handler);

        let (seen_for_thread, stop_for_thread) = (seen.clone(), stop.clone());
        let accept = thread::spawn(move || {
            for stream in listener.incoming() {
                if stop_for_thread.load(Ordering::SeqCst) {
                    break;
                }
                let Ok(stream) = stream else { continue };
                let (handler, seen) = (handler.clone(), seen_for_thread.clone());
                thread::spawn(move || serve(stream, &*handler, &seen));
            }
        });
        Self {
            address,
            seen,
            stop,
            accept: Some(accept),
        }
    }

    fn url(&self) -> HorizonUrl {
        HorizonUrl::parse(&format!("http://{}", self.address)).expect("a local address")
    }

    fn requests(&self) -> Vec<Request> {
        self.seen.lock().expect("lock").clone()
    }

    fn paths(&self) -> Vec<String> {
        self.requests().into_iter().map(|r| r.path).collect()
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        let _ = TcpStream::connect(self.address);
        if let Some(accept) = self.accept.take() {
            let _ = accept.join();
        }
    }
}

fn serve(
    mut stream: TcpStream,
    handler: &(dyn Fn(&Request) -> Reply + Send + Sync),
    seen: &Mutex<Vec<Request>>,
) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
    let mut buffer = Vec::new();
    let mut chunk = [0u8; 1024];
    while !buffer.windows(4).any(|window| window == b"\r\n\r\n") {
        match stream.read(&mut chunk) {
            Ok(0) | Err(_) => return,
            Ok(count) => buffer.extend_from_slice(&chunk[..count]),
        }
        if buffer.len() > 16 * 1024 {
            return;
        }
    }

    let text = String::from_utf8_lossy(&buffer).into_owned();
    let mut lines = text.split("\r\n");
    let mut first = lines.next().unwrap_or("").split(' ');
    let method = first.next().unwrap_or("").to_string();
    let path = first.next().unwrap_or("").to_string();
    let headers = lines
        .take_while(|line| !line.is_empty())
        .filter_map(|line| line.split_once(':'))
        .map(|(name, value)| (name.trim().to_ascii_lowercase(), value.trim().to_string()))
        .collect();
    let request = Request {
        method,
        path,
        headers,
    };
    seen.lock().expect("lock").push(request.clone());

    let reply = handler(&request);
    thread::sleep(reply.delay);
    let length = reply.claimed_length.unwrap_or(reply.body.len());
    let mut head = format!(
        "HTTP/1.1 {} Test\r\nContent-Length: {length}\r\nConnection: close\r\n",
        reply.status
    );
    for (name, value) in &reply.headers {
        head.push_str(&format!("{name}: {value}\r\n"));
    }
    head.push_str("\r\n");
    let _ = stream.write_all(head.as_bytes());
    let _ = stream.write_all(&reply.body);
    let _ = stream.flush();
}

fn account_id() -> String {
    let record: serde_json::Value = serde_json::from_str(ACCOUNT_JSON).expect("recorded JSON");
    record["id"].as_str().expect("an id").to_string()
}

fn root_json(passphrase: &str, protocol: u32) -> String {
    serde_json::json!({
        "network_passphrase": passphrase,
        "current_protocol_version": protocol,
    })
    .to_string()
}

fn testnet_root(protocol: u32) -> Reply {
    Reply::ok(&root_json("Test SDF Network ; September 2015", protocol))
}

fn good_account() -> Reply {
    Reply::ok(ACCOUNT_JSON)
        .header("Latest-Ledger", "5240432")
        .header("Date", DATE)
}

/// Serves `roots` in turn for the root address (the last one repeats) and
/// `account` for the account address.
fn routes(account: Reply, roots: Vec<Reply>) -> impl Fn(&Request) -> Reply + Send + Sync + 'static {
    let id = account_id();
    let root_reads = AtomicUsize::new(0);
    move |request| {
        if request.path == "/" {
            let index = root_reads.fetch_add(1, Ordering::SeqCst);
            roots[index.min(roots.len() - 1)].clone()
        } else if request.path == format!("/accounts/{id}") {
            account.clone()
        } else {
            Reply::status(404, "")
        }
    }
}

fn quick() -> Limits {
    Limits {
        connect_timeout: Duration::from_secs(2),
        total_timeout: Duration::from_secs(5),
        max_body_bytes: 1024 * 1024,
    }
}

fn fetch(server: &Server, account: &str, limits: &Limits) -> Result<String, FetchError> {
    fetch_snapshot(Network::Testnet, &server.url(), account, limits)
}

#[test]
fn a_good_answer_becomes_a_snapshot_the_checker_accepts() {
    let server = Server::start(routes(good_account(), vec![testnet_root(29)]));
    let text = fetch(&server, &account_id(), &quick()).expect("fetches");

    let snapshot = parse_snapshot(&text).expect("the checker accepts it");
    assert_eq!(snapshot.network(), Network::Testnet);
    assert_eq!(snapshot.provenance().as_of_ledger(), 5_240_432);
    assert_eq!(snapshot.provenance().captured_at(), "2026-10-17T08:29:07Z");
    assert_eq!(snapshot.provenance().protocol_version(), 29);
    assert_eq!(snapshot.state().additional_signer_count(), 2);
}

#[test]
fn it_reads_the_root_then_the_account_then_the_root_and_nothing_else() {
    let server = Server::start(routes(good_account(), vec![testnet_root(29)]));
    fetch(&server, &account_id(), &quick()).expect("fetches");

    let requests = server.requests();
    let sent: Vec<(&str, &str)> = requests
        .iter()
        .map(|r| (r.method.as_str(), r.path.as_str()))
        .collect();
    let account_path = format!("/accounts/{}", account_id());
    assert_eq!(
        sent,
        [("GET", "/"), ("GET", account_path.as_str()), ("GET", "/")]
    );
}

#[test]
fn it_identifies_itself_and_sends_nothing_that_could_carry_a_secret() {
    let server = Server::start(routes(good_account(), vec![testnet_root(29)]));
    fetch(&server, &account_id(), &quick()).expect("fetches");

    for request in server.requests() {
        let agent = request.header("user-agent").expect("a user agent");
        assert!(agent.starts_with("signerreach/"), "{agent}");
        for name in [
            "authorization",
            "proxy-authorization",
            "cookie",
            "accept-encoding",
            "referer",
            "origin",
        ] {
            assert!(request.header(name).is_none(), "sent a {name} header");
        }
    }
}

#[test]
fn the_same_answers_give_the_same_snapshot() {
    let server = Server::start(routes(good_account(), vec![testnet_root(29)]));
    let first = fetch(&server, &account_id(), &quick()).unwrap();
    let second = fetch(&server, &account_id(), &quick()).unwrap();
    assert_eq!(first, second);
}

#[test]
fn an_account_that_is_not_real_makes_no_request_at_all() {
    let server = Server::start(routes(good_account(), vec![testnet_root(29)]));
    for bad in ["", "not an account", "../../etc/passwd", "G"] {
        assert_eq!(
            fetch(&server, bad, &quick()),
            Err(FetchError::Url(UrlError::NotAnAccount)),
            "{bad:?}"
        );
    }
    assert!(server.requests().is_empty());
}

#[test]
fn a_server_on_the_wrong_network_is_refused_before_the_account_is_asked_for() {
    let pubnet = Reply::ok(&root_json(
        "Public Global Stellar Network ; September 2015",
        29,
    ));
    let server = Server::start(routes(good_account(), vec![pubnet]));
    assert_eq!(
        fetch(&server, &account_id(), &quick()),
        Err(FetchError::Map(MapError::WrongNetwork))
    );
    assert_eq!(server.paths(), ["/"]);
}

#[test]
fn a_protocol_change_during_the_fetch_is_refused() {
    let server = Server::start(routes(
        good_account(),
        vec![testnet_root(29), testnet_root(30)],
    ));
    assert_eq!(
        fetch(&server, &account_id(), &quick()),
        Err(FetchError::ChangedDuringFetch)
    );
    assert_eq!(server.requests().len(), 3);
}

#[test]
fn a_network_name_change_during_the_fetch_is_refused() {
    let other = Reply::ok(&root_json("Standalone Network ; February 2017", 29));
    let server = Server::start(routes(good_account(), vec![testnet_root(29), other]));
    assert_eq!(
        fetch(&server, &account_id(), &quick()),
        Err(FetchError::ChangedDuringFetch)
    );
}

#[test]
fn a_missing_account_is_reported_as_not_found() {
    let server = Server::start(routes(Reply::status(404, "{}"), vec![testnet_root(29)]));
    assert_eq!(
        fetch(&server, &account_id(), &quick()),
        Err(FetchError::NotFound)
    );
}

#[test]
fn other_statuses_are_reported_without_reading_or_repeating_the_body() {
    for code in [400u16, 403, 429, 500, 503] {
        let reply = Reply::status(code, "SECRETMARKER from the server");
        let server = Server::start(routes(reply, vec![testnet_root(29)]));
        let error = fetch(&server, &account_id(), &quick()).expect_err("an error status");
        assert_eq!(error, FetchError::Status(code));
        assert!(!error.to_string().contains("SECRETMARKER"), "{error}");
    }
}

#[test]
fn a_redirect_is_never_followed() {
    let id = account_id();
    let target = Reply::ok(ACCOUNT_JSON)
        .header("Latest-Ledger", "5240432")
        .header("Date", DATE);
    let server = Server::start(move |request: &Request| {
        if request.path == "/" {
            testnet_root(29)
        } else if request.path == format!("/accounts/{id}") {
            Reply::status(301, "").header("Location", "/elsewhere")
        } else {
            target.clone()
        }
    });
    assert_eq!(
        fetch(&server, &account_id(), &quick()),
        Err(FetchError::Redirect)
    );
    assert!(
        !server.paths().iter().any(|path| path == "/elsewhere"),
        "the redirect target was requested"
    );
}

#[test]
fn a_redirect_to_another_host_is_not_followed_either() {
    let other = Server::start(|_: &Request| Reply::ok("{}"));
    let location = format!("http://{}/", other.address);
    let id = account_id();
    let server = Server::start(move |request: &Request| {
        if request.path == "/" {
            testnet_root(29)
        } else if request.path == format!("/accounts/{id}") {
            Reply::status(302, "").header("Location", &location)
        } else {
            Reply::status(404, "")
        }
    });
    assert_eq!(
        fetch(&server, &account_id(), &quick()),
        Err(FetchError::Redirect)
    );
    assert!(other.requests().is_empty(), "the other host was contacted");
}

#[test]
fn an_answer_over_the_size_limit_is_refused() {
    let limits = Limits {
        max_body_bytes: 500,
        ..quick()
    };
    let server = Server::start(routes(good_account(), vec![testnet_root(29)]));
    assert_eq!(
        fetch(&server, &account_id(), &limits),
        Err(FetchError::TooLarge)
    );

    let big_root = Reply::ok(&format!("{{\"padding\":\"{}\"}}", "x".repeat(2000)));
    let server = Server::start(routes(good_account(), vec![big_root]));
    assert_eq!(
        fetch(&server, &account_id(), &quick_with(500)),
        Err(FetchError::TooLarge)
    );
}

fn quick_with(max: u64) -> Limits {
    Limits {
        max_body_bytes: max,
        ..quick()
    }
}

#[test]
fn an_answer_that_is_not_json_is_refused() {
    let server = Server::start(routes(
        Reply::ok("<html>sign in to the wifi</html>")
            .header("Latest-Ledger", "5240432")
            .header("Date", DATE),
        vec![testnet_root(29)],
    ));
    assert_eq!(
        fetch(&server, &account_id(), &quick()),
        Err(FetchError::Map(MapError::NotJson))
    );
}

#[test]
fn the_ledger_and_date_headers_are_required_and_strict() {
    let cases: [(Reply, FetchError); 4] = [
        (
            Reply::ok(ACCOUNT_JSON).header("Date", DATE),
            FetchError::MissingHeader("Latest-Ledger"),
        ),
        (
            Reply::ok(ACCOUNT_JSON).header("Latest-Ledger", "5240432"),
            FetchError::MissingHeader("Date"),
        ),
        (
            Reply::ok(ACCOUNT_JSON)
                .header("Latest-Ledger", "SECRETMARKER")
                .header("Date", DATE),
            FetchError::Map(MapError::BadLedger),
        ),
        (
            Reply::ok(ACCOUNT_JSON)
                .header("Latest-Ledger", "5240432")
                .header("Date", "Sun, 17 Oct 2026 08:29:07 GMT"),
            FetchError::Map(MapError::BadDate),
        ),
    ];
    for (reply, expected) in cases {
        let server = Server::start(routes(reply, vec![testnet_root(29)]));
        let error = fetch(&server, &account_id(), &quick()).expect_err("refused");
        assert_eq!(error, expected);
        assert!(!error.to_string().contains("SECRETMARKER"));
    }
}

#[test]
fn a_slow_server_times_out() {
    let limits = Limits {
        total_timeout: Duration::from_millis(300),
        ..quick()
    };
    let slow = good_account().delayed(Duration::from_millis(1500));
    let server = Server::start(routes(slow, vec![testnet_root(29)]));
    assert_eq!(
        fetch(&server, &account_id(), &limits),
        Err(FetchError::Timeout)
    );
}

#[test]
fn an_answer_cut_short_is_not_accepted() {
    let cut = good_account().claiming(ACCOUNT_JSON.len() + 500);
    let server = Server::start(routes(cut, vec![testnet_root(29)]));
    assert!(fetch(&server, &account_id(), &quick()).is_err());
}

#[test]
fn a_server_that_is_not_there_is_a_connection_error() {
    let address = {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        listener.local_addr().expect("address")
    };
    let url = HorizonUrl::parse(&format!("http://{address}")).expect("a local address");
    // Linux and macOS refuse a closed port at once. Windows keeps retrying for a
    // couple of seconds, so the connect time limit can run out first. Either way
    // the server could not be reached and no snapshot is produced.
    let result = fetch_snapshot(Network::Testnet, &url, &account_id(), &quick());
    assert!(
        matches!(result, Err(FetchError::Connect | FetchError::Timeout)),
        "{result:?}"
    );
}

#[test]
fn the_public_addresses_are_reachable_only_over_https() {
    assert!(HorizonUrl::for_network(Network::Testnet).is_secure());
    assert!(HorizonUrl::for_network(Network::Pubnet).is_secure());
    assert_eq!(
        HorizonUrl::parse("http://horizon-testnet.stellar.org"),
        Err(UrlError::NotSecure)
    );
}
