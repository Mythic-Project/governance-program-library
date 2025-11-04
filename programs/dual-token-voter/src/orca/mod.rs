pub mod math;
pub mod state;
pub mod util;
pub mod manager;
pub mod exchange_rate_calculator;

pub use math::*;
pub use state::*;
pub use util::*;
pub use manager::*;
pub use exchange_rate_calculator::*;

use crate::error::DualTokenVoterError;
use anchor_lang::prelude::*;

// Constants
pub const NO_EXPLICIT_SQRT_PRICE_LIMIT: u128 = 0;

// Helper for timestamp conversion
pub fn to_timestamp_u64(timestamp_i64: i64) -> Result<u64> {
    if timestamp_i64 < 0 {
        return Err(DualTokenVoterError::InvalidTimestamp.into());
    }
    Ok(timestamp_i64 as u64)
}
