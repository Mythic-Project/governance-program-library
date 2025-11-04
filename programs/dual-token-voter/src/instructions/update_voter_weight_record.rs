use {
    crate::{error::*, state::*},
    anchor_lang::prelude::*,
};

/// Updates the voter's weight based on deposited token_a and token_b amounts
/// Requires exchange_rate_snapshot to already exist (created via create_exchange_rate_snapshot)
/// Standalone plugin - does not compose with other plugins
#[derive(Accounts)]
#[instruction(voter_weight_action: VoterWeightAction, proposal: Pubkey)]
pub struct UpdateVoterWeightRecord<'info> {
    pub registrar: Account<'info, Registrar>,

    #[account(
        seeds = [b"voter", registrar.key().as_ref(), voter_authority.key().as_ref()],
        bump = voter.voter_bump,
    )]
    pub voter: Account<'info, Voter>,

    #[account(
        mut,
        seeds = [
            b"voter-weight-record",
            registrar.key().as_ref(),
            voter_authority.key().as_ref()
        ],
        bump = voter.voter_weight_record_bump,
        constraint = voter_weight_record.realm == registrar.realm
            @ DualTokenVoterError::InvalidSnapshot,
        constraint = voter_weight_record.governing_token_mint == registrar.realm_governing_token_mint
            @ DualTokenVoterError::InvalidSnapshot,
    )]
    pub voter_weight_record: Account<'info, VoterWeightRecord>,

    /// Exchange rate snapshot for this proposal (must already exist)
    #[account(
        seeds = [b"exchange-snapshot", registrar.key().as_ref(), proposal.as_ref()],
        bump = exchange_rate_snapshot.bump,
        constraint = exchange_rate_snapshot.proposal == proposal
            @ DualTokenVoterError::InvalidSnapshot,
    )]
    pub exchange_rate_snapshot: Account<'info, ExchangeRateSnapshot>,

    pub voter_authority: Signer<'info>,
}

/// Updates voter weight record based on token_a and token_b deposits
/// Uses existing exchange rate snapshot to calculate weight
///
/// Two behaviors:
/// - CreateProposal: Calculates weight using snapshot (no eligibility check)
/// - CastVote: Calculates weight using snapshot with eligibility window enforcement
pub fn update_voter_weight_record(
    ctx: Context<UpdateVoterWeightRecord>,
    voter_weight_action: VoterWeightAction,
    proposal: Pubkey,
) -> Result<()> {
    let clock = Clock::get()?;
    let registrar = &ctx.accounts.registrar;
    let voter = &ctx.accounts.voter;
    let voter_weight_record = &mut ctx.accounts.voter_weight_record;
    let snapshot = &ctx.accounts.exchange_rate_snapshot;

    // Calculate voting weight: token_a (1:1) + token_b (via exchange rate from snapshot)
    let token_b_weighted = voter
        .token_b_deposited_amount
        .checked_mul(snapshot.token_b_exchange_rate)
        .ok_or(DualTokenVoterError::Overflow)?
        .checked_div(1_000_000_000)
        .ok_or(DualTokenVoterError::DivisionError)?;

    let total_weight = voter
        .token_a_deposited_amount
        .checked_add(token_b_weighted)
        .ok_or(DualTokenVoterError::Overflow)?;

    match voter_weight_action {
        VoterWeightAction::CreateProposal => {
            // No eligibility check for proposal creation
            voter_weight_record.voter_weight = total_weight;
        },

        VoterWeightAction::CastVote => {
            // Check eligibility: tokens_update_timestamp must be before (proposal_creation - eligibility_window)
            let proposal_creation_time = snapshot.snapshot_timestamp;
            let eligibility_cutoff = proposal_creation_time
                .checked_sub(registrar.eligibility_window_seconds)
                .ok_or(DualTokenVoterError::MathOverflow)?;

            require!(
                voter.tokens_update_timestamp <= eligibility_cutoff,
                DualTokenVoterError::NotEligibleToVote
            );

            voter_weight_record.voter_weight = total_weight;
        },

        _ => {
            return Err(DualTokenVoterError::UnsupportedAction.into());
        }
    }

    // Set standard SPL Governance fields
    voter_weight_record.weight_action = Some(voter_weight_action);
    voter_weight_record.weight_action_target = Some(proposal);
    voter_weight_record.voter_weight_expiry = Some(clock.slot);

    Ok(())
}
