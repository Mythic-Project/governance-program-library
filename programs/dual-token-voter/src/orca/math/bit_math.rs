use crate::error::DualTokenVoterError;

use super::U256Muldiv;

pub const Q64_RESOLUTION: u8 = 64;
pub const Q64_MASK: u128 = 0xFFFF_FFFF_FFFF_FFFF;
pub const TO_Q64: u128 = 1u128 << Q64_RESOLUTION;

pub fn checked_mul_div(n0: u128, n1: u128, d: u128) -> Result<u128, DualTokenVoterError> {
    checked_mul_div_round_up_if(n0, n1, d, false)
}

pub fn checked_mul_div_round_up(n0: u128, n1: u128, d: u128) -> Result<u128, DualTokenVoterError> {
    checked_mul_div_round_up_if(n0, n1, d, true)
}

pub fn checked_mul_div_round_up_if(
    n0: u128,
    n1: u128,
    d: u128,
    round_up: bool,
) -> Result<u128, DualTokenVoterError> {
    if d == 0 {
        return Err(DualTokenVoterError::DivideByZero);
    }

    let p = n0.checked_mul(n1).ok_or(DualTokenVoterError::MulDivOverflow)?;
    let n = p / d;

    Ok(if round_up && p % d > 0 { n + 1 } else { n })
}

pub fn checked_mul_shift_right(n0: u128, n1: u128) -> Result<u64, DualTokenVoterError> {
    checked_mul_shift_right_round_up_if(n0, n1, false)
}

/// Multiplies an integer u128 and a Q64.64 fixed point number.
/// Returns a product represented as a u64 integer.
pub fn checked_mul_shift_right_round_up_if(
    n0: u128,
    n1: u128,
    round_up: bool,
) -> Result<u64, DualTokenVoterError> {
    // customized this function is used in try_get_amount_delta_b (token_math.rs)

    if n0 == 0 || n1 == 0 {
        return Ok(0);
    }

    let p = n0
        .checked_mul(n1)
        .ok_or(DualTokenVoterError::MultiplicationShiftRightOverflow)?;

    let result = (p >> Q64_RESOLUTION) as u64;

    let should_round = round_up && (p & Q64_MASK > 0);
    if should_round && result == u64::MAX {
        return Err(DualTokenVoterError::MultiplicationOverflow);
    }

    Ok(if should_round { result + 1 } else { result })
}

pub fn div_round_up(n: u128, d: u128) -> Result<u128, DualTokenVoterError> {
    div_round_up_if(n, d, true)
}

pub fn div_round_up_if(n: u128, d: u128, round_up: bool) -> Result<u128, DualTokenVoterError> {
    if d == 0 {
        return Err(DualTokenVoterError::DivideByZero);
    }

    let q = n / d;

    Ok(if round_up && n % d > 0 { q + 1 } else { q })
}

pub fn div_round_up_if_u256(
    n: U256Muldiv,
    d: U256Muldiv,
    round_up: bool,
) -> Result<u128, DualTokenVoterError> {
    let (quotient, remainder) = n.div(d, round_up);

    let result = if round_up && !remainder.is_zero() {
        quotient.add(U256Muldiv::new(0, 1))
    } else {
        quotient
    };

    result.try_into_u128()
}
