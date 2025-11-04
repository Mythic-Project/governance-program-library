use {
    crate::{state::*},
    anchor_lang::prelude::*,
};

/// Creates VoterWeightRecord and Voter accounts for a user
/// This instruction should only be executed once per realm/governing_token_mint/governing_token_owner
#[derive(Accounts)]
pub struct CreateVoterWeightRecord<'info> {
    /// The Registrar the VoterWeightRecord account belongs to
    pub registrar: Box<Account<'info, Registrar>>,

    /// Voter account stores deposit tracking information
    #[account(
        init,
        seeds = [b"voter", registrar.key().as_ref(), voter_authority.key().as_ref()],
        bump,
        payer = payer,
        space = 8 + std::mem::size_of::<Voter>()
    )]
    pub voter: Box<Account<'info, Voter>>,

    /// VoterWeightRecord account is the SPL Governance interface account
    #[account(
        init,
        seeds = [
            b"voter-weight-record",
            registrar.key().as_ref(),
            voter_authority.key().as_ref()
        ],
        bump,
        payer = payer,
        space = VoterWeightRecord::get_space()
    )]
    pub voter_weight_record: Box<Account<'info, VoterWeightRecord>>,

    /// The authority (owner) of the voter account
    pub voter_authority: Signer<'info>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
}

/// Creates both Voter and VoterWeightRecord accounts for a user
pub fn create_voter_weight_record(
    ctx: Context<CreateVoterWeightRecord>,
) -> Result<()> {
    let registrar = &ctx.accounts.registrar;
    let voter = &mut ctx.accounts.voter;
    let voter_weight_record = &mut ctx.accounts.voter_weight_record;

    // Initialize Voter account
    voter.voter_authority = ctx.accounts.voter_authority.key();
    voter.registrar = registrar.key();
    voter.token_a_deposited_amount = 0;
    voter.token_b_deposited_amount = 0;
    voter.tokens_update_timestamp = 0;
    voter.voter_bump = ctx.bumps.voter;
    voter.voter_weight_record_bump = ctx.bumps.voter_weight_record;

    // Initialize VoterWeightRecord
    voter_weight_record.realm = registrar.realm;
    voter_weight_record.governing_token_mint = registrar.realm_governing_token_mint;
    voter_weight_record.governing_token_owner = ctx.accounts.voter_authority.key();
    voter_weight_record.voter_weight = 0;
    voter_weight_record.voter_weight_expiry = Some(0);
    voter_weight_record.weight_action = None;
    voter_weight_record.weight_action_target = None;
    voter_weight_record.reserved = [0; 8];

    Ok(())
}
