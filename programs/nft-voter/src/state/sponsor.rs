use anchor_lang::prelude::*;

use crate::id;

/// Returns Sponsor PDA seeds
/// The sponsor is a SystemAccount PDA that holds SOL to pay for NFT vote record creation
pub fn get_sponsor_seeds<'a>(registrar: &'a Pubkey) -> [&'a [u8]; 2] {
    [b"sponsor", registrar.as_ref()]
}

/// Returns Sponsor PDA address
pub fn get_sponsor_address(registrar: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(&get_sponsor_seeds(registrar), &id()).0
}
