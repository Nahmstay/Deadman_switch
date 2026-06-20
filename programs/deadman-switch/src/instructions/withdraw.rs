use anchor_lang::prelude::*;

use crate::{constants::SWITCH_SEED, error::ErrorCode, state::DeadmanSwitch};

#[derive(Accounts)]
pub struct Withdraw<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(
        mut,
        seeds = [SWITCH_SEED, owner.key().as_ref()],
        bump = switch.bump,
        constraint = switch.owner == owner.key() @ ErrorCode::NotOwner,
    )]
    pub switch: Account<'info, DeadmanSwitch>,
}

pub fn handler(ctx: Context<Withdraw>, amount: u64) -> Result<()> {
    let switch = &ctx.accounts.switch;

    require!(!switch.triggered, ErrorCode::AlreadyTriggered);

    let clock = Clock::get()?;

    // Lock at deadline, not grace end. Owner can't drain funds beneficiaries are owed.
    require!(
        !switch.is_past_deadline(clock.unix_timestamp),
        ErrorCode::SwitchExpired
    );

    let switch_info = ctx.accounts.switch.to_account_info();
    let rent = Rent::get()?;
    let minimum_balance = rent.minimum_balance(switch_info.data_len());
    let available = switch_info
        .lamports()
        .checked_sub(minimum_balance)
        .unwrap_or(0);

    require!(amount > 0 && amount <= available, ErrorCode::InsufficientFunds);

    **ctx
        .accounts
        .switch
        .to_account_info()
        .try_borrow_mut_lamports()? -= amount;
    **ctx
        .accounts
        .owner
        .to_account_info()
        .try_borrow_mut_lamports()? += amount;

    Ok(())
}
