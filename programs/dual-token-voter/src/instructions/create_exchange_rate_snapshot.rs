use {
    crate::{error::*, orca::exchange_rate_calculator::*, state::*},
    anchor_lang::prelude::*,
    anchor_spl::token_interface::Mint,
};

/// Creates an exchange rate snapshot for a proposal by fetching rate from Orca
#[derive(Accounts)]
#[instruction(proposal: Pubkey)]
pub struct CreateExchangeRateSnapshot<'info> {
    pub registrar: Account<'info, Registrar>,

    /// Exchange rate snapshot for this proposal
    #[account(
        init,
        seeds = [b"exchange-snapshot", registrar.key().as_ref(), proposal.as_ref()],
        bump,
        payer = payer,
        space = 8 + std::mem::size_of::<ExchangeRateSnapshot>()
    )]
    pub exchange_rate_snapshot: Account<'info, ExchangeRateSnapshot>,

    /// Orca Whirlpool account
    /// NOTE: Must be mut because swap() modifies tick arrays internally
    #[account(mut)]
    /// CHECK: Validated by calculate_exchange_rate_from_whirlpool
    pub whirlpool: UncheckedAccount<'info>,

    /// Orca Tick Array 0
    #[account(mut)]
    /// CHECK: Validated by SparseSwapTickSequenceBuilder
    pub tick_array_0: UncheckedAccount<'info>,

    /// Orca Tick Array 1
    #[account(mut)]
    /// CHECK: Validated by SparseSwapTickSequenceBuilder
    pub tick_array_1: UncheckedAccount<'info>,

    /// Orca Tick Array 2
    #[account(mut)]
    /// CHECK: Validated by SparseSwapTickSequenceBuilder
    pub tick_array_2: UncheckedAccount<'info>,

    /// Orca Oracle account (for adaptive fees)
    /// CHECK: Validated by OracleAccessor
    pub oracle: UncheckedAccount<'info>,

    /// Token A mint (for validation)
    pub token_a_mint: Box<InterfaceAccount<'info, Mint>>,

    /// Token B mint (for validation)
    pub token_b_mint: Box<InterfaceAccount<'info, Mint>>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
}

/// Creates an exchange rate snapshot for a proposal
/// Fetches the current exchange rate from Orca Whirlpool by simulating a swap
pub fn create_exchange_rate_snapshot(
    ctx: Context<CreateExchangeRateSnapshot>,
    proposal: Pubkey,
) -> Result<()> {
    let clock = Clock::get()?;
    let registrar = &ctx.accounts.registrar;
    let snapshot = &mut ctx.accounts.exchange_rate_snapshot;

    // Validate mints match registrar
    require_keys_eq!(
        ctx.accounts.token_a_mint.key(),
        registrar.token_a_mint,
        DualTokenVoterError::InvalidMint
    );
    require_keys_eq!(
        ctx.accounts.token_b_mint.key(),
        registrar.token_b_mint,
        DualTokenVoterError::InvalidMint
    );

    // Calculate exchange rate using EXACT Orca swap logic
    let exchange_rate = calculate_exchange_rate_from_whirlpool(
        &ctx.accounts.whirlpool.to_account_info(),
        &ctx.accounts.tick_array_0.to_account_info(),
        &ctx.accounts.tick_array_1.to_account_info(),
        &ctx.accounts.tick_array_2.to_account_info(),
        &ctx.accounts.oracle.to_account_info(),
        &registrar.token_a_mint,
        &registrar.token_b_mint,
    )?;

    // Initialize snapshot
    snapshot.registrar = registrar.key();
    snapshot.proposal = proposal;
    snapshot.token_b_exchange_rate = exchange_rate;
    snapshot.snapshot_timestamp = clock.unix_timestamp;
    snapshot.snapshot_slot = clock.slot;
    snapshot.bump = ctx.bumps.exchange_rate_snapshot;

    Ok(())
}
