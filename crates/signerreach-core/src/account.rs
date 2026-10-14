//! The classic-account model: keys, weights, thresholds and account state.
//!
//! Rules referenced as R1 and R3 are defined in `docs/SPEC.md`, section 4.
//!
//! Every constructor refuses a value that Stellar could not hold, instead of
//! clamping or correcting it. A model that silently repairs its input could
//! report a state that does not exist.

use std::collections::BTreeMap;
use std::fmt;
use std::str::FromStr;

use crate::error::DomainError;

/// The most additional signers (besides the master key) an account can hold (R1).
pub const MAX_ADDITIONAL_SIGNERS: usize = 20;

/// A signer or master weight, 0 to 255 (R1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Weight(u8);

impl Weight {
    /// Weight 0: cannot sign.
    pub const ZERO: Weight = Weight(0);

    /// The largest weight.
    pub const MAX: Weight = Weight(u8::MAX);

    /// Builds a weight, refusing anything above 255.
    pub fn new(value: u32) -> Result<Self, DomainError> {
        u8::try_from(value)
            .map(Weight)
            .map_err(|_| DomainError::WeightOutOfRange(value))
    }

    /// The weight as a number.
    pub const fn get(self) -> u8 {
        self.0
    }

    /// Whether the weight is 0.
    pub const fn is_zero(self) -> bool {
        self.0 == 0
    }
}

/// An operation threshold, 0 to 255 (R1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Threshold(u8);

impl Threshold {
    /// Builds a threshold, refusing anything above 255.
    pub fn new(value: u32) -> Result<Self, DomainError> {
        u8::try_from(value)
            .map(Threshold)
            .map_err(|_| DomainError::ThresholdOutOfRange(value))
    }

    /// The threshold as a number.
    pub const fn get(self) -> u8 {
        self.0
    }
}

/// Sums weights as a number that cannot overflow.
///
/// Each weight is at most 255 (R3), so this is the weight a set of distinct
/// signers contributes toward a threshold.
pub fn sum_weights<I>(weights: I) -> u32
where
    I: IntoIterator<Item = Weight>,
{
    weights.into_iter().map(|w| u32::from(w.get())).sum()
}

/// An ed25519 public key that identifies an account or one of its signers.
///
/// Ordered by its bytes, so iteration and reports are deterministic.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SignerKey(stellar_strkey::ed25519::PublicKey);

impl SignerKey {
    /// Builds a key from its 32 raw bytes.
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(stellar_strkey::ed25519::PublicKey(bytes))
    }

    /// Parses a `G...` address. Any other kind of address is refused.
    pub fn from_strkey(text: &str) -> Result<Self, DomainError> {
        stellar_strkey::ed25519::PublicKey::from_string(text)
            .map(Self)
            .map_err(DomainError::InvalidAccountKey)
    }

    /// The 32 raw bytes.
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0.0
    }
}

impl fmt::Display for SignerKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl fmt::Debug for SignerKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SignerKey({})", self.0)
    }
}

impl FromStr for SignerKey {
    type Err = DomainError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::from_strkey(text)
    }
}

/// Which threshold an operation needs (R4, R5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ThresholdClass {
    /// Envelope and low-impact operations.
    Low,
    /// Payments and most operations.
    Medium,
    /// Signer, threshold and master-weight changes.
    High,
}

impl ThresholdClass {
    /// Every class, in ascending order.
    pub const ALL: [ThresholdClass; 3] = [
        ThresholdClass::Low,
        ThresholdClass::Medium,
        ThresholdClass::High,
    ];
}

impl fmt::Display for ThresholdClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            ThresholdClass::Low => "LOW",
            ThresholdClass::Medium => "MEDIUM",
            ThresholdClass::High => "HIGH",
        })
    }
}

/// The three thresholds of an account.
///
/// No ordering between them is enforced: Stellar allows any combination, and
/// an unusual combination is reported as a warning, not refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Thresholds {
    low: Threshold,
    medium: Threshold,
    high: Threshold,
}

impl Thresholds {
    /// Builds the three thresholds.
    pub const fn new(low: Threshold, medium: Threshold, high: Threshold) -> Self {
        Self { low, medium, high }
    }

    /// The threshold for a class.
    pub const fn get(&self, class: ThresholdClass) -> Threshold {
        match class {
            ThresholdClass::Low => self.low,
            ThresholdClass::Medium => self.medium,
            ThresholdClass::High => self.high,
        }
    }
}

/// The signer configuration of one classic account at one moment.
///
/// Invariants, enforced by [`AccountState::new`]:
/// - no additional signer is the account's own key (the master key is carried
///   by `master_weight`);
/// - no signer is listed twice;
/// - every additional signer has a weight above 0 (a weight of 0 removes a
///   signer, so it is never part of a state);
/// - there are at most [`MAX_ADDITIONAL_SIGNERS`] additional signers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountState {
    account: SignerKey,
    master_weight: Weight,
    thresholds: Thresholds,
    signers: BTreeMap<SignerKey, Weight>,
}

