//! The balance side of an account: what it holds, what it must keep, and what
//! is left to spend.
//!
//! Rules R11 to R13 in `docs/SPEC.md`, section 4. Every amount is a whole
//! number of stroops (1 XLM is 10,000,000 stroops), so no decimal is ever read
//! or rounded. Sums are done in `i128`, which cannot overflow on values that
//! each fit in `i64`.

use crate::error::DomainError;

/// The most sub-entries (signers, trust lines, offers, data entries) an account
/// can hold (R11).
pub const SUB_ENTRY_LIMIT: u32 = 1000;

/// The fee assumed for each operation when a transaction gives none, in
/// stroops. It is the base fee both networks reported on 2026-11-23, and it is
/// also the lowest fee the checker accepts (`docs/SPEC.md`, section 5.2).
pub const ASSUMED_FEE_PER_OPERATION: u32 = 100;

/// Why a new signer cannot be afforded (R11, checked in this order).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    /// The account already holds [`SUB_ENTRY_LIMIT`] sub-entries
    /// (`opTOO_MANY_SUBENTRIES`).
    TooManySubEntries,
    /// Less than one base reserve is available (`op_low_reserve`).
    LowReserve {
        /// How much is available, in stroops. May be negative.
        available: i128,
        /// The base reserve that was needed.
        needed: i128,
    },
}

/// The balance figures of one account at one moment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Reserve {
    balance: i64,
    selling_liabilities: i64,
    subentry_count: u32,
    num_sponsoring: u32,
    num_sponsored: u32,
    base_reserve: i64,
}

impl Reserve {
    /// Builds the figures, refusing anything Stellar could not hold.
    pub fn new(
        balance: i64,
        selling_liabilities: i64,
        subentry_count: u32,
        num_sponsoring: u32,
        num_sponsored: u32,
        base_reserve: i64,
    ) -> Result<Self, DomainError> {
        if balance < 0 || selling_liabilities < 0 {
            return Err(DomainError::InvalidReserve("an amount is negative"));
        }
        if selling_liabilities > balance {
            return Err(DomainError::InvalidReserve(
                "the selling liabilities are more than the balance",
            ));
        }
        if base_reserve < 1 {
            return Err(DomainError::InvalidReserve(
                "the base reserve is not above 0",
            ));
        }
        if subentry_count > SUB_ENTRY_LIMIT {
            return Err(DomainError::InvalidReserve(
                "the sub-entry count is above 1000",
            ));
        }
        let effective =
            2 + i128::from(subentry_count) + i128::from(num_sponsoring) - i128::from(num_sponsored);
        if effective < 0 {
            return Err(DomainError::InvalidReserve(
                "the sponsorship counts leave a negative number of reserved entries",
            ));
        }
        // The network computes the minimum balance in 64 bits, so a larger one
        // cannot belong to an account.
        if effective * i128::from(base_reserve) > i128::from(i64::MAX) {
            return Err(DomainError::InvalidReserve(
                "the minimum balance is more than an account can hold",
            ));
        }
        Ok(Self {
            balance,
            selling_liabilities,
            subentry_count,
            num_sponsoring,
            num_sponsored,
            base_reserve,
        })
    }

    /// Refuses figures that cannot belong to an account with this many signers,
    /// because every additional signer is a sub-entry.
    pub fn check_signer_count(&self, signers: usize) -> Result<(), DomainError> {
        if usize::try_from(self.subentry_count).is_ok_and(|count| count >= signers) {
            Ok(())
        } else {
            Err(DomainError::InvalidReserve(
                "the sub-entry count is below the number of signers",
            ))
        }
    }

    /// The native balance, in stroops.
    pub const fn balance(&self) -> i64 {
        self.balance
    }

    /// The native selling liabilities, in stroops.
    pub const fn selling_liabilities(&self) -> i64 {
        self.selling_liabilities
    }

    /// The number of sub-entries.
    pub const fn subentry_count(&self) -> u32 {
        self.subentry_count
    }

