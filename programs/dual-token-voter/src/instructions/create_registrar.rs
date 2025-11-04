use {
    crate::{error::*, state::*},
    anchor_lang::prelude::*,
    anchor_spl::token_interface::Mint,
    spl_governance::state::realm,
};

// Security bounds from audit
const MIN_ELIGIBILITY_WINDOW: i64 = 3600; // 1 hour
const MAX_ELIGIBILITY_WINDOW: i64 = 30 * 24 * 3600; // 30 days

/// Creates Registrar storing dual-token-voter configuration for spl-governance Realm
/// This instruction should only be executed once per realm/governing_token_mint to create the account
#[derive(Accounts)]
pub struct CreateRegistrar<'info> {
    /// The dual-token-voter Registrar
    /// There can only be a single registrar per governance Realm and governing mint of the Realm
    #[account(
        init,
        seeds = [b"registrar", realm.key().as_ref(), governing_token_mint.key().as_ref()],
        bump,
        payer = payer,
        space = 8 + std::mem::size_of::<Registrar>()
    )]
    pub registrar: Box<Account<'info, Registrar>>,

    /// The program id of the spl-governance program the realm belongs to
    /// CHECK: Can be any instance of spl-governance and it's not known at the compilation time
    #[account(executable)]
    pub governance_program_id: UncheckedAccount<'info>,

    /// An spl-governance Realm
    ///
    /// Realm is validated in the instruction:
    /// - Realm is owned by the governance_program_id
    /// - governing_token_mint must be the community or council mint
    /// - realm_authority is realm.authority
    ///
    /// CHECK: Owned by spl-governance instance specified in governance_program_id
    #[account(owner = governance_program_id.key())]
    pub realm: UncheckedAccount<'info>,

    /// Either the realm community mint or the council mint.
    /// It must match Realm.community_mint or Realm.config.council_mint
    pub governing_token_mint: Box<InterfaceAccount<'info, Mint>>,

    /// The first token (token_a) mint that users can deposit for voting power
    pub token_a_mint: Box<InterfaceAccount<'info, Mint>>,

    /// The second token (token_b) mint that users can deposit for voting power (converted via exchange rate)
    pub token_b_mint: Box<InterfaceAccount<'info, Mint>>,

    /// realm_authority must sign and match Realm.authority
    pub realm_authority: Signer<'info>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
}

/// Creates a new Registrar which stores dual-token-voter configuration for the given Realm
///
/// eligibility_window_seconds defines how long before proposal creation a deposit must occur
/// to be eligible for voting (e.g., 259200 = 72 hours)
pub fn create_registrar(
    ctx: Context<CreateRegistrar>,
    eligibility_window_seconds: i64,
) -> Result<()> {
    let registrar = &mut ctx.accounts.registrar;

    // Security validation: Eligibility window bounds
    require!(
        eligibility_window_seconds >= MIN_ELIGIBILITY_WINDOW,
        DualTokenVoterError::EligibilityWindowTooSmall
    );
    require!(
        eligibility_window_seconds <= MAX_ELIGIBILITY_WINDOW,
        DualTokenVoterError::EligibilityWindowTooLarge
    );

    // Security validation: token_a and token_b cannot be the same
    require_keys_neq!(
        ctx.accounts.token_a_mint.key(),
        ctx.accounts.token_b_mint.key(),
        DualTokenVoterError::SameTokenMints
    );

    registrar.governance_program_id = ctx.accounts.governance_program_id.key();
    registrar.realm = ctx.accounts.realm.key();
    registrar.realm_governing_token_mint = ctx.accounts.governing_token_mint.key();
    registrar.token_a_mint = ctx.accounts.token_a_mint.key();
    registrar.token_b_mint = ctx.accounts.token_b_mint.key();
    registrar.eligibility_window_seconds = eligibility_window_seconds;
    registrar.registrar_bump = ctx.bumps.registrar;

    // Verify realm_authority matches the realm
    let realm = realm::get_realm_data_for_governing_token_mint(
        &registrar.governance_program_id,
        &ctx.accounts.realm,
        &registrar.realm_governing_token_mint,
    )?;

    // Security fix: Check realm.authority exists before unwrap
    require!(
        realm.authority.is_some(),
        DualTokenVoterError::InvalidRealmAuthority
    );

    require_eq!(
        realm.authority.unwrap(),
        ctx.accounts.realm_authority.key(),
        DualTokenVoterError::InvalidRealmAuthority
    );

    Ok(())
}
