use crate::error::DualTokenVoterError;
use crate::orca::math::{increasing_price_order, sqrt_price_from_tick_index, U256Muldiv, Q64_RESOLUTION};
use anchor_lang::prelude::*;
use anchor_lang::Discriminator;
use std::cell::{Ref, RefMut};

use super::TICK_ARRAY_SIZE;

pub const MAX_TRADE_ENABLE_TIMESTAMP_DELTA: u64 = 60 * 60 * 72; // 72 hours

// This constant is used to scale the value of the volatility accumulator.
// The value of the volatility accumulator is decayed by the reduction factor and used as a new reference.
// However, if the volatility accumulator is simply the difference in tick_group_index, a value of 1 would quickly decay to 0.
// By scaling 1 to 10,000, for example, if the reduction factor is 0.5, the resulting value would be 5,000.
pub const VOLATILITY_ACCUMULATOR_SCALE_FACTOR: u16 = 10_000;

// The denominator of the reduction factor.
// When the reduction_factor is 5_000, the reduction factor functions as 0.5.
pub const REDUCTION_FACTOR_DENOMINATOR: u16 = 10_000;

// adaptive_fee_control_factor is used to map the square of the volatility accumulator to the fee rate.
// A larger value increases the fee rate quickly even for small volatility, while a smaller value increases the fee rate more gradually even for high volatility.
// When the adaptive_fee_control_factor is 1_000, the adaptive fee control factor functions as 0.01.
pub const ADAPTIVE_FEE_CONTROL_FACTOR_DENOMINATOR: u32 = 100_000;

// The time (in seconds) to forcibly reset the reference if it is not updated for a long time.
// A recovery measure against the act of intentionally repeating major swaps to keep the Adaptive Fee high (DoS).
pub const MAX_REFERENCE_AGE: u64 = 3_600; // 1 hour

#[zero_copy(unsafe)]
#[repr(C, packed)]
#[derive(Default, Debug, PartialEq, Eq)]
pub struct AdaptiveFeeConstants {
    // Period determine high frequency trading time window
    // The unit of time is "seconds" and is applied to the chain's block time
    pub filter_period: u16,
    // Period determine when the adaptive fee start decrease
    // The unit of time is "seconds" and is applied to the chain's block time
    pub decay_period: u16,
    // Adaptive fee rate decrement rate
    pub reduction_factor: u16,
    // Used to scale the adaptive fee component
    pub adaptive_fee_control_factor: u32,
    // Maximum number of ticks crossed can be accumulated
    // Used to cap adaptive fee rate
    pub max_volatility_accumulator: u32,
    // Tick group index is defined as floor(tick_index / tick_group_size)
    pub tick_group_size: u16,
    // Major swap threshold in tick
    pub major_swap_threshold_ticks: u16,
    // Reserved for future use
    pub reserved: [u8; 16],
}

impl AdaptiveFeeConstants {
    pub const LEN: usize = 2 + 2 + 2 + 4 + 4 + 2 + 2 + 16;

    #[allow(clippy::too_many_arguments)]
    pub fn validate_constants(
        tick_spacing: u16,
        filter_period: u16,
        decay_period: u16,
        reduction_factor: u16,
        adaptive_fee_control_factor: u32,
        max_volatility_accumulator: u32,
        tick_group_size: u16,
        major_swap_threshold_ticks: u16,
    ) -> bool {
        // filter_period validation
        // must be >= 1
        if filter_period == 0 {
            return false;
        }

        // decay_period validation
        // must be >= 1 and > filter_period
        if decay_period == 0 || decay_period <= filter_period {
            return false;
        }

        // adaptive_fee_control_factor validation
        // must be less than ADAPTIVE_FEE_CONTROL_FACTOR_DENOMINATOR
        if adaptive_fee_control_factor >= ADAPTIVE_FEE_CONTROL_FACTOR_DENOMINATOR {
            return false;
        }

        // max_volatility_accumulator validation
        // this constraint is to prevent overflow at FeeRateManager::compute_adaptive_fee_rate
        if u64::from(max_volatility_accumulator) * u64::from(tick_group_size) > u32::MAX as u64 {
            return false;
        }

        // reduction_factor validation
        if reduction_factor >= REDUCTION_FACTOR_DENOMINATOR {
            return false;
        }

        // tick_group_size validation
        if tick_group_size == 0
            || tick_group_size > tick_spacing
            || tick_spacing % tick_group_size != 0
        {
            return false;
        }

        // major_swap_threshold_ticks validation
        // there is no clear upper limit for major_swap_threshold_ticks, but as a safeguard, we set the limit to ticks in a TickArray
        let ticks_in_tick_array = tick_spacing as i32 * TICK_ARRAY_SIZE;
        if major_swap_threshold_ticks == 0
            || major_swap_threshold_ticks as i32 > ticks_in_tick_array
        {
            return false;
        }

        true
    }
}

