use crate::error::DualTokenVoterError;
use crate::orca::state::*;
use crate::orca::util::ProxiedTickArray;
use anchor_lang::prelude::*;

use crate::orca::state::LoadedTickArrayMut;

pub struct SwapTickSequence<'a> {
    pub(crate) arrays: Vec<ProxiedTickArray<'a>>,
}

impl<'a> SwapTickSequence<'a> {
    pub fn new(
        ta0: LoadedTickArrayMut<'a>,
        ta1: Option<LoadedTickArrayMut<'a>>,
        ta2: Option<LoadedTickArrayMut<'a>>,
    ) -> Self {
        Self::new_with_proxy(
            ProxiedTickArray::new_initialized(ta0),
            ta1.map(ProxiedTickArray::new_initialized),
            ta2.map(ProxiedTickArray::new_initialized),
        )
    }

    pub(crate) fn new_with_proxy(
        ta0: ProxiedTickArray<'a>,
        ta1: Option<ProxiedTickArray<'a>>,
        ta2: Option<ProxiedTickArray<'a>>,
    ) -> Self {
        let mut vec = Vec::with_capacity(3);
        vec.push(ta0);
        if let Some(ta1) = ta1 {
            vec.push(ta1);
        }
        if let Some(ta2) = ta2 {
            vec.push(ta2);
        }
        Self { arrays: vec }
    }

    /// Get the Tick object at the given tick-index & tick-spacing
    ///
    /// # Parameters
    /// - `array_index` - the array index that the tick of this given tick-index would be stored in
    /// - `tick_index` - the tick index the desired Tick object is stored in
    /// - `tick_spacing` - A u8 integer of the tick spacing for this whirlpool
    ///
    /// # Returns
    /// - `&Tick`: A reference to the desired Tick object
    /// - `TickArrayIndexOutofBounds` - The provided array-index is out of bounds
    /// - `TickNotFound`: - The provided tick-index is not an initializable tick index in this Whirlpool w/ this tick-spacing.
    pub fn get_tick(&self, array_index: usize, tick_index: i32, tick_spacing: u16) -> Result<Tick> {
        let array = self.arrays.get(array_index);
        match array {
            Some(array) => array.get_tick(tick_index, tick_spacing),
            _ => Err(DualTokenVoterError::TickArrayIndexOutofBounds.into()),
        }
    }

    /// Updates the Tick object at the given tick-index & tick-spacing
    ///
    /// # Parameters
    /// - `array_index` - the array index that the tick of this given tick-index would be stored in
    /// - `tick_index` - the tick index the desired Tick object is stored in
    /// - `tick_spacing` - A u8 integer of the tick spacing for this whirlpool
    /// - `update` - A reference to a TickUpdate object to update the Tick object at the given index
    ///
    /// # Errors
    /// - `TickArrayIndexOutofBounds` - The provided array-index is out of bounds
    /// - `TickNotFound`: - The provided tick-index is not an initializable tick index in this Whirlpool w/ this tick-spacing.
    pub fn update_tick(
        &mut self,
        array_index: usize,
        tick_index: i32,
        tick_spacing: u16,
        update: &TickUpdate,
    ) -> Result<()> {
        let array = self.arrays.get_mut(array_index);
        match array {
            Some(array) => {
                array.update_tick(tick_index, tick_spacing, update)?;
                Ok(())
            }
            _ => Err(DualTokenVoterError::TickArrayIndexOutofBounds.into()),
        }
    }

    pub fn get_tick_offset(
        &self,
        array_index: usize,
        tick_index: i32,
        tick_spacing: u16,
    ) -> Result<isize> {
        let array = self.arrays.get(array_index);
        match array {
            Some(array) => array.tick_offset(tick_index, tick_spacing),
            _ => Err(DualTokenVoterError::TickArrayIndexOutofBounds.into()),
        }
    }

    /// Get the next initialized tick in the provided tick range
    ///
    /// # Parameters
    /// - `tick_index` - the tick index to start searching from
    /// - `tick_spacing` - A u8 integer of the tick spacing for this whirlpool
    /// - `a_to_b` - If the trade is from a_to_b, the search will move to the left and the starting search tick is inclusive.
    ///              If the trade is from b_to_a, the search will move to the right and the starting search tick is not inclusive.
    /// - `start_array_index` -
    ///
    /// # Returns
    /// - `(usize, i32, &mut Tick)`: The array_index which the next initialized index was found, the next initialized tick-index & a mutable reference to that tick
    /// - `TickArraySequenceInvalidIndex` - The swap loop provided an invalid array index to query the next tick in.
    /// - `InvalidTickArraySequence`: - User provided tick-arrays are not in sequential order required to proceed in this trade direction.
    pub fn get_next_initialized_tick_index(
        &self,
        tick_index: i32,
        tick_spacing: u16,
        a_to_b: bool,
        start_array_index: usize,
    ) -> Result<(usize, i32)> {
        let ticks_in_array = TICK_ARRAY_SIZE * tick_spacing as i32;
        let mut search_index = tick_index;
        let mut array_index = start_array_index;

        // Keep looping the arrays until an initialized tick index in the subsequent tick-arrays found.
        loop {
            // If we get to the end of the array sequence and next_index is still not found, throw error
            let next_array = match self.arrays.get(array_index) {
                Some(array) => array,
                None => return Err(DualTokenVoterError::TickArraySequenceInvalidIndex.into()),
            };

            let next_index =
                next_array.get_next_init_tick_index(search_index, tick_spacing, a_to_b)?;

            match next_index {
                Some(next_index) => {
                    return Ok((array_index, next_index));
                }
                None => {
                    // If we are at the last valid tick array, return the min/max tick index
                    if a_to_b && next_array.is_min_tick_array() {
                        return Ok((array_index, MIN_TICK_INDEX));
                    } else if !a_to_b && next_array.is_max_tick_array(tick_spacing) {
                        return Ok((array_index, MAX_TICK_INDEX));
                    }

                    // If we are at the last tick array in the sequencer, return the last tick
                    if array_index + 1 == self.arrays.len() {
                        if a_to_b {
                            return Ok((array_index, next_array.start_tick_index()));
                        } else {
                            let last_tick = next_array.start_tick_index() + ticks_in_array - 1;
                            return Ok((array_index, last_tick));
                        }
                    }

                    // No initialized index found. Move the search-index to the 1st search position
                    // of the next array in sequence.
                    search_index = if a_to_b {
                        next_array.start_tick_index() - 1
                    } else {
                        next_array.start_tick_index() + ticks_in_array - 1
                    };

                    array_index += 1;
                }
            }
        }
    }
}