impl AccountState {
    /// Builds a state, refusing anything Stellar could not hold.
    ///
    /// Errors are reported for the first offending signer in input order, then
    /// the count limit, so the same input always gives the same error.
    pub fn new<I>(
        account: SignerKey,
        master_weight: Weight,
        thresholds: Thresholds,
        additional_signers: I,
    ) -> Result<Self, DomainError>
    where
        I: IntoIterator<Item = (SignerKey, Weight)>,
    {
        let mut signers = BTreeMap::new();
        for (key, weight) in additional_signers {
            if key == account {
                return Err(DomainError::SignerIsAccount(key));
            }
            if weight.is_zero() {
                return Err(DomainError::ZeroWeightSigner(key));
            }
            if signers.insert(key, weight).is_some() {
                return Err(DomainError::DuplicateSigner(key));
            }
        }
        if signers.len() > MAX_ADDITIONAL_SIGNERS {
            return Err(DomainError::TooManySigners {
                count: signers.len(),
                max: MAX_ADDITIONAL_SIGNERS,
            });
        }
        Ok(Self {
            account,
            master_weight,
            thresholds,
            signers,
        })
    }

    /// The account's own key.
    pub const fn account(&self) -> SignerKey {
        self.account
    }

    /// The weight of the account's own key.
    pub const fn master_weight(&self) -> Weight {
        self.master_weight
    }

    /// The three thresholds.
    pub const fn thresholds(&self) -> Thresholds {
        self.thresholds
    }

    /// The threshold for one class.
    pub const fn threshold(&self, class: ThresholdClass) -> Threshold {
        self.thresholds.get(class)
    }

