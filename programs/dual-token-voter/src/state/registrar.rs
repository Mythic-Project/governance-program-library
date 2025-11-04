use anchor_lang::prelude::*;

/// Registrar stores the configuration for the dual-token-voter plugin
#[account]
pub struct Registrar {
    /// The SPL Governance instance this plugin is connected to
    pub governance_program_id: Pubkey,

    /// The realm this registrar belongs to
    pub realm: Pubkey,

    /// The governing token mint for this registrar
    pub realm_governing_token_mint: Pubkey,

    /// The first token (token_a) mint - receives 1:1 voting power
    pub token_a_mint: Pubkey,

    /// The second token (token_b) mint - uses exchange rate for voting power conversion
    pub token_b_mint: Pubkey,

    /// Time window in seconds that deposits must precede proposal creation to be eligible for voting
    /// Example: 72 hours (259200 seconds) means deposits must occur 72+ hours before proposal creation
    pub eligibility_window_seconds: i64,

    /// PDA bump seed
    pub registrar_bump: u8,

    /// Reserved space for future upgrades
    pub reserved: [u8; 128],
}

impl Registrar {
    pub const fn get_space() -> usize {
        8 +  // discriminator
        32 + // governance_program_id
        32 + // realm
        32 + // realm_governing_token_mint
        32 + // token_a_mint
        32 + // token_b_mint
        8 +  // eligibility_window_seconds
        1 +  // registrar_bump
        128  // reserved
    }
}
