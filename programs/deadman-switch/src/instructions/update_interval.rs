use anchor_lang::prelude::*;

use crate::{
    constants::{MIN_CHECK_IN_INTERVAL, SWITCH_SEED},
    error::ErrorCode,
    state::DeadmanSwitch,
};

#[derive(Accounts)]
pub struct UpdateInterval<'info> {
    pub owner: Signer<'info>,

    #[account(
        mut,
        seeds = [SWITCH_SEED, owner.key().as_ref()],
        bump = switch.bump,
        constraint = switch.owner == owner.key() @ ErrorCode::NotOwner,
    )]
    pub switch: Account<'info, DeadmanSwitch>,
}

pub fn handler(
    ctx: Context<UpdateInterval>,
    check_in_interval: i64,
    grace_period: i64,
) -> Result<()> {
    require!(!ctx.accounts.switch.triggered, ErrorCode::AlreadyTriggered);
    require!(check_in_interval >= MIN_CHECK_IN_INTERVAL, ErrorCode::InvalidInterval);
    require!(grace_period >= 0, ErrorCode::InvalidInterval);

    let clock = Clock::get()?;

    // Blocked after deadline; extending interval during grace would reset the clock for free.
    require!(
        !ctx.accounts.switch.is_past_deadline(clock.unix_timestamp),
        ErrorCode::SwitchExpired
    );

    let old_interval = ctx.accounts.switch.check_in_interval;
    let old_grace    = ctx.accounts.switch.grace_period;

    ctx.accounts.switch.check_in_interval = check_in_interval;
    ctx.accounts.switch.grace_period      = grace_period;

    let new_deadline  = ctx.accounts.switch.last_check_in.saturating_add(check_in_interval);
    let new_grace_end = new_deadline.saturating_add(grace_period);

    // Reject if shortening would make the switch immediately triggerable.
    require!(
        clock.unix_timestamp <= new_grace_end,
        ErrorCode::InvalidInterval
    );

    if clock.unix_timestamp > new_deadline {
        let secs_left = new_grace_end - clock.unix_timestamp;
        msg!(
            "Shorter interval: switch is now in grace period. {} day(s) to check in or cancel.",
            secs_left / 86_400
        );
    } else {
        let days_until = (new_deadline - clock.unix_timestamp) / 86_400;
        msg!(
            "Schedule updated. Interval: {} → {} days. Grace: {} → {} days. Next deadline in {} days.",
            old_interval / 86_400,
            check_in_interval / 86_400,
            old_grace / 86_400,
            grace_period / 86_400,
            days_until,
        );
    }

    Ok(())
}
