use crate::{
    error::DualTokenVoterError,
    orca::math::{
        MAX_FEE_RATE, MAX_PROTOCOL_FEE_RATE,
    },
};
use anchor_lang::prelude::*;
use bitflags::bitflags;

#[zero_copy(unsafe)]
#[derive(Default)]
pub struct Whirlpool {
    pub whirlpools_config: Pubkey, // 32
    pub whirlpool_bump: [u8; 1],   // 1

    pub tick_spacing: u16,            // 2
    pub fee_tier_index_seed: [u8; 2], // 2

    // Stored as hundredths of a basis point
    // u16::MAX corresponds to ~6.5%
    pub fee_rate: u16, // 2

    // Portion of fee rate taken stored as basis points
    pub protocol_fee_rate: u16, // 2

    // Maximum amount that can be held by Solana account
    pub liquidity: u128, // 16

    // MAX/MIN at Q32.64, but using Q64.64 for rounder bytes
    // Q64.64
    pub sqrt_price: u128,        // 16
    pub tick_current_index: i32, // 4

    pub protocol_fee_owed_a: u64, // 8
    pub protocol_fee_owed_b: u64, // 8

    pub token_mint_a: Pubkey,  // 32
    pub token_vault_a: Pubkey, // 32

    // Q64.64
    pub fee_growth_global_a: u128, // 16

    pub token_mint_b: Pubkey,  // 32
    pub token_vault_b: Pubkey, // 32

    // Q64.64
    pub fee_growth_global_b: u128, // 16

    pub reward_last_updated_timestamp: u64, // 8

    pub reward_infos: [WhirlpoolRewardInfo; NUM_REWARDS], // 384
}

// Number of rewards supported by Whirlpools
pub const NUM_REWARDS: usize = 3;

impl Whirlpool {
    pub const LEN: usize = 8 + 261 + 384;
    pub fn seeds(&self) -> [&[u8]; 6] {
        [
            &b"whirlpool"[..],
            self.whirlpools_config.as_ref(),
            self.token_mint_a.as_ref(),
            self.token_mint_b.as_ref(),
            self.fee_tier_index_seed.as_ref(),
            self.whirlpool_bump.as_ref(),
        ]
    }

    pub fn input_token_mint(&self, a_to_b: bool) -> Pubkey {
        if a_to_b {
            self.token_mint_a
        } else {
            self.token_mint_b
        }
    }

    pub fn input_token_vault(&self, a_to_b: bool) -> Pubkey {
        if a_to_b {
            self.token_vault_a
        } else {
            self.token_vault_b
        }
    }

    pub fn output_token_mint(&self, a_to_b: bool) -> Pubkey {
        if a_to_b {
            self.token_mint_b
        } else {
            self.token_mint_a
        }
    }

    pub fn output_token_vault(&self, a_to_b: bool) -> Pubkey {
        if a_to_b {
            self.token_vault_b
        } else {
            self.token_vault_a
        }
    }

    pub fn reward_authority(&self) -> Pubkey {
        Pubkey::from(self.reward_infos[0].extension)
    }

    pub fn extension_segment_primary(&self) -> WhirlpoolExtensionSegmentPrimary {
        WhirlpoolExtensionSegmentPrimary::try_from_slice(&self.reward_infos[1].extension)
            .expect("Failed to deserialize WhirlpoolExtensionSegmentPrimary")
    }

    pub fn extension_segment_secondary(&self) -> WhirlpoolExtensionSegmentSecondary {
        WhirlpoolExtensionSegmentSecondary::try_from_slice(&self.reward_infos[2].extension)
            .expect("Failed to deserialize WhirlpoolExtensionSegmentSecondary")
    }

    // NOTE: initialize() function commented out - we only read existing Whirlpools for exchange rate calculation
    // If you need to create Whirlpools, you would need to add WhirlpoolsConfig type and uncomment this function
    /*
    #[allow(clippy::too_many_arguments)]
    pub fn initialize(
        &mut self,
        whirlpools_config: &Account<WhirlpoolsConfig>,
        fee_tier_index: u16,
        bump: u8,
        tick_spacing: u16,
        sqrt_price: u128,
        default_fee_rate: u16,
        token_mint_a: Pubkey,
        token_vault_a: Pubkey,
        token_mint_b: Pubkey,
        token_vault_b: Pubkey,
        control_flags: WhirlpoolControlFlags,
    ) -> Result<()> {
        if token_mint_a.ge(&token_mint_b) {
            return Err(DualTokenVoterError::InvalidTokenMintOrder.into());
        }

        if !(MIN_SQRT_PRICE_X64..=MAX_SQRT_PRICE_X64).contains(&sqrt_price) {
            return Err(DualTokenVoterError::SqrtPriceOutOfBounds.into());
        }

        if tick_spacing == 0 {
            // FeeTier and AdaptiveFeeTier enforce tick_spacing > 0
            unreachable!("tick_spacing must be greater than 0");
        }

        self.whirlpools_config = whirlpools_config.key();
        self.fee_tier_index_seed = fee_tier_index.to_le_bytes();
        self.whirlpool_bump = [bump];

        self.tick_spacing = tick_spacing;

        self.update_fee_rate(default_fee_rate)?;
        self.update_protocol_fee_rate(whirlpools_config.default_protocol_fee_rate)?;

        self.liquidity = 0;
        self.sqrt_price = sqrt_price;
        self.tick_current_index = tick_index_from_sqrt_price(&sqrt_price);

        self.protocol_fee_owed_a = 0;
        self.protocol_fee_owed_b = 0;

        self.token_mint_a = token_mint_a;
        self.token_vault_a = token_vault_a;
        self.fee_growth_global_a = 0;

        self.token_mint_b = token_mint_b;
        self.token_vault_b = token_vault_b;
        self.fee_growth_global_b = 0;

        self.reward_infos[0] = WhirlpoolRewardInfo::new(
            whirlpools_config
                .reward_emissions_super_authority
                .to_bytes(),
        );
        self.reward_infos[1] = WhirlpoolRewardInfo::new(
            WhirlpoolExtensionSegmentPrimary::new(control_flags).to_bytes(),
        );
        self.reward_infos[2] =
            WhirlpoolRewardInfo::new(WhirlpoolExtensionSegmentSecondary::new().to_bytes());

        Ok(())
    }
    */

