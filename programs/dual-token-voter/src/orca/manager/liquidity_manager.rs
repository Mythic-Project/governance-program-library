use super::{
    position_manager::next_position_modify_liquidity_update,
    tick_array_manager::{calculate_modify_tick_array, TickArrayUpdate},
    tick_manager::{
        next_fee_growths_inside, next_reward_growths_inside, next_tick_modify_liquidity_update,
    },
    whirlpool_manager::{next_whirlpool_liquidity, next_whirlpool_reward_infos},
};
use crate::error::DualTokenVoterError;
use crate::orca::{
    math::{get_amount_delta_a, get_amount_delta_b, sqrt_price_from_tick_index},
    state::*,
};
use anchor_lang::prelude::*;

#[derive(Debug)]
pub struct ModifyLiquidityUpdate {
    pub whirlpool_liquidity: u128,
    pub tick_lower_update: TickUpdate,
    pub tick_upper_update: TickUpdate,
    pub reward_infos: [WhirlpoolRewardInfo; NUM_REWARDS],
    pub position_update: PositionUpdate,
    pub tick_array_lower_update: TickArrayUpdate,
    pub tick_array_upper_update: TickArrayUpdate,
}

// Calculates state after modifying liquidity by the liquidity_delta for the given positon.
// Fee and reward growths will also be calculated by this function.
// To trigger only calculation of fee and reward growths, use calculate_fee_and_reward_growths.
pub fn calculate_modify_liquidity<'info>(
    whirlpool: &Whirlpool,
    position: &Position,
    tick_array_lower: &dyn TickArrayType,
    tick_array_upper: &dyn TickArrayType,
    liquidity_delta: i128,
    timestamp: u64,
) -> Result<ModifyLiquidityUpdate> {
    let tick_lower =
        tick_array_lower.get_tick(position.tick_lower_index, whirlpool.tick_spacing)?;

    let tick_upper =
        tick_array_upper.get_tick(position.tick_upper_index, whirlpool.tick_spacing)?;

    _calculate_modify_liquidity(
        whirlpool,
        position,
        &tick_lower,
        &tick_upper,
        position.tick_lower_index,
        position.tick_upper_index,
        tick_array_lower.is_variable_size(),
        tick_array_upper.is_variable_size(),
        liquidity_delta,
        timestamp,
    )
}

pub fn calculate_fee_and_reward_growths<'info>(
    whirlpool: &Whirlpool,
    position: &Position,
    tick_array_lower: &dyn TickArrayType,
    tick_array_upper: &dyn TickArrayType,
    timestamp: u64,
) -> Result<(PositionUpdate, [WhirlpoolRewardInfo; NUM_REWARDS])> {
    let tick_lower =
        tick_array_lower.get_tick(position.tick_lower_index, whirlpool.tick_spacing)?;

    let tick_upper =
        tick_array_upper.get_tick(position.tick_upper_index, whirlpool.tick_spacing)?;

    // Pass in a liquidity_delta value of 0 to trigger only calculations for fee and reward growths.
    // Calculating fees and rewards for positions with zero liquidity will result in an error.
    let update = _calculate_modify_liquidity(
        whirlpool,
        position,
        &tick_lower,
        &tick_upper,
        position.tick_lower_index,
        position.tick_upper_index,
        tick_array_lower.is_variable_size(),
        tick_array_upper.is_variable_size(),
        0,
        timestamp,
    )?;
    Ok((update.position_update, update.reward_infos))
}

