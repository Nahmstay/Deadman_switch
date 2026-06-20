mod constants;
mod error;
mod instructions;
pub mod state;

use anchor_lang::prelude::*;
use instructions::*;
use state::Beneficiary;

declare_id!("5BR3iY7HKjy2qdL7i99bQZPP9fxAzpGAVcihR7MHZWj6");

#[program]
pub mod deadman_switch {
    use super::*;

    /// Create a new deadman switch PDA for the signing owner.
    pub fn initialize(
        ctx: Context<Initialize>,
        beneficiaries: Vec<Beneficiary>,
        check_in_interval: i64,
        grace_period: i64,
        deposit_amount: u64,
        message: Option<String>,
    ) -> Result<()> {
        initialize::handler(ctx, beneficiaries, check_in_interval, grace_period, deposit_amount, message)
    }

    /// Owner resets the timer. Blocked once the grace period fully elapses.
    pub fn check_in(ctx: Context<CheckIn>) -> Result<()> {
        check_in::handler(ctx)
    }

    /// Add lamports to the switch. Anyone can call this.
    pub fn deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
        deposit::handler(ctx, amount)
    }

    /// Owner withdraws. Active period only; rent minimum always stays.
    pub fn withdraw(ctx: Context<Withdraw>, amount: u64) -> Result<()> {
        withdraw::handler(ctx, amount)
    }

    /// Permissionless: distributes lamports to beneficiaries once fully expired.
    /// Pass beneficiary accounts as remaining_accounts in order, all writable.
    pub fn trigger(ctx: Context<Trigger>) -> Result<()> {
        trigger::handler(ctx)
    }

    /// Owner sets or updates the message stored in the switch. Pass None to clear.
    pub fn add_message(ctx: Context<AddMessage>, message: Option<String>) -> Result<()> {
        add_message::handler(ctx, message)
    }

    /// Owner replaces the beneficiary list. All shares must still sum to 10,000 bps.
    /// Use this to add, remove, or rebalance; pass the full new list.
    pub fn update_beneficiaries(
        ctx: Context<UpdateBeneficiaries>,
        beneficiaries: Vec<Beneficiary>,
    ) -> Result<()> {
        update_beneficiaries::handler(ctx, beneficiaries)
    }

    /// Owner updates the check-in interval and/or grace period. Minimum 1 day.
    /// Blocked if deadline already passed; check in first, then update.
    pub fn update_interval(
        ctx: Context<UpdateInterval>,
        check_in_interval: i64,
        grace_period: i64,
    ) -> Result<()> {
        update_interval::handler(ctx, check_in_interval, grace_period)
    }

    /// Owner recovers all funds and closes the PDA. Blocked once fully expired.
    pub fn cancel(ctx: Context<Cancel>) -> Result<()> {
        cancel::handler(ctx)
    }
}
