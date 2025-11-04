use crate::error::DualTokenVoterError;

// Adds a signed liquidity delta to a given integer liquidity amount.
// Errors on overflow or underflow.
pub fn add_liquidity_delta(liquidity: u128, delta: i128) -> Result<u128, DualTokenVoterError> {
    if delta == 0 {
        return Ok(liquidity);
    }
    if delta > 0 {
        liquidity
            .checked_add(delta as u128)
            .ok_or(DualTokenVoterError::LiquidityOverflow)
    } else {
        liquidity
            .checked_sub(delta.unsigned_abs())
            .ok_or(DualTokenVoterError::LiquidityUnderflow)
    }
}

// Converts an unsigned liquidity amount to a signed liquidity delta
pub fn convert_to_liquidity_delta(
    liquidity_amount: u128,
    positive: bool,
) -> Result<i128, DualTokenVoterError> {
    if liquidity_amount > i128::MAX as u128 {
        // The liquidity_amount is converted to a liquidity_delta that is represented as an i128
        // By doing this conversion we lose the most significant bit in the u128
        // Here we enforce a max value of i128::MAX on the u128 to prevent loss of data.
        return Err(DualTokenVoterError::LiquidityTooHigh);
    }
    Ok(if positive {
        liquidity_amount as i128
    } else {
        -(liquidity_amount as i128)
    })
}
