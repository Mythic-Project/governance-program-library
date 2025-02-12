use {
    crate::{error::*, state::*},
    anchor_lang::prelude::*,
    anchor_spl::token_interface::Mint,
    spl_governance::state::{enums::ProposalState, proposal::get_proposal_data, realm},
};

/// Resizes Registrar storing Realm Voter configuration for spl-governance Realm
/// This instruction can only be ran if the max_mint is higher than currently used voting_mint_configs length
#[derive(Accounts)]
#[instruction(_root: [u8;32], uri: Option<String>)]
pub struct UpdateRegistrar<'info> {
    /// The Realm Voter Registrar
    /// There can only be a single registrar per governance Realm and governing mint of the Realm
    #[account(
        mut,
        seeds = [b"registrar".as_ref(), realm.key().as_ref(), governing_token_mint.key().as_ref()],
        bump,
        realloc = Registrar::get_space(uri.clone()),
        realloc::payer = payer,
        realloc::zero = false,
    )]
    pub registrar: Account<'info, Registrar>,

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
    ///
    /// Note: Once the Realm voter plugin is enabled the governing_token_mint is used only as identity
    /// for the voting population and the tokens of that are no longer used
    pub governing_token_mint: InterfaceAccount<'info, Mint>,

    /// realm_authority must sign and match Realm.authority
    pub realm_authority: Signer<'info>,

    #[account(mut)]
    pub payer: Signer<'info>,

    /// CHECK: Checked below.
    pub proposal: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

/// Updates a Registrar which stores Realms voter snapshot configuration for the given Realm
///
/// root can be updated as well as the uri which is an offchain reference of the root
pub fn update_registrar(
    ctx: Context<UpdateRegistrar>,
    root: [u8; 32],
    uri: Option<String>,
) -> Result<()> {
    let registrar = &mut ctx.accounts.registrar;

    registrar.root = root;
    registrar.uri = uri;
    registrar.proposal = ctx.accounts.proposal.key();

    let proposal = get_proposal_data(&registrar.governance_program_id, &ctx.accounts.proposal)?;

    if proposal.state != ProposalState::Draft {
        return Err(SnapshotVoterError::InvalidProposalState.into());
    }

    // Verify that realm_authority is the expected authority of the Realm
    // and that the mint matches one of the realm mints too
    let realm = realm::get_realm_data_for_governing_token_mint(
        &registrar.governance_program_id,
        &ctx.accounts.realm,
        &registrar.governing_token_mint,
    )?;

    require_eq!(
        realm.authority.unwrap(),
        ctx.accounts.realm_authority.key(),
        SnapshotVoterError::InvalidRealmAuthority
    );

    Ok(())
}