#[zero_copy(unsafe)]
#[repr(C, packed)]
#[derive(Default, Debug, PartialEq, Eq)]
pub struct AdaptiveFeeVariables {
    // Last timestamp (block time) when volatility_reference and tick_group_index_reference were updated
    pub last_reference_update_timestamp: u64,
    // Last timestamp (block time) when major swap was executed
    pub last_major_swap_timestamp: u64,
    // Volatility reference is decayed volatility accumulator
    pub volatility_reference: u32,
    // Active tick group index of last swap
    pub tick_group_index_reference: i32,
    // Volatility accumulator measure the number of tick group crossed since reference tick group index (scaled)
    pub volatility_accumulator: u32,
    // Reserved for future use
    pub reserved: [u8; 16],
}

impl AdaptiveFeeVariables {
    pub const LEN: usize = 8 + 8 + 4 + 4 + 4 + 16;

    pub fn update_volatility_accumulator(
        &mut self,
        tick_group_index: i32,
        adaptive_fee_constants: &AdaptiveFeeConstants,
    ) -> Result<()> {
        let index_delta = (self.tick_group_index_reference - tick_group_index).unsigned_abs();
        let volatility_accumulator = u64::from(self.volatility_reference)
            + u64::from(index_delta) * u64::from(VOLATILITY_ACCUMULATOR_SCALE_FACTOR);

        self.volatility_accumulator = std::cmp::min(
            volatility_accumulator,
            u64::from(adaptive_fee_constants.max_volatility_accumulator),
        ) as u32;

        Ok(())
    }

    pub fn update_reference(
        &mut self,
        tick_group_index: i32,
        current_timestamp: u64,
        adaptive_fee_constants: &AdaptiveFeeConstants,
    ) -> Result<()> {
        let max_timestamp = self
            .last_reference_update_timestamp
            .max(self.last_major_swap_timestamp);
        if current_timestamp < max_timestamp {
            return Err(DualTokenVoterError::InvalidTimestamp.into());
        }

        let reference_age = current_timestamp - self.last_reference_update_timestamp;
        if reference_age > MAX_REFERENCE_AGE {
            // The references are too old, so reset them
            self.tick_group_index_reference = tick_group_index;
            self.volatility_reference = 0;
            self.last_reference_update_timestamp = current_timestamp;
            return Ok(());
        }

        let elapsed = current_timestamp - max_timestamp;
        if elapsed < adaptive_fee_constants.filter_period as u64 {
            // high frequency trade
            // no change
        } else if elapsed < adaptive_fee_constants.decay_period as u64 {
            // NOT high frequency trade
            self.tick_group_index_reference = tick_group_index;
            self.volatility_reference = (u64::from(self.volatility_accumulator)
                * u64::from(adaptive_fee_constants.reduction_factor)
                / u64::from(REDUCTION_FACTOR_DENOMINATOR))
                as u32;
            self.last_reference_update_timestamp = current_timestamp;
        } else {
            // Out of decay time window
            self.tick_group_index_reference = tick_group_index;
            self.volatility_reference = 0;
            self.last_reference_update_timestamp = current_timestamp;
        }

        Ok(())
    }

    pub fn update_major_swap_timestamp(
        &mut self,
        pre_sqrt_price: u128,
        post_sqrt_price: u128,
        current_timestamp: u64,
        adaptive_fee_constants: &AdaptiveFeeConstants,
    ) -> Result<()> {
        if Self::is_major_swap(
            pre_sqrt_price,
            post_sqrt_price,
            adaptive_fee_constants.major_swap_threshold_ticks,
        )? {
            self.last_major_swap_timestamp = current_timestamp;
        }
        Ok(())
    }

