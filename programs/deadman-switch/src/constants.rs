pub const SWITCH_SEED: &[u8] = b"deadman-switch";
pub const MAX_BENEFICIARIES: usize = 10;
pub const MAX_MESSAGE_LEN: usize = 500;
pub const TOTAL_BPS: u16 = 10_000;

/// Recommended default: owner must check in every 180 days.
pub const DEFAULT_CHECK_IN_INTERVAL: i64 = 180 * 24 * 60 * 60;

/// Hard floor on check_in_interval; prevents setting a value so short the switch
/// becomes immediately triggerable.
pub const MIN_CHECK_IN_INTERVAL: i64 = 24 * 60 * 60; // 1 day

/// After the 180-day deadline passes, the owner has this many seconds to either
/// check in (resetting the full timer) or cancel before anyone can call trigger().
pub const DEFAULT_GRACE_PERIOD: i64 = 7 * 24 * 60 * 60;

/// How far in advance the off-chain monitor fires a reminder.
pub const REMINDER_THRESHOLD_SECS: i64 = 10 * 24 * 60 * 60;
