use anchor_lang::prelude::*;

use crate::constants::{MAX_BENEFICIARIES, MAX_MESSAGE_LEN};

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct Beneficiary {
    pub address: Pubkey,
    /// Share in basis points (1 = 0.01%). All shares must sum to 10,000.
    pub share_bps: u16,
}

impl Beneficiary {
    pub const SIZE: usize = 32 + 2; // Pubkey + u16
}

#[account]
#[derive(Debug)]
pub struct DeadmanSwitch {
    pub owner: Pubkey,
    pub beneficiaries: Vec<Beneficiary>,
    /// How often the owner must check in (seconds).
    pub check_in_interval: i64,
    /// Grace window after deadline; owner can still check in or cancel. Default: 7 days.
    pub grace_period: i64,
    /// Timestamp of the last check-in (or init).
    pub last_check_in: i64,
    /// Set once trigger() fires.
    pub triggered: bool,
    /// Optional note for beneficiaries.
    pub message: Option<String>,
    pub bump: u8,
}

impl DeadmanSwitch {
    pub const SPACE: usize = 8                              // discriminator
        + 32                                                // owner
        + 4 + MAX_BENEFICIARIES * Beneficiary::SIZE        // Vec<Beneficiary>
        + 8                                                 // check_in_interval
        + 8                                                 // grace_period
        + 8                                                 // last_check_in
        + 1                                                 // triggered
        + 1 + 4 + MAX_MESSAGE_LEN                          // Option<String>
        + 1;                                               // bump

    /// True once the initial check-in interval has elapsed.
    /// Grace window is now open; owner can still check in or cancel.
    pub fn is_past_deadline(&self, now: i64) -> bool {
        now > self.last_check_in.saturating_add(self.check_in_interval)
    }

    /// True once both deadline and grace period have elapsed. trigger() is allowed now.
    pub fn is_fully_expired(&self, now: i64) -> bool {
        now > self.last_check_in
            .saturating_add(self.check_in_interval)
            .saturating_add(self.grace_period)
    }

    /// True while within the grace window (past deadline but grace not yet over).
    pub fn is_in_grace(&self, now: i64) -> bool {
        self.is_past_deadline(now) && !self.is_fully_expired(now)
    }

    /// Seconds remaining in the grace window, or 0 if expired.
    pub fn grace_seconds_remaining(&self, now: i64) -> i64 {
        let grace_end = self.last_check_in
            .saturating_add(self.check_in_interval)
            .saturating_add(self.grace_period);
        grace_end.saturating_sub(now).max(0)
    }
}