    // Determine whether the difference between pre_sqrt_price and post_sqrt_price is equivalent to major_swap_threshold_ticks or more
    // Note: The error of less than 0.00000003% due to integer arithmetic of sqrt_price is acceptable
    fn is_major_swap(
        pre_sqrt_price: u128,
        post_sqrt_price: u128,
        major_swap_threshold_ticks: u16,
    ) -> Result<bool> {
        let (smaller_sqrt_price, larger_sqrt_price) =
            increasing_price_order(pre_sqrt_price, post_sqrt_price);

        // major_swap_sqrt_price_target
        //   = smaller_sqrt_price * sqrt(pow(1.0001, major_swap_threshold_ticks))
        //   = smaller_sqrt_price * sqrt_price_from_tick_index(major_swap_threshold_ticks) >> Q64_RESOLUTION
        //
        // Note: The following two are theoretically equal, but there is an integer arithmetic error.
        //       However, the error impact is less than 0.00000003% in sqrt price (x64) and is small enough.
        //       - sqrt_price_from_tick_index(a) * sqrt_price_from_tick_index(b) >> Q64_RESOLUTION   (mathematically, sqrt(pow(1.0001, a)) * sqrt(pow(1.0001, b)) = sqrt(pow(1.0001, a + b)))
        //       - sqrt_price_from_tick_index(a + b)                                                 (mathematically, sqrt(pow(1.0001, a + b)))
        let major_swap_sqrt_price_factor =
            sqrt_price_from_tick_index(major_swap_threshold_ticks as i32);
        let major_swap_sqrt_price_target = U256Muldiv::new(0, smaller_sqrt_price)
            .mul(U256Muldiv::new(0, major_swap_sqrt_price_factor))
            .shift_right(Q64_RESOLUTION as u32)
            .try_into_u128()?;

        Ok(larger_sqrt_price >= major_swap_sqrt_price_target)
    }
}

#[derive(Debug, Default, Clone)]
pub struct AdaptiveFeeInfo {
    pub constants: AdaptiveFeeConstants,
    pub variables: AdaptiveFeeVariables,
}

#[account(zero_copy(unsafe))]
#[repr(C, packed)]
#[derive(Debug)]
pub struct Oracle {
    pub whirlpool: Pubkey,
    pub trade_enable_timestamp: u64,
    pub adaptive_fee_constants: AdaptiveFeeConstants,
    pub adaptive_fee_variables: AdaptiveFeeVariables,
    // Reserved for future use
    pub reserved: [u8; 128],
}

impl Default for Oracle {
    fn default() -> Self {
        Self {
            whirlpool: Pubkey::default(),
            trade_enable_timestamp: 0,
            adaptive_fee_constants: AdaptiveFeeConstants::default(),
            adaptive_fee_variables: AdaptiveFeeVariables::default(),
            reserved: [0u8; 128],
        }
    }
}

impl Oracle {
    pub const LEN: usize = 8 + 32 + 8 + AdaptiveFeeConstants::LEN + AdaptiveFeeVariables::LEN + 128;

    #[allow(clippy::too_many_arguments)]
    pub fn initialize(
        &mut self,
        whirlpool: Pubkey,
        trade_enable_timestamp: Option<u64>,
        tick_spacing: u16,
        filter_period: u16,
        decay_period: u16,
        reduction_factor: u16,
        adaptive_fee_control_factor: u32,
        max_volatility_accumulator: u32,
        tick_group_size: u16,
        major_swap_threshold_ticks: u16,
    ) -> Result<()> {
        self.whirlpool = whirlpool;
        self.trade_enable_timestamp = trade_enable_timestamp.unwrap_or(0);

        let constants = AdaptiveFeeConstants {
            filter_period,
            decay_period,
            reduction_factor,
            adaptive_fee_control_factor,
            max_volatility_accumulator,
            tick_group_size,
            major_swap_threshold_ticks,
            reserved: [0u8; 16],
        };

        self.initialize_adaptive_fee_constants(constants, tick_spacing)?;
        self.reset_adaptive_fee_variables();

        Ok(())
    }

    pub fn initialize_adaptive_fee_constants(
        &mut self,
        constants: AdaptiveFeeConstants,
        tick_spacing: u16,
    ) -> Result<()> {
        if !AdaptiveFeeConstants::validate_constants(
            tick_spacing,
            constants.filter_period,
            constants.decay_period,
            constants.reduction_factor,
            constants.adaptive_fee_control_factor,
            constants.max_volatility_accumulator,
            constants.tick_group_size,
            constants.major_swap_threshold_ticks,
        ) {
            return Err(DualTokenVoterError::InvalidAdaptiveFeeConstants.into());
        }

        self.adaptive_fee_constants = constants;

        Ok(())
    }

    pub fn update_adaptive_fee_variables(&mut self, variables: AdaptiveFeeVariables) {
        self.adaptive_fee_variables = variables;
    }

    fn reset_adaptive_fee_variables(&mut self) {
        self.adaptive_fee_variables = AdaptiveFeeVariables::default();
    }
}

pub struct OracleAccessor<'info> {
    oracle_account_info: AccountInfo<'info>,
    oracle_account_initialized: bool,
}

