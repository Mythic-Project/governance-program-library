pub mod whirlpool;
pub mod tick;
pub mod tick_array;
pub mod oracle;
pub mod fixed_tick_array;
pub mod zeroed_tick_array;

pub use whirlpool::*;
pub use tick::*;
pub use tick_array::{*, LoadedTickArray, LoadedTickArrayMut, load_tick_array_mut};
pub use oracle::*;
pub use fixed_tick_array::FixedTickArray;
pub use zeroed_tick_array::ZeroedTickArray;

pub const TICK_ARRAY_SIZE: i32 = 88;
pub const TICK_ARRAY_SIZE_USIZE: usize = 88;
pub const NUM_REWARDS: usize = 3;
