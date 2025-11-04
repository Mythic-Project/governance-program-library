use super::{
    manager::swap,
    state::Whirlpool,
    util::SparseSwapTickSequenceBuilder,
    OracleAccessor, NO_EXPLICIT_SQRT_PRICE_LIMIT, to_timestamp_u64,
    MIN_TICK_INDEX, MAX_TICK_INDEX,
};
use crate::{error::*, orca::math::{MIN_SQRT_PRICE_X64, MAX_SQRT_PRICE_X64, MAX_FEE_RATE, MAX_PROTOCOL_FEE_RATE}};
use anchor_lang::prelude::*;
use std::mem::size_of;

/// Orca Whirlpool Program ID (mainnet)
/// TODO: Update for devnet/localnet if needed
pub const ORCA_WHIRLPOOL_PROGRAM_ID: Pubkey = pubkey!("whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc");

/// Orca Whirlpool Account Discriminator
/// This is the actual discriminator used by Orca's deployed program
/// Calculated as: sha256("account:Whirlpool")[:8]
/// We hardcode this to ensure we're reading the correct account type
pub const ORCA_WHIRLPOOL_DISCRIMINATOR: [u8; 8] = [63, 149, 209, 12, 225, 128, 99, 9];

/// Minimum Whirlpool account size
/// Must be at least: discriminator (8) + Whirlpool struct size
const MIN_WHIRLPOOL_ACCOUNT_SIZE: usize = 8 + size_of::<Whirlpool>();

/// Test swap amount for calculating exchange rate
/// Using 1 token (assuming 9 decimals) = 1,000,000,000
const TEST_SWAP_AMOUNT: u64 = 1_000_000_000;

/// Minimum exchange rate (0.000001 = 1e3 in 9-decimal representation)
const MIN_EXCHANGE_RATE: u128 = 1_000;

/// Maximum exchange rate (1,000,000 = 1e15 in 9-decimal representation)
const MAX_EXCHANGE_RATE: u128 = 1_000_000_000_000_000;

// MAX_FEE_RATE and MAX_PROTOCOL_FEE_RATE are imported from orca::math module

