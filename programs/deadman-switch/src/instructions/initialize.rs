use anchor_lang::prelude::*;
use anchor_lang::system_program;

use crate::{
    constants::{MAX_BENEFICIARIES, MAX_MESSAGE_LEN, MIN_CHECK_IN_INTERVAL, SWITCH_SEED, TOTAL_BPS},
    error::ErrorCode,
    state::{Beneficiary, DeadmanSwitch},
};

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(
        init,
        payer = owner,
        space = DeadmanSwitch::SPACE,
        seeds = [SWITCH_SEED, owner.key().as_ref()],
        bump,
    )]
    pub switch: Account<'info, DeadmanSwitch>,

    pub system_program: Program<'info, System>,
}

pub fn handler(
    ctx: Context<Initialize>,
    beneficiaries: Vec<Beneficiary>,
    check_in_interval: i64,
    grace_period: i64,
    deposit_amount: u64,
    message: Option<String>,
) -> Result<()> {
    require!(check_in_interval >= MIN_CHECK_IN_INTERVAL, ErrorCode::InvalidInterval);
    require!(grace_period >= 0, ErrorCode::InvalidInterval);
    require!(!beneficiaries.is_empty(), ErrorCode::EmptyBeneficiaries);
    require!(
        beneficiaries.len() <= MAX_BENEFICIARIES,
        ErrorCode::TooManyBeneficiaries
    );

    let total_shares: u32 = beneficiaries.iter().map(|b| b.share_bps as u32).sum();
    require!(total_shares == TOTAL_BPS as u32, ErrorCode::InvalidShares);

    // PDA as beneficiary = lamports sent to itself (permanent lock). Also block duplicates.
    let switch_key = ctx.accounts.switch.key();
    for i in 0..beneficiaries.len() {
        require!(
            beneficiaries[i].address != switch_key,
            ErrorCode::InvalidBeneficiaries
        );
        for j in (i + 1)..beneficiaries.len() {
            require!(
                beneficiaries[i].address != beneficiaries[j].address,
                ErrorCode::DuplicateBeneficiary
            );
        }
    }

    if let Some(ref msg) = message {
        require!(msg.len() <= MAX_MESSAGE_LEN, ErrorCode::MessageTooLong);
    }

    let clock = Clock::get()?;
    let switch = &mut ctx.accounts.switch;
    switch.owner = ctx.accounts.owner.key();
    switch.beneficiaries = beneficiaries;
    switch.check_in_interval = check_in_interval;
    switch.grace_period = grace_period;
    switch.last_check_in = clock.unix_timestamp;
    switch.triggered = false;
    switch.message = message;
    switch.bump = ctx.bumps.switch;

    if deposit_amount > 0 {
        let cpi_accounts = system_program::Transfer {
            from: ctx.accounts.owner.to_account_info(),
            to: ctx.accounts.switch.to_account_info(),
        };
        let cpi_ctx =
            CpiContext::new(ctx.accounts.system_program.key(), cpi_accounts);
        system_program::transfer(cpi_ctx, deposit_amount)?;
    }

    Ok(())
}
