use std::{
    cell::{Ref, RefMut},
    ops::{Deref, DerefMut},
};

use crate::error::DualTokenVoterError;
use anchor_lang::{prelude::*, Discriminator};
use arrayref::array_ref;

use super::{
    Tick, TickUpdate, MAX_TICK_INDEX, MIN_TICK_INDEX,
    fixed_tick_array::FixedTickArray,
};

// We have two consts because most of our code uses it as a i32. However,
// for us to use it in tick array declarations, anchor requires it to be a usize.
pub const TICK_ARRAY_SIZE: i32 = 88;
pub const TICK_ARRAY_SIZE_USIZE: usize = 88;

pub trait TickArrayType {
    fn is_variable_size(&self) -> bool;
    fn start_tick_index(&self) -> i32;
    fn whirlpool(&self) -> Pubkey;

    fn get_next_init_tick_index(
        &self,
        tick_index: i32,
        tick_spacing: u16,
        a_to_b: bool,
    ) -> Result<Option<i32>>;

    fn get_tick(&self, tick_index: i32, tick_spacing: u16) -> Result<Tick>;

    fn update_tick(
        &mut self,
        tick_index: i32,
        tick_spacing: u16,
        update: &TickUpdate,
    ) -> Result<()>;

    /// Checks that this array holds the next tick index for the current tick index, given the pool's tick spacing & search direction.
    ///
    /// unshifted checks on [start, start + TICK_ARRAY_SIZE * tick_spacing)
    /// shifted checks on [start - tick_spacing, start + (TICK_ARRAY_SIZE - 1) * tick_spacing) (adjusting range by -tick_spacing)
    ///
    /// shifted == !a_to_b
    ///
    /// For a_to_b swaps, price moves left. All searchable ticks in this tick-array's range will end up in this tick's usable ticks.
    /// The search range is therefore the range of the tick-array.
    ///
    /// For b_to_a swaps, this tick-array's left-most ticks can be the 'next' usable tick-index of the previous tick-array.
    /// The right-most ticks also points towards the next tick-array. The search range is therefore shifted by 1 tick-spacing.
    fn in_search_range(&self, tick_index: i32, tick_spacing: u16, shifted: bool) -> bool {
        let mut lower = self.start_tick_index();
        let mut upper = self.start_tick_index() + TICK_ARRAY_SIZE * tick_spacing as i32;
        if shifted {
            lower -= tick_spacing as i32;
            upper -= tick_spacing as i32;
        }
        tick_index >= lower && tick_index < upper
    }

    fn check_in_array_bounds(&self, tick_index: i32, tick_spacing: u16) -> bool {
        self.in_search_range(tick_index, tick_spacing, false)
    }

    fn is_min_tick_array(&self) -> bool {
        self.start_tick_index() <= MIN_TICK_INDEX
    }

    fn is_max_tick_array(&self, tick_spacing: u16) -> bool {
        self.start_tick_index() + TICK_ARRAY_SIZE * (tick_spacing as i32) > MAX_TICK_INDEX
    }

    fn tick_offset(&self, tick_index: i32, tick_spacing: u16) -> Result<isize> {
        if tick_spacing == 0 {
            return Err(DualTokenVoterError::InvalidTickSpacing.into());
        }

        Ok(get_offset(
            tick_index,
            self.start_tick_index(),
            tick_spacing,
        ))
    }
}

fn get_offset(tick_index: i32, start_tick_index: i32, tick_spacing: u16) -> isize {
    // TODO: replace with i32.div_floor once not experimental
    let lhs = tick_index - start_tick_index;
    // rhs(tick_spacing) is always positive number (non zero)
    let rhs = tick_spacing as i32;
    let d = lhs / rhs;
    let r = lhs % rhs;
    let o = if r < 0 { d - 1 } else { d };
    o as isize
}

pub type LoadedTickArray<'a> = Ref<'a, dyn TickArrayType>;

