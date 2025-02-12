use crate::error::SnapshotVoterError;
use crate::state::*;
use crate::tools::merkle::verify_proof;
use anchor_lang::prelude::*;
use spl_governance::state::token_owner_record;
use std::convert::TryInto;

/// Updates VoterWeightRecord based on Realm DAO membership
/// The membership is evaluated via a valid TokenOwnerRecord which must belong to one of the configured spl-governance instances
///
/// This instruction sets VoterWeightRecord.voter_weight which is valid for the current slot only
/// and must be executed inside the same transaction as the corresponding spl-gov instruction
#[derive(Accounts)]
pub struct UpdateVoterWeightRecord<'info> {
    /// The RealmVoter voting Registrar
    pub registrar: Account<'info, Registrar>,

    #[account(
        mut,
        constraint = voter_weight_record.realm == registrar.realm
        @ SnapshotVoterError::InvalidVoterWeightRecordRealm,

        constraint = voter_weight_record.governing_token_mint == registrar.governing_token_mint
        @ SnapshotVoterError::InvalidVoterWeightRecordMint,
    )]
    pub voter_weight_record: Account<'info, VoterWeightRecord>,

    /// TokenOwnerRecord for any of the configured spl-governance instances
    /// CHECK: Owned by any of the spl-governance instances specified in registrar.governance_program_configs
    pub token_owner_record: UncheckedAccount<'info>,

    /// CHECK: Checked below.
    pub proposal: UncheckedAccount<'info>,
}

pub fn update_voter_weight_record(
    ctx: Context<UpdateVoterWeightRecord>,
    amount: u64,
    verification_data: Vec<u8>,
) -> Result<()> {
    let registrar = &ctx.accounts.registrar;
    let voter_weight_record = &mut ctx.accounts.voter_weight_record;

    let governance_program_id = ctx.accounts.token_owner_record.owner;

    if ctx.accounts.registrar.root == [0; 32] {
        return Err(SnapshotVoterError::MerkleRootMissing.into());
    }

    if ctx.accounts.registrar.proposal != ctx.accounts.proposal.key() {
        return Err(SnapshotVoterError::ProposalMismatch.into());
    }

    // Do the verification
    let verification_index_array: [u8; 8] = verification_data[0..8]
        .try_into()
        .expect("Invalid verification data");
    let verification_index: u64 = u64::from_le_bytes(verification_index_array);

    // msg!("Verification Data {:02X?}", verification_data);
    // msg!("Verification Data Length {}", verification_data.len());
    // msg!("Amount {}", amount);
    // msg!("Verification Index {}", verification_index);

    let leaf: [u8; 32] = anchor_lang::solana_program::keccak::hashv(&[
        &verification_index.to_le_bytes(),
        &voter_weight_record.governing_token_owner.key().to_bytes(),
        &amount.to_le_bytes(),
    ])
    .0;

    let mut proof: Vec<[u8; 32]> = Vec::new();
    // Convert the Vec<u8> into Vec<[u8; 32]> and call the verifier
    let mut iter = verification_data[8..].chunks(32);
    while iter.len() > 0 {
        let next_hash: [u8; 32] = iter
            .next()
            .unwrap()
            .try_into()
            .expect("Invalid verification data");
        proof.push(next_hash);
        // msg!("Proof hash {:02X?}", next_hash);
    }

    // This is the actual verification.
    verify_proof(proof, ctx.accounts.registrar.root, leaf);

    let token_owner_record = token_owner_record::get_token_owner_record_data(
        governance_program_id,
        &ctx.accounts.token_owner_record,
    )?;
    
    // Ensure VoterWeightRecord and TokenOwnerRecord are for the same governing_token_owner
    require_eq!(
        token_owner_record.governing_token_owner,
        voter_weight_record.governing_token_owner,
        SnapshotVoterError::GoverningTokenOwnerMustMatch
    );

    // Membership of the Realm the plugin is configured for is not allowed as a source of governance power
    require_neq!(
        token_owner_record.realm,
        registrar.realm,
        SnapshotVoterError::TokenOwnerRecordFromOwnRealmNotAllowed
    );

    // Setup voter_weight
    voter_weight_record.voter_weight = amount;

    // Record is only valid as of the current slot
    voter_weight_record.voter_weight_expiry = Some(Clock::get()?.slot);

    // Set action and target to None to indicate the weight is valid for any action and target
    voter_weight_record.weight_action = None;
    // the weight is valid for only a specific proposal
    voter_weight_record.weight_action_target = Some(registrar.proposal);

    Ok(())
}