// Calculates the state changes after modifying liquidity of a whirlpool position.
#[allow(clippy::too_many_arguments)]
fn _calculate_modify_liquidity(
    whirlpool: &Whirlpool,
    position: &Position,
    tick_lower: &Tick,
    tick_upper: &Tick,
    tick_lower_index: i32,
    tick_upper_index: i32,
    tick_array_lower_variable_size: bool,
    tick_array_upper_variable_size: bool,
    liquidity_delta: i128,
    timestamp: u64,
) -> Result<ModifyLiquidityUpdate> {
    // Disallow only updating position fee and reward growth when position has zero liquidity
    if liquidity_delta == 0 && position.liquidity == 0 {
        return Err(DualTokenVoterError::LiquidityZero.into());
    }

    let next_reward_infos = next_whirlpool_reward_infos(whirlpool, timestamp)?;

    let next_global_liquidity = next_whirlpool_liquidity(
        whirlpool,
        position.tick_upper_index,
        position.tick_lower_index,
        liquidity_delta,
    )?;

    let tick_lower_update = next_tick_modify_liquidity_update(
        tick_lower,
        tick_lower_index,
        whirlpool.tick_current_index,
        whirlpool.fee_growth_global_a,
        whirlpool.fee_growth_global_b,
        &next_reward_infos,
        liquidity_delta,
        false,
    )?;

    let tick_upper_update = next_tick_modify_liquidity_update(
        tick_upper,
        tick_upper_index,
        whirlpool.tick_current_index,
        whirlpool.fee_growth_global_a,
        whirlpool.fee_growth_global_b,
        &next_reward_infos,
        liquidity_delta,
        true,
    )?;

    let (fee_growth_inside_a, fee_growth_inside_b) = next_fee_growths_inside(
        whirlpool.tick_current_index,
        tick_lower,
        tick_lower_index,
        tick_upper,
        tick_upper_index,
        whirlpool.fee_growth_global_a,
        whirlpool.fee_growth_global_b,
    );

    let reward_growths_inside = next_reward_growths_inside(
        whirlpool.tick_current_index,
        tick_lower,
        tick_lower_index,
        tick_upper,
        tick_upper_index,
        &next_reward_infos,
    );

    let position_update = next_position_modify_liquidity_update(
        position,
        liquidity_delta,
        fee_growth_inside_a,
        fee_growth_inside_b,
        &reward_growths_inside,
    )?;

    let tick_array_lower_update = calculate_modify_tick_array(
        position,
        &position_update,
        tick_array_lower_variable_size,
        tick_lower,
        &tick_lower_update,
    )?;

    let tick_array_upper_update = calculate_modify_tick_array(
        position,
        &position_update,
        tick_array_upper_variable_size,
        tick_upper,
        &tick_upper_update,
    )?;

    Ok(ModifyLiquidityUpdate {
        whirlpool_liquidity: next_global_liquidity,
        reward_infos: next_reward_infos,
        position_update,
        tick_lower_update,
        tick_upper_update,
        tick_array_lower_update,
        tick_array_upper_update,
    })
}

pub fn calculate_liquidity_token_deltas(
    current_tick_index: i32,
    sqrt_price: u128,
    position: &Position,
    liquidity_delta: i128,
) -> Result<(u64, u64)> {
    if liquidity_delta == 0 {
        return Err(DualTokenVoterError::LiquidityZero.into());
    }

    let mut delta_a: u64 = 0;
    let mut delta_b: u64 = 0;

    let liquidity: u128 = liquidity_delta.unsigned_abs();
    let round_up = liquidity_delta > 0;

    let lower_price = sqrt_price_from_tick_index(position.tick_lower_index);
    let upper_price = sqrt_price_from_tick_index(position.tick_upper_index);

    if current_tick_index < position.tick_lower_index {
        // current tick below position
        delta_a = get_amount_delta_a(lower_price, upper_price, liquidity, round_up)?;
    } else if current_tick_index < position.tick_upper_index {
        // current tick inside position
        delta_a = get_amount_delta_a(sqrt_price, upper_price, liquidity, round_up)?;
        delta_b = get_amount_delta_b(lower_price, sqrt_price, liquidity, round_up)?;
    } else {
        // current tick above position
        delta_b = get_amount_delta_b(lower_price, upper_price, liquidity, round_up)?;
    }

    Ok((delta_a, delta_b))
}

pub fn sync_modify_liquidity_values<'info>(
    whirlpool: &mut Whirlpool,
    position: &mut Position,
    tick_array_lower: &mut dyn TickArrayType,
    tick_array_upper: Option<&mut dyn TickArrayType>,
    modify_liquidity_update: &ModifyLiquidityUpdate,
    reward_last_updated_timestamp: u64,
) -> Result<()> {
    position.update(&modify_liquidity_update.position_update);

    tick_array_lower.update_tick(
        position.tick_lower_index,
        whirlpool.tick_spacing,
        &modify_liquidity_update.tick_lower_update,
    )?;

    if let Some(tick_array_upper) = tick_array_upper {
        tick_array_upper.update_tick(
            position.tick_upper_index,
            whirlpool.tick_spacing,
            &modify_liquidity_update.tick_upper_update,
        )?;
    } else {
        // Upper and lower tick arrays are the same so we only have one ref
        tick_array_lower.update_tick(
            position.tick_upper_index,
            whirlpool.tick_spacing,
            &modify_liquidity_update.tick_upper_update,
        )?;
    }

    whirlpool.update_rewards_and_liquidity(
        modify_liquidity_update.reward_infos,
        modify_liquidity_update.whirlpool_liquidity,
        reward_last_updated_timestamp,
    );

    Ok(())
}
