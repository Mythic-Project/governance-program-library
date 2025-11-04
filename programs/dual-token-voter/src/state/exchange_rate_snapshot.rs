use anchor_lang::prelude::*;

/// Exchange rate snapshot locked at proposal creation
/// This ensures fair voting by preventing exchange rate manipulation during voting period
#[account]
pub struct ExchangeRateSnapshot {
    /// The registrar this snapshot belongs to
    pub registrar: Pubkey,

    /// The proposal this snapshot is for
    pub proposal: Pubkey,

    /// Exchange rate from token_b to token_a
    /// Scaled by 1e9 (1_000_000_000) for precision
    /// Example: If 1 token_b = 0.95 token_a, token_b_exchange_rate = 950_000_000
    pub token_b_exchange_rate: u64,

    /// Unix timestamp when this snapshot was created (proposal creation time)
    /// Used to calculate eligibility cutoff: cutoff = snapshot_timestamp - eligibility_window_seconds
    pub snapshot_timestamp: i64,

    /// The slot when this snapshot was created (for expiry tracking)
    pub snapshot_slot: u64,

    /// PDA bump seed
    pub bump: u8,
}

impl ExchangeRateSnapshot {
    pub const fn get_space() -> usize {
        8 +  // discriminator
        32 + // registrar
        32 + // proposal
        8 +  // token_b_exchange_rate
        8 +  // snapshot_timestamp
        8 +  // snapshot_slot
        1    // bump
    }
}
