use anchor_lang::prelude::*;

use crate::{constants::{MAX_MESSAGE_LEN, SWITCH_SEED}, error::ErrorCode, state::DeadmanSwitch};

#[derive(Accounts)]
pub struct AddMessage<'info> {
    pub owner: Signer<'info>,

    #[account(
        mut,
        seeds = [SWITCH_SEED, owner.key().as_ref()],
        bump = switch.bump,
        constraint = switch.owner == owner.key() @ ErrorCode::NotOwner,
    )]
    pub switch: Account<'info, DeadmanSwitch>,
}

/// Set, update, or clear the message stored in the switch.
/// Pass `None` to remove an existing message.
pub fn handler(ctx: Context<AddMessage>, message: Option<String>) -> Result<()> {
    require!(!ctx.accounts.switch.triggered, ErrorCode::AlreadyTriggered);

    if let Some(ref msg) = message {
        require!(msg.len() <= MAX_MESSAGE_LEN, ErrorCode::MessageTooLong);
    }

    ctx.accounts.switch.message = message;
    Ok(())
}
