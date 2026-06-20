use anchor_lang::prelude::*;

#[error_code]
pub enum ErrorCode {
    #[msg("Only the owner can call this instruction")]
    NotOwner,

    #[msg("The deadman switch has already been triggered")]
    AlreadyTriggered,

    #[msg("The check-in deadline has not passed yet")]
    DeadlineNotPassed,

    #[msg("Action blocked: check-in deadline has passed")]
    SwitchExpired,

    #[msg("Beneficiary shares must sum to exactly 10,000 basis points (100%)")]
    InvalidShares,

    #[msg("Too many beneficiaries; maximum is 10")]
    TooManyBeneficiaries,

    #[msg("At least one beneficiary is required")]
    EmptyBeneficiaries,

    #[msg("Insufficient withdrawable balance")]
    InsufficientFunds,

    #[msg("Message exceeds the 500-character limit")]
    MessageTooLong,

    #[msg("Check-in interval must be at least 1 day; grace period must be >= 0")]
    InvalidInterval,

    #[msg("Remaining accounts must match stored beneficiaries in order")]
    InvalidBeneficiaries,

    #[msg("Duplicate beneficiary; each address can only appear once")]
    DuplicateBeneficiary,
}
