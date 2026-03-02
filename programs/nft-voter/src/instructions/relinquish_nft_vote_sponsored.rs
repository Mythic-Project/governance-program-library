use crate::error::NftVoterError;
use crate::state::*;
use crate::tools::governance::get_vote_record_address;
use anchor_lang::prelude::*;
use spl_governance::state::{enums::ProposalState, governance, proposal};
use spl_governance_tools::account::dispose_account;

/// Disposes NftVoteRecordSponsored accounts and returns rent to the Sponsor PDA
/// It can only be executed when voting on the target Proposal ended or voter withdrew vote from the Proposal
///
/// Unlike relinquish_nft_vote, this instruction enforces that lamports are returned to the
/// sponsor PDA that originally paid for the records, not an arbitrary beneficiary
#[derive(Accounts)]
pub struct RelinquishNftVoteSponsored<'info> {
    /// The NFT voting Registrar
    pub registrar: Account<'info, Registrar>,

    #[account(
        mut,
        constraint = voter_weight_record.realm == registrar.realm
        @ NftVoterError::InvalidVoterWeightRecordRealm,

        constraint = voter_weight_record.governing_token_mint == registrar.governing_token_mint
        @ NftVoterError::InvalidVoterWeightRecordMint,
    )]
    pub voter_weight_record: Account<'info, VoterWeightRecord>,

    /// The Sponsor PDA that receives the returned rent
    /// This MUST match the sponsor stored in each NftVoteRecordSponsored
    #[account(
        mut,
        seeds = [b"sponsor".as_ref(), registrar.key().as_ref()],
        bump,
    )]
    pub sponsor: SystemAccount<'info>,

    /// CHECK: Owned by spl-governance instance specified in registrar.governance_program_id
    /// Governance account the Proposal is for
    #[account(owner = registrar.governance_program_id)]
    pub governance: UncheckedAccount<'info>,

    /// CHECK: Owned by spl-governance instance specified in registrar.governance_program_id
    #[account(owner = registrar.governance_program_id)]
    pub proposal: UncheckedAccount<'info>,

    /// TokenOwnerRecord of the voter who cast the original vote
    #[account(
            owner = registrar.governance_program_id
         )]
    /// CHECK: Owned by spl-governance instance specified in registrar.governance_program_id
    voter_token_owner_record: UncheckedAccount<'info>,

    /// Authority of the voter who cast the original vote
    /// It can be either governing_token_owner or its delegate and must sign this instruction
    pub voter_authority: Signer<'info>,

    /// CHECK: Owned by spl-governance instance specified in registrar.governance_program_id
    /// The account is used to validate that it doesn't exist and if it doesn't then Anchor owner check throws error
    /// The check is disabled here and performed inside the instruction
    pub vote_record: UncheckedAccount<'info>,
}

/// Disposes NftVoteRecordSponsored accounts and returns lamports to the sponsor
///
/// remaining_accounts should be the NftVoteRecordSponsored PDAs to dispose
pub fn relinquish_nft_vote_sponsored(ctx: Context<RelinquishNftVoteSponsored>) -> Result<()> {
    let registrar = &ctx.accounts.registrar;
    let voter_weight_record = &mut ctx.accounts.voter_weight_record;

    let governing_token_owner = resolve_governing_token_owner(
        registrar,
        &ctx.accounts.voter_token_owner_record,
        &ctx.accounts.voter_authority,
        voter_weight_record,
    )?;

    // Ensure the Governance belongs to Registrar.realm and is owned by Registrar.governance_program_id
    let _governance = governance::get_governance_data_for_realm(
        &registrar.governance_program_id,
        &ctx.accounts.governance,
        &registrar.realm,
    )?;

    // Ensure the Proposal belongs to Governance from Registrar.realm and Registrar.governing_token_mint
    let proposal = proposal::get_proposal_data_for_governance_and_governing_mint(
        &registrar.governance_program_id,
        &ctx.accounts.proposal,
        &ctx.accounts.governance.key(),
        &registrar.governing_token_mint,
    )?;

    // If the Proposal is still in Voting state then we can only Relinquish the NFT votes if the Vote was withdrawn in spl-gov first
    if proposal.state == ProposalState::Voting {
        let vote_record_info = &ctx.accounts.vote_record.to_account_info();

        // Ensure the given VoteRecord address matches the expected PDA
        let vote_record_key = get_vote_record_address(
            &registrar.governance_program_id,
            &registrar.realm,
            &registrar.governing_token_mint,
            &governing_token_owner,
            &ctx.accounts.proposal.key(),
        );

        require!(
            vote_record_key == vote_record_info.key(),
            NftVoterError::InvalidVoteRecordForNftVoteRecord
        );

        require!(
            // VoteRecord doesn't exist if data is empty or account_type is 0 when the account was disposed in the same Tx
            vote_record_info.data_is_empty() || vote_record_info.try_borrow_data().unwrap()[0] == 0,
            NftVoterError::VoteRecordMustBeWithdrawn
        );
    }

    // Prevent relinquishing NftVoteRecordSponsored within the VoterWeightRecord expiration period
    // This prevents sandwich attacks when multiple voter-weight plugins are stacked
    if voter_weight_record.voter_weight_expiry >= Some(Clock::get()?.slot) {
        return err!(NftVoterError::VoterWeightRecordMustBeExpired);
    }

    let sponsor_key = ctx.accounts.sponsor.key();

    // Dispose all NftVoteRecordSponsored and return lamports to sponsor
    for nft_vote_record_info in ctx.remaining_accounts.iter() {
        // Ensure NftVoteRecordSponsored is for the correct proposal, token owner, AND sponsor
        // This is critical for security - ensures lamports return to the right sponsor
        let _nft_vote_record = get_nft_vote_record_sponsored_data_for_proposal_and_token_owner_and_sponsor(
            nft_vote_record_info,
            &ctx.accounts.proposal.key(),
            &governing_token_owner,
            &sponsor_key,
        )?;

        // Dispose the account and return lamports to the sponsor
        dispose_account(nft_vote_record_info, &ctx.accounts.sponsor.to_account_info())?;
    }

    // Reset VoterWeightRecord and set expiry to expired to prevent it from being used
    voter_weight_record.voter_weight = 0;
    voter_weight_record.voter_weight_expiry = Some(0);

    voter_weight_record.weight_action_target = None;

    Ok(())
}
