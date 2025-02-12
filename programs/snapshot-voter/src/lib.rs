use anchor_lang::prelude::*;

pub mod error;

mod instructions;
use instructions::*;

pub mod state;

mod governance;
pub mod tools;

#[macro_use]
extern crate static_assertions;

declare_id!("75vZfHMhAnDvh7wrKCSPvabK1RLTMh6jezoL53nxNqmc");

#[program]
pub mod realm_voter {

    use super::*;

    pub fn create_registrar(ctx: Context<CreateRegistrar>) -> Result<()> {
        log_version();
        instructions::create_registrar(ctx)
    }

    pub fn update_registrar(
        ctx: Context<UpdateRegistrar>,
        root: [u8; 32],
        uri: Option<String>,
    ) -> Result<()> {
        log_version();
        instructions::update_registrar(ctx, root, uri)
    }

    pub fn create_voter_weight_record(
        ctx: Context<CreateVoterWeightRecord>,
        governing_token_owner: Pubkey,
    ) -> Result<()> {
        log_version();
        instructions::create_voter_weight_record(ctx, governing_token_owner)
    }

    pub fn create_max_voter_weight_record(ctx: Context<CreateMaxVoterWeightRecord>) -> Result<()> {
        log_version();
        instructions::create_max_voter_weight_record(ctx)
    }

    pub fn update_voter_weight_record(
        ctx: Context<UpdateVoterWeightRecord>,
        amount: u64,
        verification_data: Vec<u8>,
    ) -> Result<()> {
        log_version();
        instructions::update_voter_weight_record(ctx, amount, verification_data)
    }
}

fn log_version() {
    // TODO: Check if Anchor allows to log it before instruction is deserialized
    msg!("VERSION:{:?}", env!("CARGO_PKG_VERSION"));
}
