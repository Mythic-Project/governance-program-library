use crate::error::NftVoterError;
use crate::state::*;
use anchor_lang::prelude::*;
use anchor_lang::solana_program::{program::invoke_signed, system_instruction};
use spl_governance::state::realm;

/// Withdraws SOL from the Sponsor PDA
/// Only the realm authority can withdraw funds
#[derive(Accounts)]
pub struct WithdrawSponsor<'info> {
    /// The NFT voting Registrar
    pub registrar: Account<'info, Registrar>,

    /// The Sponsor PDA to withdraw from
    #[account(
        mut,
        seeds = [b"sponsor".as_ref(), registrar.key().as_ref()],
        bump,
    )]
    pub sponsor: SystemAccount<'info>,

    /// An spl-governance Realm
    ///
    /// CHECK: Owned by spl-governance instance specified in registrar.governance_program_id
    #[account(
        owner = registrar.governance_program_id,
        constraint = registrar.realm == realm.key() @ NftVoterError::InvalidRealmForRegistrar
    )]
    pub realm: UncheckedAccount<'info>,

    /// realm_authority must sign and match Realm.authority
    pub realm_authority: Signer<'info>,

    /// The account to receive the withdrawn funds
    /// CHECK: Can be any account to receive the lamports
    #[account(mut)]
    pub destination: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

/// Withdraws SOL from the sponsor PDA to a destination
/// Only the realm authority can withdraw
pub fn withdraw_sponsor(ctx: Context<WithdrawSponsor>, amount: u64) -> Result<()> {
    let registrar = &ctx.accounts.registrar;

    // Verify that realm_authority is the expected authority of the Realm
    let realm = realm::get_realm_data_for_governing_token_mint(
        &registrar.governance_program_id,
        &ctx.accounts.realm,
        &registrar.governing_token_mint,
    )?;

    let realm_authority = realm
        .authority
        .ok_or(NftVoterError::InvalidRealmAuthority)?;

    require!(
        realm_authority == ctx.accounts.realm_authority.key(),
        NftVoterError::InvalidSponsorAuthority
    );

    // Check sponsor has enough funds
    let rent = Rent::get()?;
    let min_balance = rent.minimum_balance(0);
    let available = ctx.accounts.sponsor.lamports().saturating_sub(min_balance);

    require!(
        amount <= available,
        NftVoterError::InsufficientSponsorFunds
    );

    // Transfer via system program CPI with PDA signer
    let registrar_key = registrar.key();
    let sponsor_seeds = get_sponsor_seeds(&registrar_key);
    let (_, sponsor_bump) = Pubkey::find_program_address(&sponsor_seeds, &crate::id());
    let signer_seeds: &[&[u8]] = &[b"sponsor", registrar_key.as_ref(), &[sponsor_bump]];

    invoke_signed(
        &system_instruction::transfer(
            ctx.accounts.sponsor.key,
            ctx.accounts.destination.key,
            amount,
        ),
        &[
            ctx.accounts.sponsor.to_account_info(),
            ctx.accounts.destination.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
        ],
        &[signer_seeds],
    )?;

    Ok(())
}
