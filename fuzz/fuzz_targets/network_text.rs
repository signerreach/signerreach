//! The small text parsers in the network layer: the server address, the `Date`
//! header, the server root and the `Latest-Ledger` header.
//!
//! The input is four values separated by NUL bytes, in that order. Besides not
//! panicking, the properties checked are:
//!
//! - an address that is accepted gives back an address that is accepted again,
//!   and the same one;
//! - a date that is accepted is written as `YYYY-MM-DDTHH:MM:SSZ`;
//! - a ledger that is accepted is at least 1.

#![no_main]

use libfuzzer_sys::fuzz_target;
use signerreach_core::Network;
use signerreach_stellar::{
    HorizonUrl, RootInfo, http_date_to_rfc3339, parse_latest_ledger, passphrase_of,
};

fuzz_target!(|data: &[u8]| {
    let mut parts = data.splitn(4, |byte| *byte == 0);
    let (Some(url), Some(date), Some(root), Some(ledger)) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return;
    };
    let (Ok(url), Ok(date), Ok(root), Ok(ledger)) = (
        std::str::from_utf8(url),
        std::str::from_utf8(date),
        std::str::from_utf8(root),
        std::str::from_utf8(ledger),
    ) else {
        return;
    };

    if let Ok(parsed) = HorizonUrl::parse(url) {
        assert_eq!(HorizonUrl::parse(&parsed.base()), Ok(parsed.clone()));
        assert_eq!(HorizonUrl::parse(&parsed.root_url()), Ok(parsed.clone()));
        let _ = parsed.account_url(url);
    }

    if let Ok(stamp) = http_date_to_rfc3339(date) {
        let bytes = stamp.as_bytes();
        assert_eq!(bytes.len(), 20, "unexpected timestamp length: {stamp}");
        assert_eq!(bytes[10], b'T');
        assert_eq!(bytes[19], b'Z');
    }

    if let Ok(info) = RootInfo::parse(root) {
        let _ = info.check_network(Network::Testnet);
        let _ = info.check_network(Network::Pubnet);
        let _ = (passphrase_of(Network::Testnet), passphrase_of(Network::Pubnet));
    }

    if let Ok(number) = parse_latest_ledger(ledger) {
        assert!(number >= 1);
    }
});
