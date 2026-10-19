//! The address of a Horizon server, and the two addresses built from it.
//!
//! The rules are deliberately narrow, because the address decides where the
//! tool sends a request:
//!
//! - The scheme is `https`. Plain `http` is accepted only for a server on the
//!   same machine (`localhost`, `127.0.0.1`, `[::1]`), which is what the tests
//!   and a local development network use.
//! - The address is a host name and an optional port. A path, a query, a
//!   fragment, a user name or a password is refused.
//!
//! Only these two addresses are ever built: the server root, and one account
//! under it. The account is checked to be a real `G...` address first, so
//! nothing a caller types can reach the path in any other form.

use signerreach_core::{Network, SignerKey};

/// Why an address was refused.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum UrlError {
    /// The scheme is not `https`, and the host is not on this machine.
    #[error(
        "the address must start with https:// (plain http is allowed only for localhost, \
         127.0.0.1 and [::1])"
    )]
    NotSecure,

    /// The address is not a plain host name with an optional port.
    #[error("the address must be a host name with an optional port, and nothing after it")]
    Malformed,

    /// The account is not an ed25519 account address (`G...`).
    #[error("the account is not an ed25519 account address (a G... address)")]
    NotAnAccount,
}

/// A validated server address, such as `https://horizon-testnet.stellar.org`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HorizonUrl {
    secure: bool,
    host: String,
    port: Option<u16>,
}

impl HorizonUrl {
    /// The public server for a network.
    pub fn for_network(network: Network) -> Self {
        let host = match network {
            Network::Testnet => "horizon-testnet.stellar.org",
            Network::Pubnet => "horizon.stellar.org",
        };
        Self {
            secure: true,
            host: host.to_string(),
            port: None,
        }
    }

    /// Checks an address given by the user.
    pub fn parse(text: &str) -> Result<Self, UrlError> {
        let (secure, rest) = if let Some(rest) = text.strip_prefix("https://") {
            (true, rest)
        } else if let Some(rest) = text.strip_prefix("http://") {
            (false, rest)
        } else {
            return Err(UrlError::NotSecure);
        };

        // One trailing slash is allowed; anything else after the host is not.
        let authority = rest.strip_suffix('/').unwrap_or(rest);
        if authority.is_empty()
            || !authority
                .bytes()
                .all(|b| b.is_ascii_graphic() && !matches!(b, b'/' | b'?' | b'#' | b'@' | b'\\'))
        {
            return Err(UrlError::Malformed);
        }

        let (host, port) = split_host_port(authority)?;
        let host = host.to_ascii_lowercase();
        if !secure && !is_local(&host) {
            return Err(UrlError::NotSecure);
        }
        Ok(Self { secure, host, port })
    }

    /// The address without a trailing slash, for example
    /// `https://horizon-testnet.stellar.org`.
    pub fn base(&self) -> String {
        let scheme = if self.secure { "https" } else { "http" };
        match self.port {
            Some(port) => format!("{scheme}://{}:{port}", self.host),
            None => format!("{scheme}://{}", self.host),
        }
    }

    /// Whether the connection is encrypted.
    pub const fn is_secure(&self) -> bool {
        self.secure
    }

    /// The server root, which describes the network.
    pub fn root_url(&self) -> String {
        format!("{}/", self.base())
    }

    /// One account's record. The account is checked, so it cannot change the path.
    pub fn account_url(&self, account: &str) -> Result<String, UrlError> {
        SignerKey::from_strkey(account).map_err(|_| UrlError::NotAnAccount)?;
        Ok(format!("{}/accounts/{account}", self.base()))
    }
}

/// Splits `host`, `host:port` or `[::1]` / `[::1]:port`.
fn split_host_port(authority: &str) -> Result<(&str, Option<u16>), UrlError> {
    if let Some(after) = authority.strip_prefix('[') {
        // The only bracketed host accepted is the IPv6 loopback address.
        let (inside, tail) = after.split_once(']').ok_or(UrlError::Malformed)?;
        if inside != "::1" {
            return Err(UrlError::Malformed);
        }
        let port = match tail {
            "" => None,
            _ => Some(port(tail.strip_prefix(':').ok_or(UrlError::Malformed)?)?),
        };
        return Ok(("[::1]", port));
    }
    match authority.split_once(':') {
        None => host(authority).map(|host| (host, None)),
        Some((name, number)) => Ok((host(name)?, Some(port(number)?))),
    }
}

/// A host name: dot-separated labels of letters, digits and hyphens, none
/// empty, none starting or ending with a hyphen.
fn host(name: &str) -> Result<&str, UrlError> {
    let valid = !name.is_empty()
        && name.len() <= 253
        && name.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-')
        });
    if valid {
        Ok(name)
    } else {
        Err(UrlError::Malformed)
    }
}

/// A port: one to five plain digits, from 1 to 65535.
fn port(text: &str) -> Result<u16, UrlError> {
    if text.is_empty() || text.len() > 5 || !text.bytes().all(|b| b.is_ascii_digit()) {
        return Err(UrlError::Malformed);
    }
    match text.parse::<u16>() {
        Ok(0) | Err(_) => Err(UrlError::Malformed),
        Ok(port) => Ok(port),
    }
}

