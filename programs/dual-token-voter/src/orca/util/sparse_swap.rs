use anchor_lang::{prelude::*, system_program};
use std::collections::VecDeque;

use crate::orca::{
    math::floor_division,
    state::{
        FixedTickArray, Tick, TickArrayType, TickUpdate, Whirlpool, ZeroedTickArray,
        TICK_ARRAY_SIZE,
    },
    util::SwapTickSequence,
};

use crate::orca::state::{load_tick_array_mut, LoadedTickArrayMut};

pub(crate) enum ProxiedTickArray<'a> {
    Initialized(LoadedTickArrayMut<'a>),
    Uninitialized(ZeroedTickArray),
}

impl<'a> ProxiedTickArray<'a> {
    pub fn new_initialized(refmut: LoadedTickArrayMut<'a>) -> Self {
        ProxiedTickArray::Initialized(refmut)
    }

    pub fn new_uninitialized(start_tick_index: i32) -> Self {
        ProxiedTickArray::Uninitialized(ZeroedTickArray::new(start_tick_index))
    }

    pub fn start_tick_index(&self) -> i32 {
        self.as_ref().start_tick_index()
    }

    pub fn get_next_init_tick_index(
        &self,
        tick_index: i32,
        tick_spacing: u16,
        a_to_b: bool,
    ) -> Result<Option<i32>> {
        self.as_ref()
            .get_next_init_tick_index(tick_index, tick_spacing, a_to_b)
    }

    pub fn get_tick(&self, tick_index: i32, tick_spacing: u16) -> Result<Tick> {
        self.as_ref().get_tick(tick_index, tick_spacing)
    }

    pub fn update_tick(
        &mut self,
        tick_index: i32,
        tick_spacing: u16,
        update: &TickUpdate,
    ) -> Result<()> {
        self.as_mut().update_tick(tick_index, tick_spacing, update)
    }

    pub fn is_min_tick_array(&self) -> bool {
        self.as_ref().is_min_tick_array()
    }

    pub fn is_max_tick_array(&self, tick_spacing: u16) -> bool {
        self.as_ref().is_max_tick_array(tick_spacing)
    }

    pub fn tick_offset(&self, tick_index: i32, tick_spacing: u16) -> Result<isize> {
        self.as_ref().tick_offset(tick_index, tick_spacing)
    }
}

impl<'a> AsRef<dyn TickArrayType + 'a> for ProxiedTickArray<'a> {
    fn as_ref(&self) -> &(dyn TickArrayType + 'a) {
        match self {
            ProxiedTickArray::Initialized(ref array) => &**array,
            ProxiedTickArray::Uninitialized(ref array) => array,
        }
    }
}

impl<'a> AsMut<dyn TickArrayType + 'a> for ProxiedTickArray<'a> {
    fn as_mut(&mut self) -> &mut (dyn TickArrayType + 'a) {
        match self {
            ProxiedTickArray::Initialized(ref mut array) => &mut **array,
            ProxiedTickArray::Uninitialized(ref mut array) => array,
        }
    }
}

pub struct SparseSwapTickSequenceBuilder<'info> {
    // AccountInfo ownership must be kept while using RefMut.
    // This is why try_from and build are separated and SparseSwapTickSequenceBuilder struct is used.
    tick_array_accounts: Vec<AccountInfo<'info>>,
}

impl<'info> SparseSwapTickSequenceBuilder<'info> {
    /// Create a new SparseSwapTickSequenceBuilder from the given tick array accounts.
    ///
    /// static_tick_array_account_infos and supplemental_tick_array_account_infos will be merged,
    /// and deduplicated by key. TickArray accounts can be provided in any order.
    ///
    /// Even if over three tick arrays are provided, only three tick arrays are used in the single swap.
    /// The extra TickArray acts as a fallback in case the current price moves.
    pub fn new(
        static_tick_array_account_infos: Vec<AccountInfo<'info>>,
        supplemental_tick_array_account_infos: Option<Vec<AccountInfo<'info>>>,
    ) -> Self {
        let mut tick_array_account_infos: Vec<AccountInfo<'info>> = static_tick_array_account_infos;
        if let Some(supplemental_tick_array_account_infos) = supplemental_tick_array_account_infos {
            tick_array_account_infos.extend(supplemental_tick_array_account_infos);
        }

        // dedup by key
        tick_array_account_infos.sort_by_key(|a| a.key());
        tick_array_account_infos.dedup_by_key(|a| a.key());

        Self {
            tick_array_accounts: tick_array_account_infos,
        }
    }

