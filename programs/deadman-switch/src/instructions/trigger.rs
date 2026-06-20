use anchor_lang::prelude::*;

use crate::{constants::SWITCH_SEED, error::ErrorCode, state::DeadmanSwitch};

#[derive(Accounts)]
pub struct Trigger<'info> {
    /// No owner constraint. Anyone can call trigger; signer just pays the tx fee.
    #[account(mut)]
    pub caller: Signer<'info>,

    #[account(
        mut,
        seeds = [SWITCH_SEED, switch.owner.as_ref()],
        bump = switch.bump,
    )]
    pub switch: Account<'info, DeadmanSwitch>,
    // Pass beneficiary wallets as remaining_accounts in order, all writable.
}

pub fn handler(ctx: Context<Trigger>) -> Result<()> {
    let switch = &ctx.accounts.switch;

    require!(!switch.triggered, ErrorCode::AlreadyTriggered);

    let clock = Clock::get()?;

    // Both the deadline and grace period must have passed before trigger is allowed.
    require!(
        switch.is_fully_expired(clock.unix_timestamp),
        ErrorCode::DeadlineNotPassed
    );

    let remaining = ctx.remaining_accounts;
    require!(
        remaining.len() == switch.beneficiaries.len(),
        ErrorCode::InvalidBeneficiaries
    );
    for (i, account_info) in remaining.iter().enumerate() {
        require!(
            account_info.key() == switch.beneficiaries[i].address,
            ErrorCode::InvalidBeneficiaries
        );
        // Explicit check gives a clean error vs an opaque runtime abort.
        require!(account_info.is_writable, ErrorCode::InvalidBeneficiaries);
    }

    // Keep rent minimum in the PDA so the account stays alive as a permanent record.
    let switch_info = ctx.accounts.switch.to_account_info();
    let minimum_balance = Rent::get()?.minimum_balance(switch_info.data_len());
    let distributable = switch_info.lamports().saturating_sub(minimum_balance);

    let num = switch.beneficiaries.len();
    let mut amounts: Vec<u64> = Vec::with_capacity(num);
    let mut distributed: u64 = 0;

    for (i, b) in switch.beneficiaries.iter().enumerate() {
        let amount: u64 = if i == num - 1 {
            distributable.saturating_sub(distributed)
        } else {
            ((distributable as u128)
                .checked_mul(b.share_bps as u128)
                .unwrap_or(0)
                / 10_000u128) as u64
        };
        amounts.push(amount);
        distributed = distributed.saturating_add(amount);
    }

    // Set triggered before transfers to prevent re-entry.
    ctx.accounts.switch.triggered = true;

    for (i, amount) in amounts.iter().enumerate() {
        if *amount > 0 {
            **switch_info.try_borrow_mut_lamports()? -= amount;
            **remaining[i].try_borrow_mut_lamports()? += amount;
        }
    }

    msg!(
        "Deadman switch triggered. {} lamports distributed to {} beneficiaries.",
        distributable,
        num
    );

    Ok(())
}
