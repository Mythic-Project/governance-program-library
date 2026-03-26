use anchor_lang::prelude::*;
use anchor_lang::solana_program::program_pack::IsInitialized;

use spl_governance_tools::account::{get_account_data, AccountMaxSize};

use crate::{error::NftVoterError, id};

/// Sponsored vote record indicating the given NFT voted on the Proposal
/// The rent was paid by a Sponsor account and must be returned there on relinquish
///
/// PDA seeds: ["nft-vote-record", proposal, nft_mint]
/// Shares the same PDA namespace as NftVoteRecord to prevent double-voting
/// across sponsored and non-sponsored paths. Discriminators distinguish the types.
#[derive(Clone, Debug, PartialEq, borsh_1::BorshDeserialize, borsh_1::BorshSerialize)]
#[borsh(crate = "borsh_1")]
pub struct NftVoteRecordSponsored {
    /// NftVoteRecordSponsored discriminator sha256("account:NftVoteRecordSponsored")[..8]
    /// Note: The discriminator is used explicitly because NftVoteRecordSponsored
    /// are created and consumed dynamically using remaining_accounts
    pub account_discriminator: [u8; 8],

    /// Proposal which was voted on
    pub proposal: Pubkey,

    /// The mint of the NFT which was used for the vote
    pub nft_mint: Pubkey,

    /// The voter who casted this vote
    /// It's a Realm member pubkey corresponding to TokenOwnerRecord.governing_token_owner
    pub governing_token_owner: Pubkey,

    /// The sponsor account that paid for this record's rent
    /// Lamports MUST be returned here on relinquish
    pub sponsor: Pubkey,

    /// Reserved for future upgrades
    pub reserved: [u8; 8],
}

impl NftVoteRecordSponsored {
    /// sha256("account:NftVoteRecordSponsored")[..8]
    /// Computed: python -c "import hashlib; print(list(hashlib.sha256(b'account:NftVoteRecordSponsored').digest()[:8]))"
    pub const ACCOUNT_DISCRIMINATOR: [u8; 8] = [78, 213, 79, 243, 142, 82, 85, 174];
}

impl AccountMaxSize for NftVoteRecordSponsored {}

impl IsInitialized for NftVoteRecordSponsored {
    fn is_initialized(&self) -> bool {
        self.account_discriminator == NftVoteRecordSponsored::ACCOUNT_DISCRIMINATOR
    }
}

/// Returns NftVoteRecordSponsored PDA seeds
pub fn get_nft_vote_record_sponsored_seeds<'a>(
    proposal: &'a Pubkey,
    nft_mint: &'a Pubkey,
) -> [&'a [u8]; 3] {
    [
        b"nft-vote-record",
        proposal.as_ref(),
        nft_mint.as_ref(),
    ]
}

/// Returns NftVoteRecordSponsored PDA address
pub fn get_nft_vote_record_sponsored_address(proposal: &Pubkey, nft_mint: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &get_nft_vote_record_sponsored_seeds(proposal, nft_mint),
        &id(),
    )
    .0
}

/// Deserializes account and checks owner program
pub fn get_nft_vote_record_sponsored_data(
    nft_vote_record_info: &AccountInfo,
) -> Result<NftVoteRecordSponsored> {
    Ok(get_account_data::<NftVoteRecordSponsored>(
        &id(),
        nft_vote_record_info,
    )?)
}

/// Deserializes and validates NftVoteRecordSponsored for the given proposal, token owner, and sponsor
pub fn get_nft_vote_record_sponsored_data_for_proposal_and_token_owner_and_sponsor(
    nft_vote_record_info: &AccountInfo,
    proposal: &Pubkey,
    governing_token_owner: &Pubkey,
    sponsor: &Pubkey,
) -> Result<NftVoteRecordSponsored> {
    let nft_vote_record = get_nft_vote_record_sponsored_data(nft_vote_record_info)?;

    require!(
        nft_vote_record.proposal == *proposal,
        NftVoterError::InvalidProposalForNftVoteRecord
    );

    require!(
        nft_vote_record.governing_token_owner == *governing_token_owner,
        NftVoterError::InvalidTokenOwnerForNftVoteRecord
    );

    require!(
        nft_vote_record.sponsor == *sponsor,
        NftVoterError::InvalidSponsorForNftVoteRecord
    );

    Ok(nft_vote_record)
}