    /// The additional signers in ascending key order.
    pub fn signers(&self) -> impl Iterator<Item = (SignerKey, Weight)> + '_ {
        self.signers.iter().map(|(key, weight)| (*key, *weight))
    }

    /// How many additional signers the account has.
    pub fn additional_signer_count(&self) -> usize {
        self.signers.len()
    }

    /// The weight a key carries on this account: the master weight for the
    /// account's own key, the signer weight for an additional signer, and 0
    /// for any other key.
    pub fn weight_of(&self, key: SignerKey) -> Weight {
        if key == self.account {
            self.master_weight
        } else {
            self.signers.get(&key).copied().unwrap_or(Weight::ZERO)
        }
    }

    /// The combined weight of every key that can sign, regardless of whether
    /// anyone holds those keys (R3).
    pub fn total_weight(&self) -> u32 {
        sum_weights(std::iter::once(self.master_weight).chain(self.signers.values().copied()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(n: u8) -> SignerKey {
        SignerKey::from_bytes([n; 32])
    }

    fn w(n: u32) -> Weight {
        Weight::new(n).expect("valid weight")
    }

    fn t(n: u32) -> Threshold {
        Threshold::new(n).expect("valid threshold")
    }

    fn thresholds(low: u32, medium: u32, high: u32) -> Thresholds {
        Thresholds::new(t(low), t(medium), t(high))
    }

    #[test]
    fn weight_accepts_0_and_255_and_refuses_256() {
        assert_eq!(w(0), Weight::ZERO);
        assert_eq!(w(255), Weight::MAX);
        assert_eq!(Weight::new(256), Err(DomainError::WeightOutOfRange(256)));
        assert_eq!(
            Weight::new(u32::MAX),
            Err(DomainError::WeightOutOfRange(u32::MAX))
        );
    }

    #[test]
    fn threshold_accepts_255_and_refuses_256() {
        assert_eq!(t(255).get(), 255);
        assert_eq!(
            Threshold::new(256),
            Err(DomainError::ThresholdOutOfRange(256))
        );
    }

    #[test]
    fn sum_of_weights_does_not_overflow_a_byte() {
        assert_eq!(sum_weights([w(255), w(255), w(255)]), 765);
        assert_eq!(sum_weights(std::iter::empty()), 0);
    }

    #[test]
    fn thresholds_are_returned_by_class() {
        let th = thresholds(1, 2, 3);
        assert_eq!(th.get(ThresholdClass::Low), t(1));
        assert_eq!(th.get(ThresholdClass::Medium), t(2));
        assert_eq!(th.get(ThresholdClass::High), t(3));
    }

    #[test]
    fn unusual_threshold_order_is_not_refused() {
        let th = thresholds(5, 1, 0);
        assert_eq!(th.get(ThresholdClass::Low), t(5));
    }

    #[test]
    fn classes_are_ordered_and_named() {
        assert!(ThresholdClass::Low < ThresholdClass::Medium);
        assert!(ThresholdClass::Medium < ThresholdClass::High);
        let names: Vec<String> = ThresholdClass::ALL
            .iter()
            .map(ToString::to_string)
            .collect();
        assert_eq!(names, ["LOW", "MEDIUM", "HIGH"]);
    }

    #[test]
    fn key_round_trips_through_its_g_address() {
        let original = key(7);
        let text = original.to_string();
        assert!(text.starts_with("G"));
        assert_eq!(text.len(), 56);
        assert_eq!(SignerKey::from_strkey(&text), Ok(original));
        assert_eq!(text.parse::<SignerKey>(), Ok(original));
    }

    #[test]
    fn key_refuses_other_address_kinds_and_garbage() {
        let contract = stellar_strkey::Contract([3u8; 32]).to_string();
        assert!(matches!(
            SignerKey::from_strkey(&contract),
            Err(DomainError::InvalidAccountKey(_))
        ));
        let muxed = stellar_strkey::ed25519::MuxedAccount {
            ed25519: [3u8; 32],
            id: 1,
        }
        .to_string();
        assert!(matches!(
            SignerKey::from_strkey(&muxed),
            Err(DomainError::InvalidAccountKey(_))
        ));
        assert!(SignerKey::from_strkey("").is_err());
        assert!(SignerKey::from_strkey("not a key").is_err());
    }

    #[test]
    fn key_with_a_changed_character_fails_its_checksum() {
        let good = key(7).to_string();
        let first = good.chars().nth(10).expect("character");
        let swapped = if first == 'A' { "B" } else { "A" };
        let mut bad = good.clone();
        bad.replace_range(10..11, swapped);
        assert_ne!(good, bad);
        assert!(SignerKey::from_strkey(&bad).is_err());
    }

    #[test]
    fn keys_order_by_bytes() {
        assert!(key(1) < key(2));
    }

    #[test]
    fn state_accepts_a_valid_account() {
        let state = AccountState::new(
            key(1),
            w(1),
            thresholds(1, 2, 2),
            [(key(2), w(1)), (key(3), w(1))],
        )
        .expect("valid state");
        assert_eq!(state.account(), key(1));
        assert_eq!(state.master_weight(), w(1));
        assert_eq!(state.additional_signer_count(), 2);
        assert_eq!(state.threshold(ThresholdClass::High), t(2));
    }

    #[test]
    fn state_refuses_the_account_key_as_a_signer() {
        let err = AccountState::new(key(1), w(1), thresholds(0, 0, 0), [(key(1), w(1))])
            .expect_err("account key as signer");
        assert_eq!(err, DomainError::SignerIsAccount(key(1)));
    }

    #[test]
    fn state_refuses_a_zero_weight_signer() {
        let err = AccountState::new(key(1), w(1), thresholds(0, 0, 0), [(key(2), w(0))])
            .expect_err("zero weight signer");
        assert_eq!(err, DomainError::ZeroWeightSigner(key(2)));
    }

    #[test]
    fn state_refuses_a_duplicate_signer() {
        let err = AccountState::new(
            key(1),
            w(1),
            thresholds(0, 0, 0),
            [(key(2), w(1)), (key(2), w(5))],
        )
        .expect_err("duplicate signer");
        assert_eq!(err, DomainError::DuplicateSigner(key(2)));
    }

    #[test]
    fn state_allows_20_additional_signers_and_refuses_21() {
        let make = |count: u8| (2..2 + count).map(|n| (key(n), w(1))).collect::<Vec<_>>();
        assert!(AccountState::new(key(1), w(1), thresholds(0, 0, 0), make(20)).is_ok());
        let err =
            AccountState::new(key(1), w(1), thresholds(0, 0, 0), make(21)).expect_err("21 signers");
        assert_eq!(err, DomainError::TooManySigners { count: 21, max: 20 });
    }

    #[test]
    fn first_offending_signer_in_input_order_is_reported() {
        let err = AccountState::new(
            key(1),
            w(1),
            thresholds(0, 0, 0),
            [(key(2), w(0)), (key(1), w(1))],
        )
        .expect_err("two problems");
        assert_eq!(err, DomainError::ZeroWeightSigner(key(2)));
    }

    #[test]
    fn weight_of_covers_master_signer_and_stranger() {
        let state = AccountState::new(key(1), w(3), thresholds(0, 0, 0), [(key(2), w(4))])
            .expect("valid state");
        assert_eq!(state.weight_of(key(1)), w(3));
        assert_eq!(state.weight_of(key(2)), w(4));
        assert_eq!(state.weight_of(key(9)), Weight::ZERO);
    }

    #[test]
    fn total_weight_counts_master_and_all_signers() {
        let state = AccountState::new(
            key(1),
            w(2),
            thresholds(0, 0, 0),
            [(key(2), w(255)), (key(3), w(255))],
        )
        .expect("valid state");
        assert_eq!(state.total_weight(), 512);
    }

    #[test]
    fn a_master_weight_of_zero_adds_nothing() {
        let state = AccountState::new(key(1), w(0), thresholds(0, 0, 0), [(key(2), w(1))])
            .expect("valid state");
        assert_eq!(state.total_weight(), 1);
    }

    #[test]
    fn signers_iterate_in_ascending_key_order() {
        let state = AccountState::new(
            key(1),
            w(1),
            thresholds(0, 0, 0),
            [(key(9), w(1)), (key(3), w(2)), (key(5), w(3))],
        )
        .expect("valid state");
        let keys: Vec<SignerKey> = state.signers().map(|(k, _)| k).collect();
        assert_eq!(keys, [key(3), key(5), key(9)]);
    }
}
