//! Diagnostic codes, as defined in `docs/SPEC.md`, section 7.

use std::fmt;

/// What a finding does to the verdict (`docs/SPEC.md`, section 7).
///
/// Ordered by severity, so the worst effect of a set of findings is the
/// largest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Effect {
    /// Reported for information; the verdict is unchanged.
    Info,
    /// The result cannot be established.
    Inconclusive,
    /// The plan fails or access is lost.
    Unsafe,
}

/// A stable identifier for one kind of finding.
///
/// The strings are part of the tool's public output and do not change once
/// released.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Code {
    /// `SR-AUTH-01`: a transaction or operation is not authorized by the state before it.
    Auth01,
    /// `SR-TX-01`: a signature matches no signer of the pre-transaction state.
    Tx01,
    /// `SR-TX-02`: a signature is not necessary, so rejection depends on signature order.
    Tx02,
    /// `SR-TX-03`: LOW threshold unreachable; administration needs another funded source.
    Tx03,
    /// `SR-TX-04`: fee-bump, preconditions, or another source account.
    Tx04,
    /// `SR-APPLY-01`: an authorized transaction would fail when applied.
    Apply01,
    /// `SR-LOCK-01`: total weight of all signers is below the HIGH threshold.
    Lock01,
    /// `SR-LOSS-01`: HIGH is unreachable with the signers declared available.
    Loss01,
    /// `SR-LOCK-02`: MEDIUM unreachable (informational).
    Lock02,
    /// `SR-AVAIL-01`: the outcome depends on signers whose availability is unknown.
    Avail01,
    /// `SR-SIGNER-01`: a signer type other than ed25519 is present.
    Signer01,
    /// `SR-OP-01`: an operation type that is not supported.
    Op01,
    /// `SR-INPUT-01`: invalid, incomplete or inconsistent input.
    Input01,
    /// `SR-INPUT-02`: unsupported account kind (contract, muxed).
    Input02,
    /// `SR-FRESH-01`: snapshot provenance missing.
    Fresh01,
    /// `SR-RULES-01`: protocol or rule version not supported.
    Rules01,
    /// `SR-STATE-01`: sponsorship or reserve conditions are not modelled.
    State01,
    /// `SR-INTENT-01`: plan declared irreversible; presentation only.
    Intent01,
}

impl Code {
    /// Every code, in the order of the specification.
    pub const ALL: [Code; 18] = [
        Code::Auth01,
        Code::Tx01,
        Code::Tx02,
        Code::Tx03,
        Code::Tx04,
        Code::Apply01,
        Code::Lock01,
        Code::Loss01,
        Code::Lock02,
        Code::Avail01,
        Code::Signer01,
        Code::Op01,
        Code::Input01,
        Code::Input02,
        Code::Fresh01,
        Code::Rules01,
        Code::State01,
        Code::Intent01,
    ];

    /// What the code does to the verdict.
    pub const fn effect(self) -> Effect {
        match self {
            Code::Auth01 | Code::Tx01 | Code::Apply01 | Code::Lock01 | Code::Loss01 => {
                Effect::Unsafe
            }
            Code::Tx02
            | Code::Tx03
            | Code::Tx04
            | Code::Avail01
            | Code::Signer01
            | Code::Op01
            | Code::Input01
            | Code::Input02
            | Code::Fresh01
            | Code::Rules01
            | Code::State01 => Effect::Inconclusive,
            Code::Lock02 | Code::Intent01 => Effect::Info,
        }
    }

    /// The public identifier, for example `SR-AUTH-01`.
    pub const fn as_str(self) -> &'static str {
        match self {
            Code::Auth01 => "SR-AUTH-01",
            Code::Tx01 => "SR-TX-01",
            Code::Tx02 => "SR-TX-02",
            Code::Tx03 => "SR-TX-03",
            Code::Tx04 => "SR-TX-04",
            Code::Apply01 => "SR-APPLY-01",
            Code::Lock01 => "SR-LOCK-01",
            Code::Loss01 => "SR-LOSS-01",
            Code::Lock02 => "SR-LOCK-02",
            Code::Avail01 => "SR-AVAIL-01",
            Code::Signer01 => "SR-SIGNER-01",
            Code::Op01 => "SR-OP-01",
            Code::Input01 => "SR-INPUT-01",
            Code::Input02 => "SR-INPUT-02",
            Code::Fresh01 => "SR-FRESH-01",
            Code::Rules01 => "SR-RULES-01",
            Code::State01 => "SR-STATE-01",
            Code::Intent01 => "SR-INTENT-01",
        }
    }
}

impl fmt::Display for Code {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn identifiers_are_unique_and_well_formed() {
        let names: BTreeSet<&str> = Code::ALL.iter().map(|c| c.as_str()).collect();
        assert_eq!(names.len(), Code::ALL.len());
        for name in names {
            assert!(name.starts_with("SR-"), "{name}");
            let last = name.rsplit('-').next().expect("suffix");
            assert_eq!(last.len(), 2, "{name}");
            assert!(last.chars().all(|c| c.is_ascii_digit()), "{name}");
        }
    }

    #[test]
    fn effects_match_the_specification_table() {
        let unsafe_codes = [
            Code::Auth01,
            Code::Tx01,
            Code::Apply01,
            Code::Lock01,
            Code::Loss01,
        ];
        let info_codes = [Code::Lock02, Code::Intent01];
        for code in Code::ALL {
            let expected = if unsafe_codes.contains(&code) {
                Effect::Unsafe
            } else if info_codes.contains(&code) {
                Effect::Info
            } else {
                Effect::Inconclusive
            };
            assert_eq!(code.effect(), expected, "{code}");
        }
        assert!(Effect::Unsafe > Effect::Inconclusive);
        assert!(Effect::Inconclusive > Effect::Info);
    }

    #[test]
    fn display_matches_the_identifier() {
        assert_eq!(Code::Loss01.to_string(), "SR-LOSS-01");
        assert_eq!(Code::Input02.to_string(), "SR-INPUT-02");
    }
}
