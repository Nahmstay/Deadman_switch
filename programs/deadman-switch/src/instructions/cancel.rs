use anchor_lang::prelude::*;

use crate::{constants::SWITCH_SEED, error::ErrorCode, state::DeadmanSwitch};

#[derive(Accounts)]
pub struct Cancel<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    /// `close = owner` sends all lamports (including rent) back and zeroes the account.
    #[account(
        mut,
        close = owner,
        seeds = [SWITCH_SEED, owner.key().as_ref()],
        bump = switch.bump,
        constraint = switch.owner == owner.key() @ ErrorCode::NotOwner,
    )]
    pub switch: Account<'info, DeadmanSwitch>,

    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<Cancel>) -> Result<()> {
    require!(!ctx.accounts.switch.triggered, ErrorCode::AlreadyTriggered);

    let clock = Clock::get()?;
    // Once fully expired, only trigger() can run; cancel is blocked to prevent front-running.
    require!(
        !ctx.accounts.switch.is_fully_expired(clock.unix_timestamp),
        ErrorCode::SwitchExpired
    );

    msg!(
        "Deadman switch cancelled by owner. All funds returned to {}.",
        ctx.accounts.owner.key()
    );

    Ok(())
}