pub fn load_tick_array<'a>(
    account: &'a AccountInfo<'_>,
    whirlpool: &Pubkey,
) -> Result<LoadedTickArray<'a>> {
    if *account.owner != crate::ID {
        return Err(ErrorCode::AccountOwnedByWrongProgram.into());
    }

    let data = account.try_borrow_data()?;

    if data.len() < 8 {
        return Err(ErrorCode::AccountDiscriminatorNotFound.into());
    }

    let discriminator = array_ref![data, 0, 8];
    let tick_array: LoadedTickArray<'a> = if *discriminator == FixedTickArray::DISCRIMINATOR {
        Ref::map(data, |data| {
            let tick_array: &FixedTickArray = bytemuck::from_bytes(&data[8..]);
            tick_array
        })
    } else {
        // We only support FixedTickArray for exchange rate calculation
        return Err(ErrorCode::AccountDiscriminatorMismatch.into());
    };

    if tick_array.whirlpool() != *whirlpool {
        return Err(DualTokenVoterError::DifferentWhirlpoolTickArrayAccount.into());
    }

    Ok(tick_array)
}

pub type LoadedTickArrayMut<'a> = RefMut<'a, dyn TickArrayType>;

pub fn load_tick_array_mut<'a, 'info>(
    account: &'a AccountInfo<'info>,
    whirlpool: &Pubkey,
) -> Result<LoadedTickArrayMut<'a>> {
    if !account.is_writable {
        return Err(ErrorCode::AccountNotMutable.into());
    }

    if *account.owner != crate::ID {
        return Err(ErrorCode::AccountOwnedByWrongProgram.into());
    }

    let data = account.try_borrow_mut_data()?;

    if data.len() < 8 {
        return Err(ErrorCode::AccountDiscriminatorNotFound.into());
    }

    let discriminator = array_ref![data, 0, 8];
    let tick_array: LoadedTickArrayMut<'a> = if *discriminator == FixedTickArray::DISCRIMINATOR {
        RefMut::map(data, |data| {
            let tick_array: &mut FixedTickArray =
                bytemuck::from_bytes_mut(&mut data.deref_mut()[8..]);
            tick_array
        })
    } else {
        // We only support FixedTickArray for exchange rate calculation
        return Err(ErrorCode::AccountDiscriminatorMismatch.into());
    };

    if tick_array.whirlpool() != *whirlpool {
        return Err(DualTokenVoterError::DifferentWhirlpoolTickArrayAccount.into());
    }

    Ok(tick_array)
}

/// In increase and decrease liquidity, we directly load the tick arrays mutably.
/// Lower and upper ticker arrays might refer to the same account. We cannot load
/// the same account mutably twice so we just return None if the accounts are the same.
pub struct TickArraysMut<'a> {
    lower_tick_array_ref: LoadedTickArrayMut<'a>,
    upper_tick_array_ref: Option<LoadedTickArrayMut<'a>>,
}

impl<'a> TickArraysMut<'a> {
    pub fn load(
        lower_tick_array_info: &'a AccountInfo<'_>,
        upper_tick_array_info: &'a AccountInfo<'_>,
        whirlpool: &Pubkey,
    ) -> Result<Self> {
        let lower_tick_array = load_tick_array_mut(lower_tick_array_info, whirlpool)?;
        let upper_tick_array = if lower_tick_array_info.key() == upper_tick_array_info.key() {
            None
        } else {
            Some(load_tick_array_mut(upper_tick_array_info, whirlpool)?)
        };
        Ok(Self {
            lower_tick_array_ref: lower_tick_array,
            upper_tick_array_ref: upper_tick_array,
        })
    }

    pub fn deref(&self) -> (&dyn TickArrayType, &dyn TickArrayType) {
        if let Some(upper_tick_array_ref) = &self.upper_tick_array_ref {
            (
                self.lower_tick_array_ref.deref(),
                upper_tick_array_ref.deref(),
            )
        } else {
            (
                self.lower_tick_array_ref.deref(),
                self.lower_tick_array_ref.deref(),
            )
        }
    }

    // Since we can only borrow mutably once, we return None if the upper tick array
    // is the same as the lower tick array
    pub fn deref_mut(&mut self) -> (&mut dyn TickArrayType, Option<&mut dyn TickArrayType>) {
        if let Some(upper_tick_array_ref) = &mut self.upper_tick_array_ref {
            (
                self.lower_tick_array_ref.deref_mut(),
                Some(upper_tick_array_ref.deref_mut()),
            )
        } else {
            (self.lower_tick_array_ref.deref_mut(), None)
        }
    }
}
