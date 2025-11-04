use anchor_lang::prelude::*;

mod instructions;
use instructions::*;
use state::VoterWeightAction;

pub mod error;
pub mod state;
pub mod orca;

declare_id!("CVSU6KL6zzJrreYn9G5L5h9Z6FnKAKBQNaeivrPzti2c");

#[program]
pub mod dual_token_voter {
    use super::*;

    /// Creates the registrar for the dual-token-voter plugin
    pub fn create_registrar(
        ctx: Context<CreateRegistrar>,
        eligibility_window_seconds: i64,
    ) -> Result<()> {
        log_version();
        instructions::create_registrar(ctx, eligibility_window_seconds)
    }

    /// Creates voter and voter_weight_record accounts for a user
    pub fn create_voter_weight_record(
        ctx: Context<CreateVoterWeightRecord>,
    ) -> Result<()> {
        log_version();
        instructions::create_voter_weight_record(ctx)
    }

    /// Creates an exchange rate snapshot for a proposal by fetching rate from Orca
    pub fn create_exchange_rate_snapshot(
        ctx: Context<CreateExchangeRateSnapshot>,
        proposal: Pubkey,
    ) -> Result<()> {
        log_version();
        instructions::create_exchange_rate_snapshot(ctx, proposal)
    }

    /// Deposits token_a and/or token_b into their respective vaults
    pub fn deposit(
        ctx: Context<Deposit>,
        token_a_amount: u64,
        token_b_amount: u64,
    ) -> Result<()> {
        log_version();
        instructions::deposit(ctx, token_a_amount, token_b_amount)
    }

    /// Withdraws token_a and/or token_b from their respective vaults
    pub fn withdraw(
        ctx: Context<Withdraw>,
        token_a_amount: u64,
        token_b_amount: u64,
    ) -> Result<()> {
        log_version();
        instructions::withdraw(ctx, token_a_amount, token_b_amount)
    }

    /// Updates voter weight by combining token_a with converted token_b weight
    /// Uses existing exchange_rate_snapshot (created via create_exchange_rate_snapshot)
    /// Standalone plugin - does not compose with other plugins
    pub fn update_voter_weight_record(
        ctx: Context<UpdateVoterWeightRecord>,
        voter_weight_action: VoterWeightAction,
        proposal: Pubkey,
    ) -> Result<()> {
        log_version();
        instructions::update_voter_weight_record(ctx, voter_weight_action, proposal)
    }
}

fn log_version() {
    msg!("VERSION:{:?}", env!("CARGO_PKG_VERSION"));
}
