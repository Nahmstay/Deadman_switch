use anchor_lang::prelude::*;

use crate::{
    constants::{MAX_BENEFICIARIES, SWITCH_SEED, TOTAL_BPS},
    error::ErrorCode,
    state::{Beneficiary, DeadmanSwitch},
};

#[derive(Accounts)]
pub struct UpdateBeneficiaries<'info> {
    pub owner: Signer<'info>,

    #[account(
        mut,
        seeds = [SWITCH_SEED, owner.key().as_ref()],
        bump = switch.bump,
        constraint = switch.owner == owner.key() @ ErrorCode::NotOwner,
    )]
    pub switch: Account<'info, DeadmanSwitch>,
}

/// Replace the entire beneficiary list. Blocked after deadline; check in first.
pub fn handler(
    ctx: Context<UpdateBeneficiaries>,
    beneficiaries: Vec<Beneficiary>,
) -> Result<()> {
    require!(!ctx.accounts.switch.triggered, ErrorCode::AlreadyTriggered);

    let clock = Clock::get()?;
    // Locked at deadline; a compromised key shouldn't be able to redirect funds during grace.
    require!(
        !ctx.accounts.switch.is_past_deadline(clock.unix_timestamp),
        ErrorCode::SwitchExpired
    );

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

    ctx.accounts.switch.beneficiaries = beneficiaries;
    Ok(())
}
