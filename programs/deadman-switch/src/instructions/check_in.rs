use anchor_lang::prelude::*;

use crate::{constants::SWITCH_SEED, error::ErrorCode, state::DeadmanSwitch};

#[derive(Accounts)]
pub struct CheckIn<'info> {
    pub owner: Signer<'info>,

    #[account(
        mut,
        seeds = [SWITCH_SEED, owner.key().as_ref()],
        bump = switch.bump,
        constraint = switch.owner == owner.key() @ ErrorCode::NotOwner,
    )]
    pub switch: Account<'info, DeadmanSwitch>,
}

pub fn handler(ctx: Context<CheckIn>) -> Result<()> {
    require!(!ctx.accounts.switch.triggered, ErrorCode::AlreadyTriggered);

    let clock = Clock::get()?;

    // Still allowed during grace; checking in resets the full 180-day timer.
    require!(
        !ctx.accounts.switch.is_fully_expired(clock.unix_timestamp),
        ErrorCode::SwitchExpired
    );

    let was_in_grace = ctx.accounts.switch.is_in_grace(clock.unix_timestamp);

    ctx.accounts.switch.last_check_in = clock.unix_timestamp;

    let next_deadline = ctx.accounts.switch.last_check_in + ctx.accounts.switch.check_in_interval;
    let days_until = ctx.accounts.switch.check_in_interval / 86_400;

    if was_in_grace {
        msg!(
            "Grace check-in; timer fully reset. Next deadline: {} days (Unix {}).",
            days_until,
            next_deadline
        );
    } else {
        msg!(
            "Check-in recorded. Next deadline in {} days (Unix {}).",
            days_until,
            next_deadline
        );
    }

    let switch_info = ctx.accounts.switch.to_account_info();
    let rent = Rent::get()?;
    let minimum_balance = rent.minimum_balance(switch_info.data_len());
    let current_balance = switch_info.lamports();
    let distributable = current_balance.saturating_sub(minimum_balance);

    msg!(
        "Balance: {} lamports | Rent minimum: {} lamports | Available for beneficiaries: {} lamports",
        current_balance,
        minimum_balance,
        distributable
    );

    if distributable == 0 {
        msg!("No funds to distribute. Deposit SOL before this fires.");
    }

    Ok(())
}
