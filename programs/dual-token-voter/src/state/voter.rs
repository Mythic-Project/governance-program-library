use anchor_lang::prelude::*;

/// Voter account tracks token deposits and eligibility for a specific user
#[account]
pub struct Voter {
    /// The user who owns this voter account
    pub voter_authority: Pubkey,

    /// The registrar this voter belongs to
    pub registrar: Pubkey,

    /// Amount of token_a deposited by the user (1:1 voting power)
    pub token_a_deposited_amount: u64,

    /// Amount of token_b deposited by the user (converted via exchange rate)
    pub token_b_deposited_amount: u64,

    /// Unix timestamp when tokens were last deposited or withdrawn
    /// Used for eligibility window enforcement - deposits must occur before
    /// (proposal_creation_timestamp - eligibility_window_seconds) to be eligible for voting
    pub tokens_update_timestamp: i64,

    /// PDA bump seed for voter account
    pub voter_bump: u8,

    /// PDA bump seed for voter_weight_record account
    pub voter_weight_record_bump: u8,

    /// Reserved space for future upgrades
    pub reserved: [u8; 128],
}

impl Voter {
    pub const fn get_space() -> usize {
        8 +   // discriminator
        32 +  // voter_authority
        32 +  // registrar
        8 +   // token_a_deposited_amount
        8 +   // token_b_deposited_amount
        8 +   // tokens_update_timestamp (i64)
        1 +   // voter_bump
        1 +   // voter_weight_record_bump
        128   // reserved
    }
}