    /// # Parameters
    /// - `whirlpool` - Whirlpool account
    /// - `a_to_b` - Direction of the swap
    ///
    /// # Errors
    /// - `DifferentWhirlpoolTickArrayAccount` - If the provided TickArray account is not for the whirlpool
    /// - `InvalidTickArraySequence` - If no valid TickArray account for the swap is found
    /// - `AccountNotMutable` - If the provided TickArray account is not mutable
    /// - `AccountOwnedByWrongProgram` - If the provided initialized TickArray account is not owned by this program
    /// - `AccountDiscriminatorNotFound` - If the provided TickArray account does not have a discriminator
    /// - `AccountDiscriminatorMismatch` - If the provided TickArray account has a mismatched discriminator
    pub fn try_build<'a>(
        &'a self,
        whirlpool: &Whirlpool,
        whirlpool_key: &Pubkey,
        a_to_b: bool,
    ) -> Result<SwapTickSequence<'a>> {
        let mut loaded_tick_arrays: Vec<LoadedTickArrayMut> = Vec::with_capacity(3);
        for account_info in &self.tick_array_accounts {
            let tick_array = maybe_load_tick_array(account_info, whirlpool_key)?;
            if let Some(tick_array) = tick_array {
                loaded_tick_arrays.push(tick_array);
            }
        }

        let start_tick_indexes = get_start_tick_indexes(whirlpool, a_to_b);
        let mut required_tick_arrays: VecDeque<ProxiedTickArray> = VecDeque::with_capacity(3);
        for start_tick_index in start_tick_indexes.iter() {
            let pos = loaded_tick_arrays
                .iter()
                .position(|tick_array| tick_array.start_tick_index() == *start_tick_index);
            if let Some(pos) = pos {
                let tick_array = loaded_tick_arrays.remove(pos);
                required_tick_arrays.push_back(ProxiedTickArray::new_initialized(tick_array));
                continue;
            }

            let tick_array_pda = derive_tick_array_pda(whirlpool_key, *start_tick_index);
            let has_account_info = self
                .tick_array_accounts
                .iter()
                .any(|account_info| account_info.key() == tick_array_pda);
            if has_account_info {
                required_tick_arrays
                    .push_back(ProxiedTickArray::new_uninitialized(*start_tick_index));
                continue;
            }
            break;
        }

        if required_tick_arrays.is_empty() {
            return Err(crate::error::DualTokenVoterError::InvalidTickArraySequence.into());
        }

        Ok(SwapTickSequence::new_with_proxy(
            required_tick_arrays.pop_front().unwrap(),
            required_tick_arrays.pop_front(),
            required_tick_arrays.pop_front(),
        ))
    }
}

fn maybe_load_tick_array<'a>(
    account_info: &'a AccountInfo<'_>,
    whirlpool_key: &Pubkey,
) -> Result<Option<LoadedTickArrayMut<'a>>> {
    if *account_info.owner == system_program::ID && account_info.data_is_empty() {
        return Ok(None);
    }

    let tick_array = load_tick_array_mut(account_info, whirlpool_key)?;
    Ok(Some(tick_array))
}

fn derive_tick_array_pda(whirlpool_key: &Pubkey, start_tick_index: i32) -> Pubkey {
    Pubkey::find_program_address(
        &[
            b"tick_array",
            whirlpool_key.as_ref(),
            start_tick_index.to_string().as_bytes(),
        ],
        &FixedTickArray::owner(),
    )
    .0
}

fn get_start_tick_indexes(whirlpool: &Whirlpool, a_to_b: bool) -> Vec<i32> {
    let tick_current_index = whirlpool.tick_current_index;
    let tick_spacing_u16 = whirlpool.tick_spacing;
    let tick_spacing_i32 = whirlpool.tick_spacing as i32;
    let ticks_in_array = TICK_ARRAY_SIZE * tick_spacing_i32;

    let start_tick_index_base = floor_division(tick_current_index, ticks_in_array) * ticks_in_array;
    let offset = if a_to_b {
        [0, -1, -2]
    } else {
        let shifted =
            tick_current_index + tick_spacing_i32 >= start_tick_index_base + ticks_in_array;
        if shifted {
            [1, 2, 3]
        } else {
            [0, 1, 2]
        }
    };

    let start_tick_indexes = offset
        .iter()
        .filter_map(|&o| {
            let start_tick_index = start_tick_index_base + o * ticks_in_array;
            if Tick::check_is_valid_start_tick(start_tick_index, tick_spacing_u16) {
                Some(start_tick_index)
            } else {
                None
            }
        })
        .collect::<Vec<i32>>();

    start_tick_indexes
}