    /// The number of entries this account sponsors.
    pub const fn num_sponsoring(&self) -> u32 {
        self.num_sponsoring
    }

    /// The number of this account's entries that others sponsor.
    pub const fn num_sponsored(&self) -> u32 {
        self.num_sponsored
    }

    /// The network's base reserve, in stroops.
    pub const fn base_reserve(&self) -> i64 {
        self.base_reserve
    }

    /// What the account must keep: `(2 + sub-entries + sponsoring - sponsored)`
    /// base reserves.
    pub fn min_balance(&self) -> i128 {
        let effective = 2 + i128::from(self.subentry_count) + i128::from(self.num_sponsoring)
            - i128::from(self.num_sponsored);
        effective * i128::from(self.base_reserve)
    }

    /// What can be spent: the balance, less the minimum balance, less selling
    /// liabilities. Negative when the account holds less than it must keep.
    pub fn available(&self) -> i128 {
        i128::from(self.balance) - self.min_balance() - i128::from(self.selling_liabilities)
    }

    /// The figures after a fee of `fee` stroops is charged (R13).
    #[must_use]
    pub fn after_fee(&self, fee: u32) -> Self {
        Self {
            balance: self.balance.saturating_sub(i64::from(fee)),
            ..*self
        }
    }

    /// Whether a new signer can be added now (R11, steps b and c).
    pub fn check_new_signer(&self) -> Result<(), Refusal> {
        if self.subentry_count >= SUB_ENTRY_LIMIT {
            return Err(Refusal::TooManySubEntries);
        }
        let needed = i128::from(self.base_reserve);
        let available = self.available();
        if available < needed {
            return Err(Refusal::LowReserve { available, needed });
        }
        Ok(())
    }

    /// The figures after a new signer is added.
    #[must_use]
    pub fn with_signer_added(&self) -> Self {
        Self {
            subentry_count: self.subentry_count.saturating_add(1),
            ..*self
        }
    }

