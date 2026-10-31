//! What a Horizon server says about itself, and the ledger number in a response.
//!
//! The server root describes the network it serves: its name (the "network
//! passphrase") and its protocol version. Before an account record is trusted,
//! the network is checked against the one that was asked for, so that a
//! snapshot labelled `testnet` cannot hold data from the public network, or the
//! reverse. The protocol version is recorded in the snapshot, and the checker
//! refuses any version whose rules have not been verified.

use serde_json::Value;
use signerreach_core::Network;

use crate::error::MapError;
use crate::strict_json;

/// The name by which a network identifies itself. These are the published values
/// and were confirmed against the live servers on 2026-10-18.
pub const fn passphrase_of(network: Network) -> &'static str {
    match network {
        Network::Testnet => "Test SDF Network ; September 2015",
        Network::Pubnet => "Public Global Stellar Network ; September 2015",
    }
}

/// The two facts taken from the server root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RootInfo {
    /// The network name the server reports.
    pub passphrase: String,
    /// The protocol version the network is running.
    pub protocol_version: u32,
}

impl RootInfo {
    /// Reads the server root document.
    pub fn parse(text: &str) -> Result<Self, MapError> {
        let root: Value = strict_json::parse(text)?;
        let passphrase = root
            .get("network_passphrase")
            .ok_or(MapError::MissingField("network_passphrase"))?
            .as_str()
            .ok_or(MapError::BadField("network_passphrase"))?
            .to_string();
        let protocol_version = root
            .get("current_protocol_version")
            .ok_or(MapError::MissingField("current_protocol_version"))?
            .as_u64()
            .and_then(|version| u32::try_from(version).ok())
            .ok_or(MapError::BadField("current_protocol_version"))?;
        Ok(Self {
            passphrase,
            protocol_version,
        })
    }

    /// Confirms the server is on the network that was asked for.
    pub fn check_network(&self, expected: Network) -> Result<(), MapError> {
        if self.passphrase == passphrase_of(expected) {
            Ok(())
        } else {
            Err(MapError::WrongNetwork)
        }
    }
}

