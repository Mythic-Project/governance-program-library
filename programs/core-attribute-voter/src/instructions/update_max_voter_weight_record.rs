use crate::error::CoreNftAttributeVoterError;
use crate::state::*;
use anchor_lang::prelude::*;
use max_voter_weight_record::MaxVoterWeightRecord;

// Takes all collections added to `register`, iterates over them and calculates
// the max voter weight
#[derive(Accounts)]
pub struct UpdateMaxVoterWeightRecord<'info> {
    /// The NFT voting Registrar
    pub registrar: Account<'info, Registrar>,

    #[account(
        mut,
        constraint = max_voter_weight_record.realm == registrar.realm
        @ CoreNftAttributeVoterError::InvalidVoterWeightRecordRealm,

        constraint = max_voter_weight_record.governing_token_mint == registrar.governing_token_mint
        @ CoreNftAttributeVoterError::InvalidVoterWeightRecordMint,
    )]
    pub max_voter_weight_record: Account<'info, MaxVoterWeightRecord>,
}

pub fn update_max_voter_weight_record(ctx: Context<UpdateMaxVoterWeightRecord>) -> Result<()> {
    let registrar = &ctx.accounts.registrar;

    // Calculate the max voter weight by iterating over all collections and summing
    // the max weight of each collection.
    ctx.accounts.max_voter_weight_record.max_voter_weight = registrar.max_voter_weight()?;

    // Record is only valid as of the current slot
    let slot = Clock::get()?.slot;
    msg!("Clock: {:?}", slot);

    ctx.accounts.max_voter_weight_record.max_voter_weight_expiry = Some(slot);

    Ok(())
}
