use crate::error::DualTokenVoterError;
use crate::orca::{
    math::add_liquidity_delta,
    state::{Tick, TickUpdate, WhirlpoolRewardInfo, NUM_REWARDS},
};

pub fn next_tick_cross_update(
    tick: &Tick,
    fee_growth_global_a: u128,
    fee_growth_global_b: u128,
    reward_infos: &[WhirlpoolRewardInfo; NUM_REWARDS],
) -> Result<TickUpdate, DualTokenVoterError> {
    let mut update = TickUpdate::from(*tick);

    update.fee_growth_outside_a = fee_growth_global_a.wrapping_sub(tick.fee_growth_outside_a);
    update.fee_growth_outside_b = fee_growth_global_b.wrapping_sub(tick.fee_growth_outside_b);

    for (i, reward_info) in reward_infos.iter().enumerate() {
        if !reward_info.initialized() {
            continue;
        }

        update.reward_growths_outside[i] = reward_info
            .growth_global_x64
            .wrapping_sub(tick.reward_growths_outside[i]);
    }
    Ok(update)
}

#[allow(clippy::too_many_arguments)]
pub fn next_tick_modify_liquidity_update(
    tick: &Tick,
    tick_index: i32,
    tick_current_index: i32,
    fee_growth_global_a: u128,
    fee_growth_global_b: u128,
    reward_infos: &[WhirlpoolRewardInfo; NUM_REWARDS],
    liquidity_delta: i128,
    is_upper_tick: bool,
) -> Result<TickUpdate, DualTokenVoterError> {
    // noop if there is no change in liquidity
    if liquidity_delta == 0 {
        return Ok((*tick).into());
    }

    let liquidity_gross = add_liquidity_delta(tick.liquidity_gross, liquidity_delta)?;

    // Update to an uninitialized tick if remaining liquidity is being removed
    if liquidity_gross == 0 {
        return Ok(TickUpdate::default());
    }

    let (fee_growth_outside_a, fee_growth_outside_b, reward_growths_outside) =
        if tick.liquidity_gross == 0 {
            // By convention, assume all prior growth happened below the tick
            if tick_current_index >= tick_index {
                (
                    fee_growth_global_a,
                    fee_growth_global_b,
                    WhirlpoolRewardInfo::to_reward_growths(reward_infos),
                )
            } else {
                (0, 0, [0; NUM_REWARDS])
            }
        } else {
            (
                tick.fee_growth_outside_a,
                tick.fee_growth_outside_b,
                tick.reward_growths_outside,
            )
        };

    let liquidity_net = if is_upper_tick {
        tick.liquidity_net
            .checked_sub(liquidity_delta)
            .ok_or(DualTokenVoterError::LiquidityNetError)?
    } else {
        tick.liquidity_net
            .checked_add(liquidity_delta)
            .ok_or(DualTokenVoterError::LiquidityNetError)?
    };

    Ok(TickUpdate {
        initialized: true,
        liquidity_net,
        liquidity_gross,
        fee_growth_outside_a,
        fee_growth_outside_b,
        reward_growths_outside,
    })
}

// Calculates the fee growths inside of tick_lower and tick_upper based on their
// index relative to tick_current_index.
pub fn next_fee_growths_inside(
    tick_current_index: i32,
    tick_lower: &Tick,
    tick_lower_index: i32,
    tick_upper: &Tick,
    tick_upper_index: i32,
    fee_growth_global_a: u128,
    fee_growth_global_b: u128,
) -> (u128, u128) {
    // By convention, when initializing a tick, all fees have been earned below the tick.
    let (fee_growth_below_a, fee_growth_below_b) = if !tick_lower.initialized {
        (fee_growth_global_a, fee_growth_global_b)
    } else if tick_current_index < tick_lower_index {
        (
            fee_growth_global_a.wrapping_sub(tick_lower.fee_growth_outside_a),
            fee_growth_global_b.wrapping_sub(tick_lower.fee_growth_outside_b),
        )
    } else {
        (
            tick_lower.fee_growth_outside_a,
            tick_lower.fee_growth_outside_b,
        )
    };

    // By convention, when initializing a tick, no fees have been earned above the tick.
    let (fee_growth_above_a, fee_growth_above_b) = if !tick_upper.initialized {
        (0, 0)
    } else if tick_current_index < tick_upper_index {
        (
            tick_upper.fee_growth_outside_a,
            tick_upper.fee_growth_outside_b,
        )
    } else {
        (
            fee_growth_global_a.wrapping_sub(tick_upper.fee_growth_outside_a),
            fee_growth_global_b.wrapping_sub(tick_upper.fee_growth_outside_b),
        )
    };

    (
        fee_growth_global_a
            .wrapping_sub(fee_growth_below_a)
            .wrapping_sub(fee_growth_above_a),
        fee_growth_global_b
            .wrapping_sub(fee_growth_below_b)
            .wrapping_sub(fee_growth_above_b),
    )
}

// Calculates the reward growths inside of tick_lower and tick_upper based on their positions
// relative to tick_current_index. An uninitialized reward will always have a reward growth of zero.
pub fn next_reward_growths_inside(
    tick_current_index: i32,
    tick_lower: &Tick,
    tick_lower_index: i32,
    tick_upper: &Tick,
    tick_upper_index: i32,
    reward_infos: &[WhirlpoolRewardInfo; NUM_REWARDS],
) -> [u128; NUM_REWARDS] {
    let mut reward_growths_inside = [0; NUM_REWARDS];

    for i in 0..NUM_REWARDS {
        if !reward_infos[i].initialized() {
            continue;
        }

        // By convention, assume all prior growth happened below the tick
        let reward_growths_below = if !tick_lower.initialized {
            reward_infos[i].growth_global_x64
        } else if tick_current_index < tick_lower_index {
            reward_infos[i]
                .growth_global_x64
                .wrapping_sub(tick_lower.reward_growths_outside[i])
        } else {
            tick_lower.reward_growths_outside[i]
        };

        // By convention, assume all prior growth happened below the tick, not above
        let reward_growths_above = if !tick_upper.initialized {
            0
        } else if tick_current_index < tick_upper_index {
            tick_upper.reward_growths_outside[i]
        } else {
            reward_infos[i]
                .growth_global_x64
                .wrapping_sub(tick_upper.reward_growths_outside[i])
        };

        reward_growths_inside[i] = reward_infos[i]
            .growth_global_x64
            .wrapping_sub(reward_growths_below)
            .wrapping_sub(reward_growths_above);
    }

    reward_growths_inside
}
