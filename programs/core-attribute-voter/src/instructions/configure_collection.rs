use anchor_lang::{
    account,
    prelude::{Context, Signer},
    Accounts,
};

use anchor_lang::prelude::*;
use mpl_core::accounts::BaseCollectionV1;
use mpl_core::types::PluginAuthority;
use spl_governance::state::realm;

use crate::error::CoreNftAttributeVoterError;
use crate::state::{max_voter_weight_record::MaxVoterWeightRecord, CollectionConfig, Registrar};

/// Configures NFT voting collection which defines what NFTs can be used for governances
/// and what weight they have (read from per-NFT attributes)
/// The instruction updates MaxVoterWeightRecord which is used by spl-gov to determine max voting power
/// used to calculate voting quorum
#[derive(Accounts)]
pub struct ConfigureCollection<'info> {
    /// Registrar for which we configure this Collection
    #[account(mut)]
    pub registrar: Account<'info, Registrar>,

    #[account(
       address = registrar.realm @ CoreNftAttributeVoterError::InvalidRealmForRegistrar,
       owner = registrar.governance_program_id
    )]
    /// CHECK: Owned by spl-governance instance specified in registrar.governance_program_id
    pub realm: UncheckedAccount<'info>,

    /// Authority of the Realm must sign and match Realm.authority
    pub realm_authority: Signer<'info>,

    // Collection which is going to be used for voting
    pub collection: Account<'info, BaseCollectionV1>,

    #[account(
        mut,
        constraint = max_voter_weight_record.realm == registrar.realm
        @ CoreNftAttributeVoterError::InvalidMaxVoterWeightRecordRealm,

        constraint = max_voter_weight_record.governing_token_mint == registrar.governing_token_mint
        @ CoreNftAttributeVoterError::InvalidMaxVoterWeightRecordMint,
    )]
    pub max_voter_weight_record: Account<'info, MaxVoterWeightRecord>,
}

pub fn configure_collection(
    ctx: Context<ConfigureCollection>,
    max_weight: u64,
    weight_attribute_key: String,
    expected_attribute_authority: PluginAuthority,
) -> Result<()> {
    let collection_key = ctx.accounts.collection.key();
    let registrar = &mut ctx.accounts.registrar;

    let realm = realm::get_realm_data_for_governing_token_mint(
        &registrar.governance_program_id,
        &ctx.accounts.realm,
        &registrar.governing_token_mint,
    )?;

    require!(
        realm.authority == Some(ctx.accounts.realm_authority.key()),
        CoreNftAttributeVoterError::InvalidRealmAuthority
    );

    require!(
        max_weight > 0,
        CoreNftAttributeVoterError::InvalidMaxWeight
    );

    // Validate weight_attribute_key
    require!(
        !weight_attribute_key.is_empty() && weight_attribute_key.len() <= 32,
        CoreNftAttributeVoterError::InvalidWeightAttributeKey
    );

    let collection_config = CollectionConfig {
        collection: collection_key,
        max_weight,
        weight_attribute_key,
        expected_attribute_authority,
        reserved: [0; 8],
    };

    let collection_idx = registrar
        .collection_configs
        .iter()
        .position(|cc| cc.collection == collection_key);

    if let Some(collection_idx) = collection_idx {
        registrar.collection_configs[collection_idx] = collection_config;
    } else {
        // Note: In the current runtime version push() would throw an error if we exceed
        // max_collections specified when the Registrar was created
        registrar.collection_configs.push(collection_config);
    }

    // Update MaxVoterWeightRecord based on max voting power of the collections
    let max_voter_weight_record = &mut ctx.accounts.max_voter_weight_record;

    max_voter_weight_record.max_voter_weight = registrar.max_voter_weight()?;

    // The weight never expires and only changes when collections are configured
    max_voter_weight_record.max_voter_weight_expiry = None;

    Ok(())
}
