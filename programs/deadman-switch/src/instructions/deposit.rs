use anchor_lang::prelude::*;
use anchor_lang::system_program;

use crate::{constants::SWITCH_SEED, error::ErrorCode, state::DeadmanSwitch};

#[derive(Accounts)]
pub struct Deposit<'info> {
    #[account(mut)]
    pub depositor: Signer<'info>,

    /// Derived from switch.owner, not the signer — anyone can top up.
    #[account(
        mut,
        seeds = [SWITCH_SEED, switch.owner.as_ref()],
        bump = switch.bump,
    )]
    pub switch: Account<'info, DeadmanSwitch>,

    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<Deposit>, amount: u64) -> Result<()> {
    require!(!ctx.accounts.switch.triggered, ErrorCode::AlreadyTriggered);
    require!(amount > 0, ErrorCode::InsufficientFunds);

    let cpi_accounts = system_program::Transfer {
        from: ctx.accounts.depositor.to_account_info(),
        to: ctx.accounts.switch.to_account_info(),
    };
    let cpi_ctx = CpiContext::new(ctx.accounts.system_program.key(), cpi_accounts);
    system_program::transfer(cpi_ctx, amount)?;

    Ok(())
}