impl<'info> OracleAccessor<'info> {
    pub fn new(
        whirlpool_key: &Pubkey,
        oracle_account_info: AccountInfo<'info>,
    ) -> Result<Self> {
        let oracle_account_initialized =
            Self::is_oracle_account_initialized(&oracle_account_info, *whirlpool_key)?;
        Ok(Self {
            oracle_account_info,
            oracle_account_initialized,
        })
    }

    pub fn is_trade_enabled(&self, current_timestamp: u64) -> Result<bool> {
        if !self.oracle_account_initialized {
            return Ok(true);
        }

        let oracle = self.load()?;
        Ok(oracle.trade_enable_timestamp <= current_timestamp)
    }

    pub fn get_adaptive_fee_info(&self) -> Result<Option<AdaptiveFeeInfo>> {
        if !self.oracle_account_initialized {
            return Ok(None);
        }

        let oracle = self.load()?;
        Ok(Some(AdaptiveFeeInfo {
            constants: oracle.adaptive_fee_constants,
            variables: oracle.adaptive_fee_variables,
        }))
    }

    pub fn update_adaptive_fee_variables(
        &self,
        adaptive_fee_info: &Option<AdaptiveFeeInfo>,
    ) -> Result<()> {
        // If the Oracle account is not initialized, load_mut access will be skipped.
        // In other words, no need for writable flag on the Oracle account if it is not initialized.

        match (self.oracle_account_initialized, adaptive_fee_info) {
            // Oracle account has been initialized and adaptive fee info is provided
            (true, Some(adaptive_fee_info)) => {
                let mut oracle = self.load_mut()?;
                oracle.update_adaptive_fee_variables(adaptive_fee_info.variables);
                Ok(())
            }
            // Oracle account has not been initialized and adaptive fee info is not provided
            (false, None) => Ok(()),
            _ => unreachable!(),
        }
    }

    fn is_oracle_account_initialized(
        oracle_account_info: &AccountInfo<'info>,
        whirlpool: Pubkey,
    ) -> Result<bool> {
        // following process is ported from anchor-lang's AccountLoader::try_from and AccountLoader::load_mut
        // AccountLoader can handle initialized account and partially initialized (owner program changed) account only.
        // So we need to handle uninitialized account manually.

        // Note: intentionally do not check if the account is writable here, defer the evaluation until load_mut is called

        // uninitialized account (owned by system program and its data size is zero)
        if oracle_account_info.owner == &System::id() && oracle_account_info.data_is_empty() {
            // oracle is not initialized
            return Ok(false);
        }

        // owner program check
        if oracle_account_info.owner != &Oracle::owner() {
            return Err(
                Error::from(anchor_lang::error::ErrorCode::AccountOwnedByWrongProgram)
                    .with_pubkeys((*oracle_account_info.owner, Oracle::owner())),
            );
        }

        let data = oracle_account_info.try_borrow_data()?;
        if data.len() < Oracle::DISCRIMINATOR.len() {
            return Err(anchor_lang::error::ErrorCode::AccountDiscriminatorNotFound.into());
        }

        if &data[..8] != Oracle::DISCRIMINATOR {
            return Err(anchor_lang::error::ErrorCode::AccountDiscriminatorMismatch.into());
        }

        // whirlpool check
        let oracle_ref: Ref<Oracle> = Ref::map(data, |data| {
            bytemuck::from_bytes(&data[8..std::mem::size_of::<Oracle>() + 8])
        });
        if oracle_ref.whirlpool != whirlpool {
            // Just for safety: Oracle address is derived from Whirlpool address, so this should not happen.
            unreachable!();
        }

        Ok(true)
    }

    fn load(&self) -> Result<Ref<'_, Oracle>> {
        // is_oracle_account_initialized already checked if the account is initialized

        let data = self.oracle_account_info.try_borrow_data()?;
        let oracle_ref: Ref<Oracle> = Ref::map(data, |data| {
            bytemuck::from_bytes(&data[8..std::mem::size_of::<Oracle>() + 8])
        });

        Ok(oracle_ref)
    }

    fn load_mut(&self) -> Result<RefMut<'_, Oracle>> {
        // is_oracle_account_initialized already checked if the account is initialized

        use std::ops::DerefMut;

        // account must be writable
        if !self.oracle_account_info.is_writable {
            return Err(anchor_lang::error::ErrorCode::AccountNotMutable.into());
        }

        let data = self.oracle_account_info.try_borrow_mut_data()?;
        let oracle_refmut: RefMut<Oracle> = RefMut::map(data, |data| {
            bytemuck::from_bytes_mut(&mut data.deref_mut()[8..std::mem::size_of::<Oracle>() + 8])
        });

        Ok(oracle_refmut)
    }
}
