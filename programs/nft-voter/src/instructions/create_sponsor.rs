use crate::error::NftVoterError;
use crate::state::*;
use anchor_lang::prelude::*;
use anchor_lang::solana_program::{program::invoke, system_instruction};
use spl_governance::state::realm;

/// Creates/validates a Sponsor PDA for holding SOL to pay NFT vote record rent on behalf of voters
/// The sponsor is a SystemAccount PDA - this instruction validates realm authority
#[derive(Accounts)]
pub struct CreateSponsor<'info> {
    /// The NFT voting Registrar
    pub registrar: Account<'info, Registrar>,

    /// The Sponsor PDA - a system-owned account that holds SOL for sponsored voting
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

    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
}

/// Creates the Sponsor PDA by funding it with the rent-exempt minimum
/// The sponsor is a SystemAccount PDA that holds SOL for sponsored voting
pub fn create_sponsor(ctx: Context<CreateSponsor>) -> Result<()> {
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
        NftVoterError::InvalidRealmAuthority
    );

    // Fund the sponsor PDA with rent-exempt minimum so it exists on-chain
    let rent = Rent::get()?;
    let min_balance = rent.minimum_balance(0);

    if ctx.accounts.sponsor.lamports() < min_balance {
        let needed = min_balance.saturating_sub(ctx.accounts.sponsor.lamports());
        invoke(
            &system_instruction::transfer(
                ctx.accounts.payer.key,
                ctx.accounts.sponsor.key,
                needed,
            ),
            &[
                ctx.accounts.payer.to_account_info(),
                ctx.accounts.sponsor.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
            ],
        )?;
    }

    Ok(())
}