/// Reads the `Latest-Ledger` header: plain digits, a whole number of at least 1.
///
/// The value must already be trimmed. Anything else, including a sign, a space
/// or a leading `+`, is refused.
pub fn parse_latest_ledger(value: &str) -> Result<u64, MapError> {
    if value.is_empty() || !value.bytes().all(|b| b.is_ascii_digit()) {
        return Err(MapError::BadLedger);
    }
    match value.parse::<u64>() {
        Ok(ledger) if ledger >= 1 => Ok(ledger),
        _ => Err(MapError::BadLedger),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root(passphrase: &str, version: &str) -> String {
        format!(
            r#"{{"horizon_version":"29.0.0","network_passphrase":"{passphrase}",
            "current_protocol_version":{version},"history_latest_ledger":5240283,"extra":[1,2]}}"#
        )
    }

    #[test]
    fn the_published_passphrases_are_the_ones_the_servers_send() {
        assert_eq!(
            passphrase_of(Network::Testnet),
            "Test SDF Network ; September 2015"
        );
        assert_eq!(
            passphrase_of(Network::Pubnet),
            "Public Global Stellar Network ; September 2015"
        );
    }

    #[test]
    fn a_real_looking_root_document_is_read() {
        let info = RootInfo::parse(&root("Test SDF Network ; September 2015", "29")).unwrap();
        assert_eq!(info.passphrase, "Test SDF Network ; September 2015");
        assert_eq!(info.protocol_version, 29);
    }

    #[test]
    fn duplicate_root_fields_are_refused() {
        let text = r#"{"network_passphrase":"Test SDF Network ; September 2015","current_protocol_version":29,"current_protocol_version":30}"#;
        assert!(RootInfo::parse(text).is_err());
        let nested = r#"{"network_passphrase":"Test SDF Network ; September 2015","current_protocol_version":29,"extra":{"x":1,"x":2}}"#;
        assert!(RootInfo::parse(nested).is_err());
    }

    #[test]
    fn the_network_is_checked_against_the_one_asked_for() {
        let testnet = RootInfo::parse(&root(passphrase_of(Network::Testnet), "29")).unwrap();
        assert_eq!(testnet.check_network(Network::Testnet), Ok(()));
        assert_eq!(
            testnet.check_network(Network::Pubnet),
            Err(MapError::WrongNetwork)
        );
        let pubnet = RootInfo::parse(&root(passphrase_of(Network::Pubnet), "29")).unwrap();
        assert_eq!(pubnet.check_network(Network::Pubnet), Ok(()));
        assert_eq!(
            pubnet.check_network(Network::Testnet),
            Err(MapError::WrongNetwork)
        );
    }

    #[test]
    fn a_private_or_lookalike_network_is_not_either_public_network() {
        for name in [
            "",
            "Standalone Network ; February 2017",
            "test sdf network ; september 2015",
            "Test SDF Network ; September 2015 ",
            "Test SDF Network ; September 2015\n",
        ] {
            // Encoded as JSON, so a trailing newline really is part of the name.
            let document = serde_json::json!({
                "network_passphrase": name,
                "current_protocol_version": 29,
            })
            .to_string();
            let info = RootInfo::parse(&document).unwrap();
            assert_eq!(info.passphrase, name);
            let expected_ok = name == "Test SDF Network ; September 2015";
            assert_eq!(
                info.check_network(Network::Testnet).is_ok(),
                expected_ok,
                "{name:?}"
            );
            assert!(info.check_network(Network::Pubnet).is_err(), "{name:?}");
        }
    }

    #[test]
    fn missing_or_mistyped_fields_are_named() {
        assert_eq!(RootInfo::parse("not json"), Err(MapError::NotJson));
        assert_eq!(
            RootInfo::parse("{}"),
            Err(MapError::MissingField("network_passphrase"))
        );
        assert_eq!(
            RootInfo::parse(r#"{"network_passphrase":"x"}"#),
            Err(MapError::MissingField("current_protocol_version"))
        );
        assert_eq!(
            RootInfo::parse(r#"{"network_passphrase":7,"current_protocol_version":29}"#),
            Err(MapError::BadField("network_passphrase"))
        );
        for bad in ["\"29\"", "-1", "29.5", "4294967296", "null"] {
            assert_eq!(
                RootInfo::parse(&root("x", bad)),
                Err(MapError::BadField("current_protocol_version")),
                "{bad}"
            );
        }
    }

    #[test]
    fn a_root_document_can_be_compared_before_and_after() {
        let a = RootInfo::parse(&root(passphrase_of(Network::Testnet), "29")).unwrap();
        let b = RootInfo::parse(&root(passphrase_of(Network::Testnet), "29")).unwrap();
        let c = RootInfo::parse(&root(passphrase_of(Network::Testnet), "30")).unwrap();
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn the_ledger_header_is_plain_digits_of_at_least_one() {
        assert_eq!(parse_latest_ledger("5240291"), Ok(5_240_291));
        assert_eq!(parse_latest_ledger("1"), Ok(1));
        assert_eq!(parse_latest_ledger("64988002"), Ok(64_988_002));
        assert_eq!(parse_latest_ledger("18446744073709551615"), Ok(u64::MAX));
        for bad in [
            "",
            "0",
            "00",
            "-1",
            "+5",
            " 5",
            "5 ",
            "5.0",
            "5e3",
            "0x10",
            "abc",
            "18446744073709551616",
            "\u{661}",
        ] {
            let result = parse_latest_ledger(bad);
            // "00" is all digits but equals 0, which is not a ledger.
            assert_eq!(result, Err(MapError::BadLedger), "{bad:?}");
        }
    }

    #[test]
    fn error_messages_do_not_repeat_what_the_server_sent() {
        let error = RootInfo::parse(&root("SECRETMARKER", "29"))
            .unwrap()
            .check_network(Network::Testnet)
            .expect_err("wrong network");
        assert!(!error.to_string().contains("SECRETMARKER"));
        let error = parse_latest_ledger("SECRETMARKER").expect_err("bad");
        assert!(!error.to_string().contains("SECRETMARKER"));
    }
}
