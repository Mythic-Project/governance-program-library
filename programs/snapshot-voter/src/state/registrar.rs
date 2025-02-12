use crate::{
    id, max_voter_weight_record,
    vote_weight_record,
};
use anchor_lang::{prelude::*, Discriminator};
use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
use solana_program::pubkey::PUBKEY_BYTES;



// Generate a VoteWeightRecord & MaxVoterWeightRecord Anchor wrapper, owned by the current program.
// VoteWeightRecords & MaxVoterWeightRecord are unique in that they are defined by the SPL governance
// program, but they are actually owned by this program.
vote_weight_record!(crate::ID);
max_voter_weight_record!(crate::ID);

/// Enum defining collection item change type
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
pub enum CollectionItemChangeType {
    /// Update item in the collection if it already exists and Insert the item if it doesn't
    Upsert,
    /// Remove item from the collection
    Remove,
}

/// Registrar which stores spl-governance configurations for the given Realm
#[account]
#[derive(Debug, PartialEq)]
pub struct Registrar {
    /// spl-governance program the Realm belongs to
    pub governance_program_id: Pubkey,

    /// Realm of the Registrar
    pub realm: Pubkey,

    /// Governing token mint the Registrar is for
    /// It can either be the Community or the Council mint of the Realm
    /// When the plugin is enabled the mint is only used as the identity of the governing power (voting population)
    /// and the actual token of the mint is not used
    pub governing_token_mint: Pubkey,

    pub root: [u8; 32],
    pub uri: Option<String>,
    pub proposal: Pubkey,
}

impl Registrar {
    pub fn get_space(uri: Option<String>) -> usize {
        Registrar::discriminator().len() + PUBKEY_BYTES * 3 + 32 + uri.as_ref().map_or(0, |u| u.len())
    }
}

/// Returns Registrar PDA seeds
pub fn get_registrar_seeds<'a>(
    realm: &'a Pubkey,
    governing_token_mint: &'a Pubkey,
) -> [&'a [u8]; 3] {
    [b"registrar", realm.as_ref(), governing_token_mint.as_ref()]
}

/// Returns Registrar PDA address
pub fn get_registrar_address(realm: &Pubkey, governing_token_mint: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(&get_registrar_seeds(realm, governing_token_mint), &id()).0
}

#[cfg(test)]
mod test {

    use super::*;

    #[test]
    fn test_get_space() {
        // Arrange
        let expected_space = Registrar::get_space(None);

        let registrar = Registrar {
            governance_program_id: Pubkey::default(),
            realm: Pubkey::default(),
            governing_token_mint: Pubkey::default(),
            root: [0; 32],
            uri: None,
            proposal: Pubkey::default(),
        };

        // Act
        let actual_space = Registrar::discriminator().len() + registrar.try_to_vec().unwrap().len();

        // Assert
        assert_eq!(expected_space, actual_space);
    }
}