/// Calculate exchange rate from Whirlpool by simulating a test swap
///
/// This replicates the exact Orca swap instruction logic to get accurate
/// exchange rates including fees and price impact.
///
/// # Security
/// This function performs extensive validation to prevent attacks:
/// - Verifies all accounts are owned by Orca Whirlpool program
/// - Manually validates account discriminators (hardcoded expected values)
/// - Uses zero-copy access for Whirlpool (no deserialization overhead)
/// - Checks whirlpool state integrity (sqrt_price, tick_index, liquidity)
/// - Validates token mints match expected values
/// - Ensures tick arrays belong to the whirlpool
/// - Validates oracle is not stale
/// - Bounds-checks all numeric values
/// - Ensures read-only operation (no state persisted)
///
/// # Arguments
/// * `whirlpool_account` - Whirlpool account (must be owned by Orca program)
/// * `tick_array_accounts` - 3 tick array accounts (must belong to whirlpool)
/// * `oracle_account` - Oracle account (must belong to whirlpool)
/// * `expected_token_a` - Expected token A mint (validated against whirlpool)
/// * `expected_token_b` - Expected token B mint (validated against whirlpool)
///
/// # Returns
/// * Exchange rate as u64 (how much token_b per token_a, scaled by 1e9)
///
/// # Errors
/// * `InvalidWhirlpool` - Account not owned by Orca program or wrong discriminator
/// * `InvalidWhirlpoolMint` - Token mints don't match expected values
/// * `NoLiquidity` - Whirlpool has zero liquidity
/// * `SqrtPriceOutOfBounds` - Price is outside valid range
/// * `InvalidTickArraySequence` - Tick arrays invalid or don't belong to whirlpool
/// * `TradeNotEnabled` - Oracle indicates trading is disabled
/// * `InvalidExchangeRate` - Calculated rate is outside bounds
pub fn calculate_exchange_rate_from_whirlpool<'info>(
    whirlpool_account: &AccountInfo<'info>,
    tick_array_0: &AccountInfo<'info>,
    tick_array_1: &AccountInfo<'info>,
    tick_array_2: &AccountInfo<'info>,
    oracle_account: &AccountInfo<'info>,
    expected_token_a: &Pubkey,
    expected_token_b: &Pubkey,
) -> Result<u64> {
    // ============================================
    // SECURITY VALIDATION: Program Ownership
    // ============================================

    // Validate whirlpool is owned by Orca Whirlpool program
    require_keys_eq!(
        *whirlpool_account.owner,
        ORCA_WHIRLPOOL_PROGRAM_ID,
        DualTokenVoterError::InvalidWhirlpool
    );

    // Validate all tick arrays are owned by Orca program
    require_keys_eq!(
        *tick_array_0.owner,
        ORCA_WHIRLPOOL_PROGRAM_ID,
        DualTokenVoterError::InvalidTickArraySequence
    );
    require_keys_eq!(
        *tick_array_1.owner,
        ORCA_WHIRLPOOL_PROGRAM_ID,
        DualTokenVoterError::InvalidTickArraySequence
    );
    require_keys_eq!(
        *tick_array_2.owner,
        ORCA_WHIRLPOOL_PROGRAM_ID,
        DualTokenVoterError::InvalidTickArraySequence
    );

    // Validate oracle is owned by Orca program
    require_keys_eq!(
        *oracle_account.owner,
        ORCA_WHIRLPOOL_PROGRAM_ID,
        DualTokenVoterError::TradeNotEnabled
    );

    // ============================================
    // SECURITY VALIDATION: Discriminator (Manual)
    // ============================================

    // Get account data
    let whirlpool_data = whirlpool_account.try_borrow_data()
        .map_err(|_| error!(DualTokenVoterError::InvalidWhirlpool))?;

    // Validate account size
    require!(
        whirlpool_data.len() >= MIN_WHIRLPOOL_ACCOUNT_SIZE,
        DualTokenVoterError::InvalidWhirlpool
    );

    // CRITICAL: Manually validate discriminator matches expected value
    // This ensures we're reading an actual Orca Whirlpool account
    // and not some fake account with similar structure
    let account_discriminator = &whirlpool_data[0..8];
    require!(
        account_discriminator == ORCA_WHIRLPOOL_DISCRIMINATOR,
        DualTokenVoterError::InvalidWhirlpool
    );

    // ============================================
    // ZERO-COPY ACCESS: Load Whirlpool
    // ============================================

    // Use zero-copy access to read Whirlpool struct
    // This is the correct way to access #[account(zero_copy)] accounts
    // No deserialization, just direct memory mapping
    let whirlpool_bytes = &whirlpool_data[8..]; // Skip discriminator

    // Cast bytes to Whirlpool struct using unsafe reference cast
    // SAFETY: We've validated:
    // 1. Account is owned by Orca program (can't be modified by attacker)
    // 2. Discriminator matches (correct account type)
    // 3. Size is sufficient (no buffer overflow)
    // 4. Whirlpool is #[zero_copy(unsafe)] which guarantees compatible memory layout
    let whirlpool: &Whirlpool = unsafe {
        &*(whirlpool_bytes.as_ptr() as *const Whirlpool)
    };

    // ============================================
    // SECURITY VALIDATION: Token Mints
    // ============================================

    // Validate token mints match expected values
    // Prevents attacker from passing wrong pool
    require_keys_eq!(
        whirlpool.token_mint_a,
        *expected_token_a,
        DualTokenVoterError::InvalidWhirlpoolMint
    );
    require_keys_eq!(
        whirlpool.token_mint_b,
        *expected_token_b,
        DualTokenVoterError::InvalidWhirlpoolMint
    );

    // ============================================
    // SECURITY VALIDATION: Whirlpool State Integrity
    // ============================================

    // Validate liquidity is non-zero
    // Zero liquidity could cause division errors or indicate pool is inactive
    require!(
        whirlpool.liquidity > 0,
        DualTokenVoterError::NoLiquidity
    );

    // Validate sqrt_price is within valid bounds
    // Prevents overflow/underflow in calculations
    require!(
        whirlpool.sqrt_price >= MIN_SQRT_PRICE_X64
            && whirlpool.sqrt_price <= MAX_SQRT_PRICE_X64,
        DualTokenVoterError::SqrtPriceOutOfBounds
    );

    // Validate tick_current_index is within valid bounds
    // Prevents out-of-bounds array access in tick array lookups
    require!(
        whirlpool.tick_current_index >= MIN_TICK_INDEX
            && whirlpool.tick_current_index <= MAX_TICK_INDEX,
        DualTokenVoterError::InvalidTickArraySequence
    );

    // Validate fee rates are within bounds
    // Prevents integer overflow and ensures reasonable fees
    require!(
        whirlpool.fee_rate <= MAX_FEE_RATE,
        DualTokenVoterError::InvalidAdaptiveFeeConstants
    );
    require!(
        whirlpool.protocol_fee_rate <= MAX_PROTOCOL_FEE_RATE,
        DualTokenVoterError::InvalidAdaptiveFeeConstants
    );

    // ============================================
    // SECURITY VALIDATION: Timestamp
    // ============================================

    // Get current timestamp and validate it's not negative
    let clock = Clock::get()?;
    let timestamp = to_timestamp_u64(clock.unix_timestamp)?;

    // ============================================
    // SWAP SIMULATION: Build Tick Sequence
    // ============================================

    // Build SparseSwapTickSequence (EXACTLY like Orca swap instruction)
    // This validates:
    // 1. Tick arrays have correct discriminator (via load_tick_array_mut)
    // 2. Tick arrays belong to this whirlpool (checked inside try_build)
    // 3. Tick arrays form a valid sequence covering current price
    let swap_tick_sequence_builder = SparseSwapTickSequenceBuilder::new(
        vec![
            tick_array_0.clone(),
            tick_array_1.clone(),
            tick_array_2.clone(),
        ],
        None, // No supplemental tick arrays
    );

    let a_to_b = true; // Swap token A for token B
    let whirlpool_key = whirlpool_account.key();
    let mut swap_tick_sequence = swap_tick_sequence_builder.try_build(&whirlpool, &whirlpool_key, a_to_b)?;

    // ============================================
    // SECURITY VALIDATION: Oracle
    // ============================================

    // Create OracleAccessor which validates:
    // 1. Oracle account size
    // 2. Oracle discriminator (if initialized)
    // 3. Oracle belongs to this whirlpool
    let oracle_accessor = OracleAccessor::new(&whirlpool_key, oracle_account.clone())?;

    // Check if trading is enabled
    // This validates oracle timestamp is not too old
    if !oracle_accessor.is_trade_enabled(timestamp)? {
        return Err(DualTokenVoterError::TradeNotEnabled.into());
    }

    // Get adaptive fee info (if available)
    let adaptive_fee_info = oracle_accessor.get_adaptive_fee_info()?;

    // ============================================
    // SWAP SIMULATION: Execute Swap
    // ============================================

    // Call swap() function (EXACTLY like Orca)
    // IMPORTANT: This modifies in-memory state but does NOT persist changes
    // The swap() function returns PostSwapUpdate but never writes to accounts
    // When this instruction ends, all modifications are discarded (read-only)
    let swap_update = swap(
        &whirlpool,
        &mut swap_tick_sequence,
        TEST_SWAP_AMOUNT,
        NO_EXPLICIT_SQRT_PRICE_LIMIT, // No price limit
        true,                          // amount_specified_is_input
        a_to_b,                        // Swap A -> B
        timestamp,
        &adaptive_fee_info,
    )?;

    // ============================================
    // SECURITY VALIDATION: Swap Result
    // ============================================

    // Extract output amount
    let output_amount = swap_update.amount_b; // Since a_to_b = true

    // Validate output amount is non-zero
    // Zero output could indicate:
    // 1. No liquidity in range
    // 2. Price impact too high
    // 3. Manipulated pool state
    require!(
        output_amount > 0,
        DualTokenVoterError::ZeroTradableAmount
    );

    // ============================================
    // EXCHANGE RATE CALCULATION
    // ============================================

    // Calculate exchange rate: (output / input) * 1e9
    // This gives us "how much token_b per token_a" scaled by 1e9
    let raw_rate = (output_amount as u128)
        .checked_mul(1_000_000_000)
        .ok_or(DualTokenVoterError::MathOverflow)?
        .checked_div(TEST_SWAP_AMOUNT as u128)
        .ok_or(DualTokenVoterError::DivisionError)?;

    // ============================================
    // SECURITY VALIDATION: Exchange Rate Bounds
    // ============================================

    // Validate rate is within reasonable bounds
    // MIN: 0.000001 (prevents near-zero rates that could be manipulation)
    // MAX: 1,000,000 (prevents overflow in voting weight calculations)
    require!(
        raw_rate >= MIN_EXCHANGE_RATE && raw_rate <= MAX_EXCHANGE_RATE,
        DualTokenVoterError::InvalidExchangeRate
    );

    // ============================================
    // SANITY CHECK: Rate vs Sqrt Price
    // ============================================

    // The exchange rate should be reasonably consistent with sqrt_price
    // sqrt_price = sqrt(price_b/price_a) in Q64.64 format
    // price_b/price_a = (sqrt_price / 2^64)^2
    // We can do a rough sanity check here to detect gross manipulation

    // Calculate rough rate from sqrt_price for comparison
    let sqrt_price_f64 = (whirlpool.sqrt_price as f64) / (1u128 << 64) as f64;
    let sqrt_price_rate = (sqrt_price_f64 * sqrt_price_f64 * 1_000_000_000.0) as u128;

    // Allow up to 20% deviation (accounts for fees, slippage, rounding)
    // If deviation is more than 20%, likely manipulation or wrong pool
    let rate_diff = if raw_rate > sqrt_price_rate {
        raw_rate - sqrt_price_rate
    } else {
        sqrt_price_rate - raw_rate
    };

    let max_deviation = sqrt_price_rate / 5; // 20%
    require!(
        rate_diff <= max_deviation,
        DualTokenVoterError::InvalidExchangeRate
    );

    Ok(raw_rate as u64)
}