    /// The figures after a signer is removed.
    #[must_use]
    pub fn with_signer_removed(&self) -> Self {
        Self {
            subentry_count: self.subentry_count.saturating_sub(1),
            ..*self
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BASE: i64 = 5_000_000;

    fn reserve(balance: i64, subentries: u32) -> Reserve {
        Reserve::new(balance, 0, subentries, 0, 0, BASE).expect("valid")
    }

    #[test]
    fn the_minimum_balance_counts_two_entries_plus_sub_entries() {
        assert_eq!(reserve(0, 0).min_balance(), 10_000_000);
        assert_eq!(reserve(0, 1).min_balance(), 15_000_000);
        assert_eq!(reserve(0, 1000).min_balance(), 1002 * 5_000_000);
    }

    #[test]
    fn sponsorship_moves_the_minimum_balance() {
        let sponsoring = Reserve::new(0, 0, 1, 3, 0, BASE).unwrap();
        assert_eq!(sponsoring.min_balance(), 6 * 5_000_000);
        let sponsored = Reserve::new(0, 0, 3, 0, 2, BASE).unwrap();
        assert_eq!(sponsored.min_balance(), 3 * 5_000_000);
    }

    #[test]
    fn available_is_balance_less_minimum_less_selling_liabilities() {
        let r = Reserve::new(20_000_100, 40, 1, 0, 0, BASE).unwrap();
        assert_eq!(r.available(), 20_000_100 - 15_000_000 - 40);
        // Below the minimum balance, available is negative, not clamped.
        assert_eq!(reserve(9_000_000, 0).available(), -1_000_000);
    }

    // R11: equal is allowed, one stroop less is not.
    #[test]
    fn a_signer_needs_exactly_one_base_reserve_available() {
        assert_eq!(reserve(20_000_000, 1).check_new_signer(), Ok(()));
        assert_eq!(
            reserve(19_999_999, 1).check_new_signer(),
            Err(Refusal::LowReserve {
                available: 4_999_999,
                needed: 5_000_000
            })
        );
    }

    #[test]
    fn selling_liabilities_reduce_what_is_available() {
        let r = Reserve::new(20_000_000, 1, 1, 0, 0, BASE).unwrap();
        assert!(matches!(
            r.check_new_signer(),
            Err(Refusal::LowReserve { .. })
        ));
    }

    #[test]
    fn the_sub_entry_limit_is_checked_before_the_reserve() {
        // Plenty of money, and also too many entries: the limit is reported.
        assert_eq!(
            reserve(i64::MAX, 1000).check_new_signer(),
            Err(Refusal::TooManySubEntries)
        );
        // Too many entries and too little money: still the limit.
        assert_eq!(
            reserve(0, 1000).check_new_signer(),
            Err(Refusal::TooManySubEntries)
        );
        assert_eq!(reserve(i64::MAX, 999).check_new_signer(), Ok(()));
    }

    #[test]
    fn adding_and_removing_a_signer_moves_the_sub_entry_count() {
        let r = reserve(25_000_000, 1);
        assert_eq!(r.with_signer_added().subentry_count(), 2);
        assert_eq!(r.with_signer_removed().subentry_count(), 0);
        // Removing the only entry again does not wrap.
        assert_eq!(reserve(0, 0).with_signer_removed().subentry_count(), 0);
    }

    #[test]
    fn a_fee_lowers_the_balance_and_nothing_else() {
        let r = reserve(20_000_100, 1);
        let after = r.after_fee(100);
        assert_eq!(after.balance(), 20_000_000);
        assert_eq!(after.subentry_count(), 1);
        assert_eq!(after.available(), r.available() - 100);
    }

    #[test]
    fn impossible_figures_are_refused() {
        assert!(Reserve::new(-1, 0, 0, 0, 0, BASE).is_err());
        assert!(Reserve::new(0, -1, 0, 0, 0, BASE).is_err());
        assert!(Reserve::new(0, 0, 0, 0, 0, 0).is_err());
        assert!(Reserve::new(0, 0, 0, 0, 0, -5).is_err());
        assert!(Reserve::new(0, 0, 1001, 0, 0, BASE).is_err());
        assert!(Reserve::new(0, 0, 1000, 0, 0, BASE).is_ok());
        // 2 + 0 + 0 - 3 is negative.
        assert!(Reserve::new(0, 0, 0, 0, 3, BASE).is_err());
        assert!(Reserve::new(0, 0, 0, 0, 2, BASE).is_ok());
    }

    #[test]
    fn the_sub_entry_count_cannot_be_below_the_number_of_signers() {
        let r = reserve(0, 2);
        assert!(r.check_signer_count(2).is_ok());
        assert!(r.check_signer_count(0).is_ok());
        assert!(r.check_signer_count(3).is_err());
    }

    #[test]
    fn the_largest_values_do_not_overflow() {
        let r = Reserve::new(i64::MAX, i64::MAX, 1000, 0, 0, i64::MAX / 1002).unwrap();
        let _ = (r.min_balance(), r.available(), r.check_new_signer());
        let _ = r.after_fee(u32::MAX).with_signer_added();
        let r = Reserve::new(i64::MAX, 0, 1000, u32::MAX, 0, 1).unwrap();
        let _ = (r.min_balance(), r.available(), r.check_new_signer());
    }

    #[test]
    fn figures_that_no_account_could_have_are_refused() {
        // More selling liabilities than balance.
        assert!(Reserve::new(10, 11, 0, 0, 0, BASE).is_err());
        assert!(Reserve::new(10, 10, 0, 0, 0, BASE).is_ok());
        // A minimum balance above what 64 bits can hold.
        assert!(Reserve::new(i64::MAX, 0, 1000, 0, 0, i64::MAX).is_err());
        assert!(Reserve::new(i64::MAX, 0, 0, 0, 0, i64::MAX / 2).is_ok());
        assert!(Reserve::new(i64::MAX, 0, 0, 0, 0, i64::MAX / 2 + 1).is_err());
    }
}