fn is_local(host: &str) -> bool {
    matches!(host, "localhost" | "127.0.0.1" | "[::1]")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn account() -> String {
        SignerKey::from_bytes([1; 32]).to_string()
    }

    #[test]
    fn the_public_servers_are_the_known_https_addresses() {
        assert_eq!(
            HorizonUrl::for_network(Network::Testnet).base(),
            "https://horizon-testnet.stellar.org"
        );
        assert_eq!(
            HorizonUrl::for_network(Network::Pubnet).base(),
            "https://horizon.stellar.org"
        );
        assert!(HorizonUrl::for_network(Network::Pubnet).is_secure());
    }

    #[test]
    fn https_addresses_are_accepted_and_normalised() {
        for (given, base) in [
            ("https://horizon.stellar.org", "https://horizon.stellar.org"),
            (
                "https://horizon.stellar.org/",
                "https://horizon.stellar.org",
            ),
            ("https://Horizon.Stellar.ORG", "https://horizon.stellar.org"),
            ("https://example.org:8443", "https://example.org:8443"),
            ("https://example.org:8443/", "https://example.org:8443"),
            ("https://10.0.0.5", "https://10.0.0.5"),
            ("https://a-b.c-d.example", "https://a-b.c-d.example"),
        ] {
            let parsed = HorizonUrl::parse(given).expect(given);
            assert_eq!(parsed.base(), base, "{given}");
            assert!(parsed.is_secure());
        }
    }

    #[test]
    fn plain_http_is_allowed_only_for_this_machine() {
        for (given, base) in [
            ("http://localhost", "http://localhost"),
            ("http://localhost:8000", "http://localhost:8000"),
            ("http://127.0.0.1:12345/", "http://127.0.0.1:12345"),
            ("http://[::1]:9000", "http://[::1]:9000"),
            ("http://LOCALHOST", "http://localhost"),
        ] {
            let parsed = HorizonUrl::parse(given).expect(given);
            assert_eq!(parsed.base(), base, "{given}");
            assert!(!parsed.is_secure());
        }
        for given in [
            "http://horizon.stellar.org",
            "http://10.0.0.5",
            "http://127.0.0.2",
            "http://localhost.evil.example",
            "http://[2001:db8::1]",
        ] {
            let error = HorizonUrl::parse(given).expect_err(given);
            assert!(
                matches!(error, UrlError::NotSecure | UrlError::Malformed),
                "{given}: {error}"
            );
        }
        assert_eq!(
            HorizonUrl::parse("http://horizon.stellar.org"),
            Err(UrlError::NotSecure)
        );
    }

    #[test]
    fn other_schemes_and_missing_schemes_are_refused() {
        for given in [
            "",
            "horizon.stellar.org",
            "ftp://horizon.stellar.org",
            "file:///etc/passwd",
            "HTTPS://horizon.stellar.org",
            "//horizon.stellar.org",
            "https:/horizon.stellar.org",
        ] {
            assert_eq!(
                HorizonUrl::parse(given),
                Err(UrlError::NotSecure),
                "{given:?}"
            );
        }
    }

    #[test]
    fn anything_after_the_host_is_refused() {
        for given in [
            "https://",
            "https:///",
            "https://horizon.stellar.org/accounts",
            "https://horizon.stellar.org//",
            "https://horizon.stellar.org?x=1",
            "https://horizon.stellar.org#frag",
            "https://user@horizon.stellar.org",
            "https://user:pass@horizon.stellar.org",
            "https://horizon.stellar.org\\evil",
            "https://horizon stellar.org",
            "https://horizon.stellar.org\n",
            "https://horizon.stellar.org\t",
            "https://h\u{f6}rizon.example",
        ] {
            assert_eq!(
                HorizonUrl::parse(given),
                Err(UrlError::Malformed),
                "{given:?}"
            );
        }
    }

    #[test]
    fn badly_formed_hosts_and_ports_are_refused() {
        for given in [
            "https://.example",
            "https://example.",
            "https://exa..mple",
            "https://-example.org",
            "https://example-.org",
            "https://exa_mple.org",
            "https://example.org:",
            "https://example.org:0",
            "https://example.org:65536",
            "https://example.org:+80",
            "https://example.org:080808",
            "https://example.org:8o",
            "https://example.org:80:80",
            "https://[::1",
            "https://[::2]",
            "https://[::1]x",
            "https://[::1]:",
            &format!("https://{}.example", "a".repeat(64)),
        ] {
            assert_eq!(
                HorizonUrl::parse(given),
                Err(UrlError::Malformed),
                "{given:?}"
            );
        }
    }

    #[test]
    fn the_two_built_addresses_have_the_expected_shape() {
        let url = HorizonUrl::parse("https://example.org:8443/").unwrap();
        assert_eq!(url.root_url(), "https://example.org:8443/");
        let id = account();
        assert_eq!(
            url.account_url(&id).unwrap(),
            format!("https://example.org:8443/accounts/{id}")
        );
    }

    #[test]
    fn an_account_that_is_not_a_real_address_cannot_reach_the_path() {
        let url = HorizonUrl::for_network(Network::Testnet);
        for bad in [
            "",
            "../../etc/passwd",
            "G",
            "accounts/../x",
            "GAAQ/../",
            "GAAQCAIBAEAQCAIBAEAQCAIBAEAQCAIBAEAQCAIBAEAQCAIBAEAQDZ7I",
        ] {
            assert_eq!(url.account_url(bad), Err(UrlError::NotAnAccount), "{bad:?}");
        }
        // A contract address is well formed but is not an account.
        assert_eq!(
            url.account_url("CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABSC4"),
            Err(UrlError::NotAnAccount)
        );
    }

    #[test]
    fn error_messages_do_not_repeat_the_address() {
        let secret = "https://user:SECRETMARKER@host.example";
        let error = HorizonUrl::parse(secret).expect_err("userinfo");
        assert!(!error.to_string().contains("SECRETMARKER"));
        let error = HorizonUrl::for_network(Network::Pubnet)
            .account_url("SECRETMARKER")
            .expect_err("not an account");
        assert!(!error.to_string().contains("SECRETMARKER"));
    }
}
