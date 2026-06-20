use {
    anchor_lang::{solana_program::instruction::Instruction, InstructionData, ToAccountMetas},
    deadman_switch::state::Beneficiary,
    litesvm::LiteSVM,
    solana_keypair::Keypair,
    solana_message::{Message, VersionedMessage},
    solana_signer::Signer,
    solana_transaction::versioned::VersionedTransaction,
};

use anchor_lang::solana_program::pubkey::Pubkey;

fn program_id() -> Pubkey {
    deadman_switch::id()
}

fn switch_pda(owner: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"deadman-switch", owner.as_ref()], &program_id())
}

fn system_program_id() -> Pubkey {
    anchor_lang::solana_program::system_program::ID
}

fn send(
    svm: &mut LiteSVM,
    payer: &Keypair,
    instruction: Instruction,
    signers: &[&Keypair],
) -> litesvm::types::TransactionResult {
    let blockhash = svm.latest_blockhash();
    let msg = Message::new_with_blockhash(&[instruction], Some(&payer.pubkey()), &blockhash);
    let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(msg), signers).unwrap();
    svm.send_transaction(tx)
}

fn make_svm() -> (LiteSVM, Keypair) {
    let owner = Keypair::new();
    let mut svm = LiteSVM::new();
    let bytes = include_bytes!("../../../target/deploy/deadman_switch.so");
    svm.add_program(program_id(), bytes).unwrap();
    svm.airdrop(&owner.pubkey(), 10_000_000_000).unwrap();
    (svm, owner)
}

// ── initialize ────────────────────────────────────────────────────────────────

#[test]
fn test_initialize_happy_path() {
    let (mut svm, owner) = make_svm();
    let (pda, _) = switch_pda(&owner.pubkey());

    let beneficiary = Keypair::new();
    let ix = Instruction::new_with_bytes(
        program_id(),
        &deadman_switch::instruction::Initialize {
            beneficiaries: vec![Beneficiary {
                address: beneficiary.pubkey(),
                share_bps: 10_000,
            }],
            check_in_interval: 86_400,        // 1 day
            grace_period: 3_600,              // 1 hour grace
            deposit_amount: 1_000_000_000,
            message: Some("For my family".to_string()),
        }
        .data(),
        deadman_switch::accounts::Initialize {
            owner: owner.pubkey(),
            switch: pda,
            system_program: system_program_id(),
        }
        .to_account_metas(None),
    );

    let res = send(&mut svm, &owner, ix, &[&owner]);
    assert!(res.is_ok(), "initialize failed: {:?}", res.err());
}

#[test]
fn test_initialize_invalid_shares_fails() {
    let (mut svm, owner) = make_svm();
    let (pda, _) = switch_pda(&owner.pubkey());

    let beneficiary = Keypair::new();
    let ix = Instruction::new_with_bytes(
        program_id(),
        &deadman_switch::instruction::Initialize {
            beneficiaries: vec![Beneficiary {
                address: beneficiary.pubkey(),
                share_bps: 5_000, // only 50% — must fail
            }],
            check_in_interval: 86_400,
            grace_period: 3_600,
            deposit_amount: 0,
            message: None,
        }
        .data(),
        deadman_switch::accounts::Initialize {
            owner: owner.pubkey(),
            switch: pda,
            system_program: system_program_id(),
        }
        .to_account_metas(None),
    );

    let res = send(&mut svm, &owner, ix, &[&owner]);
    assert!(res.is_err(), "expected failure for invalid shares");
}

#[test]
fn test_initialize_multiple_beneficiaries() {
    let (mut svm, owner) = make_svm();
    let (pda, _) = switch_pda(&owner.pubkey());

    let alice = Keypair::new();
    let bob = Keypair::new();
    let ix = Instruction::new_with_bytes(
        program_id(),
        &deadman_switch::instruction::Initialize {
            beneficiaries: vec![
                Beneficiary { address: alice.pubkey(), share_bps: 6_000 },
                Beneficiary { address: bob.pubkey(),   share_bps: 4_000 },
            ],
            check_in_interval: 3_600,
            grace_period: 600,               // 10-minute grace
            deposit_amount: 500_000_000,
            message: None,
        }
        .data(),
        deadman_switch::accounts::Initialize {
            owner: owner.pubkey(),
            switch: pda,
            system_program: system_program_id(),
        }
        .to_account_metas(None),
    );

    let res = send(&mut svm, &owner, ix, &[&owner]);
    assert!(res.is_ok(), "multi-beneficiary initialize failed: {:?}", res.err());
}

// ── check_in ─────────────────────────────────────────────────────────────────

fn initialize_switch(svm: &mut LiteSVM, owner: &Keypair, beneficiary: &Keypair) {
    let (pda, _) = switch_pda(&owner.pubkey());
    let ix = Instruction::new_with_bytes(
        program_id(),
        &deadman_switch::instruction::Initialize {
            beneficiaries: vec![Beneficiary {
                address: beneficiary.pubkey(),
                share_bps: 10_000,
            }],
            check_in_interval: 86_400,
            grace_period: 3_600,
            deposit_amount: 0,
            message: None,
        }
        .data(),
        deadman_switch::accounts::Initialize {
            owner: owner.pubkey(),
            switch: pda,
            system_program: system_program_id(),
        }
        .to_account_metas(None),
    );
    send(svm, owner, ix, &[owner]).unwrap();
}

#[test]
fn test_check_in_by_owner_succeeds() {
    let (mut svm, owner) = make_svm();
    let beneficiary = Keypair::new();
    initialize_switch(&mut svm, &owner, &beneficiary);

    let (pda, _) = switch_pda(&owner.pubkey());
    let ix = Instruction::new_with_bytes(
        program_id(),
        &deadman_switch::instruction::CheckIn {}.data(),
        deadman_switch::accounts::CheckIn {
            owner: owner.pubkey(),
            switch: pda,
        }
        .to_account_metas(None),
    );

    let res = send(&mut svm, &owner, ix, &[&owner]);
    assert!(res.is_ok(), "check_in failed: {:?}", res.err());
}

#[test]
fn test_check_in_wrong_pda_fails() {
    let (mut svm, owner) = make_svm();
    let attacker = Keypair::new();
    svm.airdrop(&attacker.pubkey(), 1_000_000_000).unwrap();

    let beneficiary = Keypair::new();
    initialize_switch(&mut svm, &owner, &beneficiary);

    let (attacker_pda, _) = switch_pda(&attacker.pubkey());
    let ix = Instruction::new_with_bytes(
        program_id(),
        &deadman_switch::instruction::CheckIn {}.data(),
        deadman_switch::accounts::CheckIn {
            owner: attacker.pubkey(),
            switch: attacker_pda,
        }
        .to_account_metas(None),
    );

    let res = send(&mut svm, &attacker, ix, &[&attacker]);
    assert!(res.is_err(), "attacker should not be able to check in");
}