    /// Update all reward values for the Whirlpool.
    ///
    /// # Parameters
    /// - `reward_infos` - An array of all updated whirlpool rewards
    /// - `reward_last_updated_timestamp` - The timestamp when the rewards were last updated
    pub fn update_rewards(
        &mut self,
        reward_infos: [WhirlpoolRewardInfo; NUM_REWARDS],
        reward_last_updated_timestamp: u64,
    ) {
        self.reward_last_updated_timestamp = reward_last_updated_timestamp;
        self.reward_infos = reward_infos;
    }

    pub fn update_rewards_and_liquidity(
        &mut self,
        reward_infos: [WhirlpoolRewardInfo; NUM_REWARDS],
        liquidity: u128,
        reward_last_updated_timestamp: u64,
    ) {
        self.update_rewards(reward_infos, reward_last_updated_timestamp);
        self.liquidity = liquidity;
    }

    /// Update the reward authority.
    pub fn update_reward_authority(&mut self, authority: Pubkey) -> Result<()> {
        self.reward_infos[0]
            .extension
            .copy_from_slice(&authority.to_bytes());

        Ok(())
    }

    pub fn update_emissions(
        &mut self,
        index: usize,
        reward_infos: [WhirlpoolRewardInfo; NUM_REWARDS],
        timestamp: u64,
        emissions_per_second_x64: u128,
    ) -> Result<()> {
        if index >= NUM_REWARDS {
            return Err(DualTokenVoterError::InvalidRewardIndex.into());
        }
        self.update_rewards(reward_infos, timestamp);
        self.reward_infos[index].emissions_per_second_x64 = emissions_per_second_x64;

        Ok(())
    }

