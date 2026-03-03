use anchor_lang::prelude::*;
use mpl_core::types::PluginAuthority;

use crate::error::CoreNftAttributeVoterError;

/// Configuration of an NFT collection used for attribute-based governance power
#[derive(AnchorSerialize, AnchorDeserialize, Debug, Clone, PartialEq)]
pub struct CollectionConfig {
    /// The NFT collection used for governance
    pub collection: Pubkey,

    /// The size of the NFT collection used to calculate max voter weight
    pub size: u32,

    /// Maximum governance power weight of the collection (ceiling for max voter weight calculation)
    /// Note: The weight is scaled accordingly to the governing_token_mint decimals
    pub max_weight: u64,

    /// The attribute key to read the voting weight from on each NFT
    /// The attribute value must be a valid u64 string
    pub weight_attribute_key: String,

    /// The expected plugin authority for the Attributes plugin on each NFT.
    /// Only attributes set by this authority are trusted for vote weight.
    pub expected_attribute_authority: PluginAuthority,

    /// Reserved for future upgrades
    pub reserved: [u8; 8],
}

impl CollectionConfig {
    /// Borsh serialized size: 32 (Pubkey) + 4 (u32) + 8 (u64) + 4+32 (String with max 32 chars) + 33 (PluginAuthority) + 8 (reserved)
    pub const SERIALIZED_SIZE: usize = 32 + 4 + 8 + 36 + 33 + 8;

    pub fn get_max_weight(&self) -> Result<u64> {
        (self.size as u64)
            .checked_mul(self.max_weight)
            .ok_or_else(|| CoreNftAttributeVoterError::ArithmeticOverflow.into())
    }
}

impl Default for CollectionConfig {
    fn default() -> Self {
        Self {
            collection: Pubkey::default(),
            size: 0,
            max_weight: 0,
            // Default to a 32-byte zero-padded string for deterministic sizing
            weight_attribute_key: "\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0".to_string(),
            expected_attribute_authority: PluginAuthority::Address { address: Pubkey::default() },
            reserved: [0; 8],
        }
    }
}
