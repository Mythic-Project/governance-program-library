use crate::error::NftVoterError;
use crate::state::*;
use crate::tools::account::create_and_serialize_account_from_pda;
use crate::{id, state::get_sponsor_seeds};
use anchor_lang::prelude::*;
use anchor_lang::Accounts;
use itertools::Itertools;

/// Casts NFT vote with rent sponsored by the DAO
/// The NFTs used for voting are tracked using NftVoteRecordSponsored accounts
/// The rent for these accounts is paid from the Sponsor PDA instead of the voter
///
/// This instruction updates VoterWeightRecord which is valid for the current Slot and the target Proposal only
/// and hence the instruction has to be executed inside the same transaction as spl-gov.CastVote
///
/// CastNftVoteSponsored is accumulative and can be invoked using several transactions if voter owns more than 5 NFTs
/// In this scenario only the last CastNftVoteSponsored should be bundled with spl-gov.CastVote in the same transaction
#[derive(Accounts)]
pub struct CastNftVoteSponsored<'info> {
    /// The NFT voting registrar
    pub registrar: Account<'info, Registrar>,

    #[account(
        mut,
        constraint = voter_weight_record.realm == registrar.realm
        @ NftVoterError::InvalidVoterWeightRecordRealm,

        constraint = voter_weight_record.governing_token_mint == registrar.governing_token_mint
        @ NftVoterError::InvalidVoterWeightRecordMint,
    )]
    pub voter_weight_record: Account<'info, VoterWeightRecord>,

    /// The Sponsor PDA that pays for NftVoteRecordSponsored rent
    #[account(
        mut,
        seeds = [b"sponsor".as_ref(), registrar.key().as_ref()],
        bump,
    )]
    pub sponsor: SystemAccount<'info>,

    /// TokenOwnerRecord of the voter who casts the vote
    #[account(
        owner = registrar.governance_program_id
     )]
    /// CHECK: Owned by spl-governance instance specified in registrar.governance_program_id
    voter_token_owner_record: UncheckedAccount<'info>,

    /// Authority of the voter who casts the vote
    /// It can be either governing_token_owner or its delegate and must sign this instruction
    pub voter_authority: Signer<'info>,

    pub system_program: Program<'info, System>,
}

/// Casts vote with the NFT, with rent sponsored by the DAO
///
/// remaining_accounts are passed in tuples of 3:
/// - nft_info: The NFT token account
/// - nft_metadata_info: The NFT metadata account
/// - nft_vote_record_sponsored_info: The NftVoteRecordSponsored PDA to create
pub fn cast_nft_vote_sponsored<'info>(
    ctx: Context<'_, '_, '_, 'info, CastNftVoteSponsored<'info>>,
    proposal: Pubkey,
) -> Result<()> {
    let registrar = &ctx.accounts.registrar;
    let voter_weight_record = &mut ctx.accounts.voter_weight_record;

    let governing_token_owner = resolve_governing_token_owner(
        registrar,
        &ctx.accounts.voter_token_owner_record,
        &ctx.accounts.voter_authority,
        voter_weight_record,
    )?;

    let mut voter_weight = 0u64;

    // Ensure all voting nfts in the batch are unique
    let mut unique_nft_mints = vec![];

    let rent = Rent::get()?;

    // Calculate rent needed per NftVoteRecordSponsored account
    let nft_vote_record_size = std::mem::size_of::<NftVoteRecordSponsored>();
    let rent_per_record = rent.minimum_balance(nft_vote_record_size);

    // Count how many NFTs we're processing
    let nft_count = ctx.remaining_accounts.len() / 3;

    // Verify sponsor has enough funds
    let total_rent_needed = rent_per_record
        .checked_mul(nft_count as u64)
        .ok_or(NftVoterError::InsufficientSponsorFunds)?;

    let sponsor_min_balance = rent.minimum_balance(0);
    let sponsor_available = ctx
        .accounts
        .sponsor
        .lamports()
        .saturating_sub(sponsor_min_balance);

    require!(
        sponsor_available >= total_rent_needed,
        NftVoterError::InsufficientSponsorFunds
    );

    // Build sponsor seeds for signing
    let registrar_key = registrar.key();
    let sponsor_seeds = get_sponsor_seeds(&registrar_key);
    let (_, sponsor_bump) = Pubkey::find_program_address(&sponsor_seeds, &id());
    let sponsor_seeds_with_bump: &[&[u8]] = &[b"sponsor", registrar_key.as_ref(), &[sponsor_bump]];

    let sponsor_info = ctx.accounts.sponsor.to_account_info();

    for (nft_info, nft_metadata_info, nft_vote_record_sponsored_info) in
        ctx.remaining_accounts.iter().tuples()
    {
        let (nft_vote_weight, nft_mint) = resolve_nft_vote_weight_and_mint(
            registrar,
            &governing_token_owner,
            nft_info,
            nft_metadata_info,
            &mut unique_nft_mints,
        )?;

        voter_weight = voter_weight.checked_add(nft_vote_weight).unwrap();

        // Ensure the NftVoteRecordSponsored doesn't already exist
        require!(
            nft_vote_record_sponsored_info.data_is_empty(),
            NftVoterError::NftAlreadyVoted
        );

        let nft_vote_record_sponsored = NftVoteRecordSponsored {
            account_discriminator: NftVoteRecordSponsored::ACCOUNT_DISCRIMINATOR,
            proposal,
            nft_mint,
            governing_token_owner,
            sponsor: ctx.accounts.sponsor.key(),
            reserved: [0; 8],
        };

        // Create NftVoteRecordSponsored funded by the sponsor PDA
        let nft_vote_record_seeds =
            get_nft_vote_record_sponsored_seeds(&proposal, &nft_mint);

        create_and_serialize_account_from_pda(
            &sponsor_info,
            sponsor_seeds_with_bump,
            nft_vote_record_sponsored_info,
            &nft_vote_record_seeds,
            &nft_vote_record_sponsored,
            &id(),
            &ctx.accounts.system_program.to_account_info(),
            &rent,
        )?;
    }

    if voter_weight_record.weight_action_target == Some(proposal)
        && voter_weight_record.weight_action == Some(VoterWeightAction::CastVote)
    {
        // If cast_nft_vote_sponsored is called for the same proposal then we keep accumulating the weight
        voter_weight_record.voter_weight = voter_weight_record
            .voter_weight
            .checked_add(voter_weight)
            .unwrap();
    } else {
        voter_weight_record.voter_weight = voter_weight;
    }

    // The record is only valid as of the current slot
    voter_weight_record.voter_weight_expiry = Some(Clock::get()?.slot);

    // The record is only valid for casting vote on the given Proposal
    voter_weight_record.weight_action = Some(VoterWeightAction::CastVote);
    voter_weight_record.weight_action_target = Some(proposal);

    Ok(())
}