    pub fn initialize_reward(&mut self, index: usize, mint: Pubkey, vault: Pubkey) -> Result<()> {
        if index >= NUM_REWARDS {
            return Err(DualTokenVoterError::InvalidRewardIndex.into());
        }

        let lowest_index = match self.reward_infos.iter().position(|r| !r.initialized()) {
            Some(lowest_index) => lowest_index,
            None => return Err(DualTokenVoterError::InvalidRewardIndex.into()),
        };

        if lowest_index != index {
            return Err(DualTokenVoterError::InvalidRewardIndex.into());
        }

        self.reward_infos[index].mint = mint;
        self.reward_infos[index].vault = vault;

        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn update_after_swap(
        &mut self,
        liquidity: u128,
        tick_index: i32,
        sqrt_price: u128,
        fee_growth_global: u128,
        reward_infos: [WhirlpoolRewardInfo; NUM_REWARDS],
        protocol_fee: u64,
        is_token_fee_in_a: bool,
        reward_last_updated_timestamp: u64,
    ) {
        self.tick_current_index = tick_index;
        self.sqrt_price = sqrt_price;
        self.liquidity = liquidity;
        self.reward_infos = reward_infos;
        self.reward_last_updated_timestamp = reward_last_updated_timestamp;
        if is_token_fee_in_a {
            // Add fees taken via a
            self.fee_growth_global_a = fee_growth_global;
            self.protocol_fee_owed_a += protocol_fee;
        } else {
            // Add fees taken via b
            self.fee_growth_global_b = fee_growth_global;
            self.protocol_fee_owed_b += protocol_fee;
        }
    }

    pub fn update_fee_rate(&mut self, fee_rate: u16) -> Result<()> {
        if fee_rate > MAX_FEE_RATE {
            return Err(DualTokenVoterError::FeeRateMaxExceeded.into());
        }
        self.fee_rate = fee_rate;

        Ok(())
    }

    pub fn update_protocol_fee_rate(&mut self, protocol_fee_rate: u16) -> Result<()> {
        if protocol_fee_rate > MAX_PROTOCOL_FEE_RATE {
            return Err(DualTokenVoterError::ProtocolFeeRateMaxExceeded.into());
        }
        self.protocol_fee_rate = protocol_fee_rate;

        Ok(())
    }

    pub fn reset_protocol_fees_owed(&mut self) {
        self.protocol_fee_owed_a = 0;
        self.protocol_fee_owed_b = 0;
    }

    pub fn fee_tier_index(&self) -> u16 {
        u16::from_le_bytes(self.fee_tier_index_seed)
    }

    pub fn is_initialized_with_adaptive_fee_tier(&self) -> bool {
        self.fee_tier_index() != self.tick_spacing
    }

    pub fn is_non_transferable_position_required(&self) -> bool {
        // Only newly created pools and migrated pools have control_flags
        // TODO: remove this check once all whirlpools have been migrated
        if self.reward_infos[2].extension != [0u8; 32] {
            return false;
        }

        self.extension_segment_primary()
            .control_flags()
            .contains(WhirlpoolControlFlags::REQUIRE_NON_TRANSFERABLE_POSITION)
    }

    pub fn is_position_with_token_extensions_required(&self) -> bool {
        if self.is_non_transferable_position_required() {
            return true;
        }

        false
    }
}

/// Stores the state relevant for tracking liquidity mining rewards at the `Whirlpool` level.
/// These values are used in conjunction with `PositionRewardInfo`, `Tick.reward_growths_outside`,
/// and `Whirlpool.reward_last_updated_timestamp` to determine how many rewards are earned by open
/// positions.
#[zero_copy(unsafe)]
#[derive(Default, Debug)]
pub struct WhirlpoolRewardInfo {
    /// Reward token mint.
    pub mint: Pubkey,
    /// Reward vault token account.
    pub vault: Pubkey,
    /// reward_infos[0]: Authority account that has permission to initialize the reward and set emissions.
    /// reward_infos[1]: used for a struct that contains fields for extending the functionality of Whirlpool.
    /// reward_infos[2]: reserved for future use.
    ///
    /// Historical notes:
    /// Originally, this was a field named "authority", but it was found that there was no opportunity
    /// to set different authorities for the three rewards. Therefore, the use of this field was changed for Whirlpool's future extensibility.
    pub extension: [u8; 32],
    /// Q64.64 number that indicates how many tokens per second are earned per unit of liquidity.
    pub emissions_per_second_x64: u128,
    /// Q64.64 number that tracks the total tokens earned per unit of liquidity since the reward
    /// emissions were turned on.
    pub growth_global_x64: u128,
}

impl WhirlpoolRewardInfo {
    /// Creates a new `WhirlpoolRewardInfo` with the extension set
    pub fn new(extension: [u8; 32]) -> Self {
        Self {
            extension,
            ..Default::default()
        }
    }

    /// Returns true if this reward is initialized.
    /// Once initialized, a reward cannot transition back to uninitialized.
    pub fn initialized(&self) -> bool {
        self.mint.ne(&Pubkey::default())
    }

    /// Maps all reward data to only the reward growth accumulators
    pub fn to_reward_growths(
        reward_infos: &[WhirlpoolRewardInfo; NUM_REWARDS],
    ) -> [u128; NUM_REWARDS] {
        let mut reward_growths = [0u128; NUM_REWARDS];
        for i in 0..NUM_REWARDS {
            reward_growths[i] = reward_infos[i].growth_global_x64;
        }
        reward_growths
    }
}

#[derive(Copy, Clone, Default, Debug, PartialEq)]
pub struct WhirlpoolControlFlags(u16);

bitflags! {
    impl WhirlpoolControlFlags: u16 {
        const REQUIRE_NON_TRANSFERABLE_POSITION = 0b0000_0000_0000_0001;
    }
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, PartialEq)]
pub struct WhirlpoolExtensionSegmentPrimary {
    // total length must be 32 bytes
    pub control_flags: u16,
    pub reserved: [u8; 30],
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, PartialEq)]
pub struct WhirlpoolExtensionSegmentSecondary {
    // total length must be 32 bytes
    pub reserved: [u8; 32],
}

impl WhirlpoolExtensionSegmentPrimary {
    pub fn to_bytes(&self) -> [u8; 32] {
        // We can ensure that the serialization will always produce 32 bytes
        self.try_to_vec()
            .expect("Failed to serialize WhirlpoolExtensionSegmentPrimary")
            .try_into()
            .expect("Serialized data length mismatch")
    }

    pub fn new(control_flags: WhirlpoolControlFlags) -> Self {
        Self {
            control_flags: control_flags.bits(),
            reserved: [0; 30],
        }
    }

    pub fn control_flags(&self) -> WhirlpoolControlFlags {
        WhirlpoolControlFlags::from_bits_truncate(self.control_flags)
    }
}

impl WhirlpoolExtensionSegmentSecondary {
    pub fn to_bytes(&self) -> [u8; 32] {
        // We can ensure that the serialization will always produce 32 bytes
        self.try_to_vec()
            .expect("Failed to serialize WhirlpoolExtensionSegmentSecondary")
            .try_into()
            .expect("Serialized data length mismatch")
    }

    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self { reserved: [0; 32] }
    }
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Default, Copy)]
pub struct WhirlpoolBumps {
    pub whirlpool_bump: u8,
}
