# Deadman Switch 🔐⏳

A deadman switch program written in Solana, designed as an on-chain inheritance program. Owner checks in periodically; miss the deadline long enough and the funds automatically distribute to whoever you've set as beneficiaries. Built with Anchor 1.0. Still at work in progress.

---

## How it works

Owner deposits SOL, sets a check-in interval (180 days by default), and calls `check_in` periodically to reset the timer. Miss the deadline, a grace window opens. Miss that too, and anyone can call `trigger()` to distribute funds proportionally.

Trigger is permissionless: any wallet can call it, so there's no single point of failure.

### State machine

```
┌─────────────────────────────────────────────────────────────────────┐
│                                                                     │
│   ACTIVE               GRACE PERIOD          FULLY EXPIRED         │
│   (0 → 180d)           (180d → 187d)         (187d+)               │
│                                                                     │
│   check_in ✓           check_in ✓            check_in ✗            │
│   withdraw ✓           withdraw ✗            withdraw ✗            │
│   deposit  ✓           deposit  ✓            deposit  ✗            │
│   cancel   ✓           cancel   ✓            cancel   ✗            │
│   trigger  ✗           trigger  ✗            trigger  ✓ (anyone)   │
│   upd_ben  ✓           upd_ben  ✗            upd_ben  ✗            │
│   upd_int  ✓           upd_int  ✗            upd_int  ✗            │
│                                                                     │
│                    ──────────────────────────────────────────►      │
│                    last_check_in + 180d     + 7d grace             │
└─────────────────────────────────────────────────────────────────────┘
```

A few things that aren't obvious from the table:
- `withdraw` locks at the **deadline**, not grace end. Prevents the owner from draining funds beneficiaries are owed.
- `cancel` works during grace but blocks once fully expired; prevents front-running `trigger()`.
- `update_beneficiaries` and `update_interval` both lock at the deadline. Check in first, then update.
- Checking in during grace resets the **full** 180-day timer.

---

## Instructions

| Instruction            | Who    | Notes                                                                  |
| ---------------------- | ------ | ---------------------------------------------------------------------- |
| `initialize`           | owner  | creates the switch; optional initial deposit                           |
| `check_in`             | owner  | resets the timer; works during grace                                   |
| `deposit`              | anyone | top up before trigger                                                  |
| `withdraw`             | owner  | active period only; preserves rent minimum                             |
| `trigger`              | anyone | after full expiry; pass beneficiaries as `remaining_accounts` in order |
| `add_message`          | owner  | set or clear a note for beneficiaries                                  |
| `update_beneficiaries` | owner  | full replacement; blocked after deadline                               |
| `update_interval`      | owner  | blocked after deadline; can't shorten into an immediate trigger        |
| `cancel`               | owner  | closes PDA, returns everything; blocked after full expiry              |

Beneficiaries are stored as basis points summing to 10,000 (= 100%). Max 10. Last beneficiary absorbs any rounding dust at trigger time.

---

## Quick start

Requires Anchor 1.0+ and a Solana toolchain.

```bash
anchor build
cargo test
anchor deploy
```

---

*Built with Anchor and Solana*
