use std::convert::TryInto;

use crate::error::DualTokenVoterError;
use crate::orca::math::*;

pub const NO_EXPLICIT_SQRT_PRICE_LIMIT: u128 = 0u128;

#[derive(PartialEq, Debug)]
pub struct SwapStepComputation {
    pub amount_in: u64,
    pub amount_out: u64,
    pub next_price: u128,
    pub fee_amount: u64,
}

pub fn compute_swap(
    amount_remaining: u64,
    fee_rate: u32,
    liquidity: u128,
    sqrt_price_current: u128,
    sqrt_price_target: u128,
    amount_specified_is_input: bool,
    a_to_b: bool,
) -> Result<SwapStepComputation, DualTokenVoterError> {
    // Since SplashPool (aka FullRange only pool) has only 2 initialized ticks at both ends,
    // the possibility of exceeding u64 when calculating "delta amount" is higher than concentrated pools.
    // This problem occurs with ExactIn.
    // The reason is that in ExactOut, "fixed delta" never exceeds the amount of tokens present in the pool and is clearly within the u64 range.
    // On the other hand, for ExactIn, "fixed delta" may exceed u64 because it calculates the amount of tokens needed to move the price to the end.
    // However, the primary purpose of initial calculation of "fixed delta" is to determine whether or not the iteration is "max swap" or not.
    // So the info that "the amount of tokens required exceeds the u64 range" is sufficient to determine that the iteration is NOT "max swap".
    //
    // delta <= u64::MAX: AmountDeltaU64::Valid
    // delta >  u64::MAX: AmountDeltaU64::ExceedsMax
    let initial_amount_fixed_delta = try_get_amount_fixed_delta(
        sqrt_price_current,
        sqrt_price_target,
        liquidity,
        amount_specified_is_input,
        a_to_b,
    )?;

    let mut amount_calc = amount_remaining;
    if amount_specified_is_input {
        amount_calc = checked_mul_div(
            amount_remaining as u128,
            FEE_RATE_MUL_VALUE - fee_rate as u128,
            FEE_RATE_MUL_VALUE,
        )?
        .try_into()
        .map_err(|_| DualTokenVoterError::TokenMaxExceeded)?;
    }

    let next_sqrt_price = if initial_amount_fixed_delta.lte(amount_calc) {
        sqrt_price_target
    } else {
        get_next_sqrt_price(
            sqrt_price_current,
            liquidity,
            amount_calc,
            amount_specified_is_input,
            a_to_b,
        )?
    };

    let is_max_swap = next_sqrt_price == sqrt_price_target;

    let amount_unfixed_delta = get_amount_unfixed_delta(
        sqrt_price_current,
        next_sqrt_price,
        liquidity,
        amount_specified_is_input,
        a_to_b,
    )?;

    // If the swap is not at the max, we need to readjust the amount of the fixed token we are using
    let amount_fixed_delta = if !is_max_swap || initial_amount_fixed_delta.exceeds_max() {
        // next_sqrt_price is calculated by get_next_sqrt_price and the result will be in the u64 range.
        get_amount_fixed_delta(
            sqrt_price_current,
            next_sqrt_price,
            liquidity,
            amount_specified_is_input,
            a_to_b,
        )?
    } else {
        // the result will be in the u64 range.
        initial_amount_fixed_delta.value()
    };

    let (amount_in, mut amount_out) = if amount_specified_is_input {
        (amount_fixed_delta, amount_unfixed_delta)
    } else {
        (amount_unfixed_delta, amount_fixed_delta)
    };

    // Cap output amount if using output
    if !amount_specified_is_input && amount_out > amount_remaining {
        amount_out = amount_remaining;
    }

    let fee_amount = if amount_specified_is_input && !is_max_swap {
        amount_remaining - amount_in
    } else {
        checked_mul_div_round_up(
            amount_in as u128,
            fee_rate as u128,
            FEE_RATE_MUL_VALUE - fee_rate as u128,
        )?
        .try_into()
        .map_err(|_| DualTokenVoterError::TokenMaxExceeded)?
    };

    Ok(SwapStepComputation {
        amount_in,
        amount_out,
        next_price: next_sqrt_price,
        fee_amount,
    })
}

fn get_amount_fixed_delta(
    sqrt_price_current: u128,
    sqrt_price_target: u128,
    liquidity: u128,
    amount_specified_is_input: bool,
    a_to_b: bool,
) -> Result<u64, DualTokenVoterError> {
    if a_to_b == amount_specified_is_input {
        get_amount_delta_a(
            sqrt_price_current,
            sqrt_price_target,
            liquidity,
            amount_specified_is_input,
        )
    } else {
        get_amount_delta_b(
            sqrt_price_current,
            sqrt_price_target,
            liquidity,
            amount_specified_is_input,
        )
    }
}

fn try_get_amount_fixed_delta(
    sqrt_price_current: u128,
    sqrt_price_target: u128,
    liquidity: u128,
    amount_specified_is_input: bool,
    a_to_b: bool,
) -> Result<AmountDeltaU64, DualTokenVoterError> {
    if a_to_b == amount_specified_is_input {
        try_get_amount_delta_a(
            sqrt_price_current,
            sqrt_price_target,
            liquidity,
            amount_specified_is_input,
        )
    } else {
        try_get_amount_delta_b(
            sqrt_price_current,
            sqrt_price_target,
            liquidity,
            amount_specified_is_input,
        )
    }
}

fn get_amount_unfixed_delta(
    sqrt_price_current: u128,
    sqrt_price_target: u128,
    liquidity: u128,
    amount_specified_is_input: bool,
    a_to_b: bool,
) -> Result<u64, DualTokenVoterError> {
    if a_to_b == amount_specified_is_input {
        get_amount_delta_b(
            sqrt_price_current,
            sqrt_price_target,
            liquidity,
            !amount_specified_is_input,
        )
    } else {
        get_amount_delta_a(
            sqrt_price_current,
            sqrt_price_target,
            liquidity,
            !amount_specified_is_input,
        )
    }
}
