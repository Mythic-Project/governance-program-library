use anchor_lang::prelude::*;
use gpl_shared::{anchor::{DISCRIMINATOR_SIZE, PUBKEY_SIZE}, compose::VoterWeightRecordBase};

/// VoterWeightAction enum as defined in spl-governance-addin-api
/// It's redefined here for Anchor to export it to IDL
#[derive(AnchorSerialize, AnchorDeserialize, Debug, Clone, Copy, PartialEq)]
pub enum VoterWeightAction {
    /// Cast vote for a proposal. Target: Proposal
    CastVote,

    /// Comment a proposal. Target: Proposal
    CommentProposal,

    /// Create Governance within a realm. Target: Realm
    CreateGovernance,

    /// Create a proposal for a governance. Target: Governance
    CreateProposal,

    /// Signs off a proposal for a governance. Target: Proposal
    /// Note: SignOffProposal is not supported in the current version
    SignOffProposal,
}

/// VoterWeightRecord account as defined in spl-governance-addin-api
/// It's redefined here without account_discriminator for Anchor to treat it as native account
///
/// The account is used as an api interface to provide voting power to the governance program from external addin contracts
#[account]
#[derive(Debug, PartialEq)]
pub struct VoterWeightRecord {
    /// The Realm the VoterWeightRecord belongs to
    pub realm: Pubkey,

    /// Governing Token Mint the VoterWeightRecord is associated with
    pub governing_token_mint: Pubkey,

    /// The owner of the governing token and voter
    pub governing_token_owner: Pubkey,

    /// Voter's weight (combined base weight + converted secondary token weight)
    pub voter_weight: u64,

    /// The slot when the voting weight expires
    pub voter_weight_expiry: Option<u64>,

    /// The governance action the voter's weight pertains to
    pub weight_action: Option<VoterWeightAction>,

    /// The target the voter's weight action pertains to (proposal, governance, etc.)
    pub weight_action_target: Option<Pubkey>,

    /// Reserved space for future upgrades
    pub reserved: [u8; 8],
}

impl VoterWeightRecord {
    pub fn get_space() -> usize {
        DISCRIMINATOR_SIZE + PUBKEY_SIZE * 4 + 8 + 1 + 8 + 1 + 1 + 1 + 8
    }
}

impl<'a> VoterWeightRecordBase<'a> for VoterWeightRecord {
    fn get_governing_token_mint(&'a self) -> &'a Pubkey {
        &self.governing_token_mint
    }

    fn get_governing_token_owner(&'a self) -> &'a Pubkey {
        &self.governing_token_owner
    }
}

impl Default for VoterWeightRecord {
    fn default() -> Self {
        Self {
            realm: Default::default(),
            governing_token_mint: Default::default(),
            governing_token_owner: Default::default(),
            voter_weight: Default::default(),
            voter_weight_expiry: Some(0),
            weight_action: Some(VoterWeightAction::CastVote),
            weight_action_target: Some(Default::default()),
            reserved: Default::default(),
        }
    }
}
