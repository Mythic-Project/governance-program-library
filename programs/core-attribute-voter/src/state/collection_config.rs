use anchor_lang::prelude::*;
use mpl_core::types::PluginAuthority;

/// Configuration of an NFT collection used for attribute-based governance power
#[derive(AnchorSerialize, AnchorDeserialize, Debug, Clone, PartialEq)]
pub struct CollectionConfig {
    /// The NFT collection used for governance
    pub collection: Pubkey,

    /// Maximum governance power weight of the collection
    /// Any NFT whose configured attribute value exceeds this cap is limited to this value.
    pub max_weight: u64,

    /// Total governance power contribution of the collection to quorum denominator.
    /// This value is summed across collections to produce MaxVoterWeightRecord.max_voter_weight.
    pub total_weight: u64,

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
    /// Borsh serialized size: 32 (Pubkey) + 8 (max_weight) + 8 (total_weight)
    /// + 4+32 (String with max 32 chars) + 33 (PluginAuthority) + 8 (reserved)
    pub const SERIALIZED_SIZE: usize = 32 + 8 + 8 + 36 + 33 + 8;

}

impl Default for CollectionConfig {
    fn default() -> Self {
        Self {
            collection: Pubkey::default(),
            max_weight: 0,
            total_weight: 0,
            // Default to a 32-byte zero-padded string for deterministic sizing
            weight_attribute_key: "\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0".to_string(),
            expected_attribute_authority: PluginAuthority::Address { address: Pubkey::default() },
            reserved: [0; 8],
        }
    }
}
