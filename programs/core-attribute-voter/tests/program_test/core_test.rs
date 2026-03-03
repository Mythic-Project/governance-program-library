use std::{str::FromStr, sync::Arc};

use anchor_lang::prelude::Pubkey;
use solana_program_test::ProgramTest;
use solana_sdk::{signature::Keypair, signer::Signer, system_program, transport::TransportError};

use crate::program_test::program_test_bench::{ProgramTestBench, WalletCookie};

pub struct AssetCookie {
    pub asset: Pubkey,
}

pub struct CollectionCookie {
    pub collection: Pubkey,
    pub authority: Keypair,
}

pub struct CoreTest {
    pub bench: Arc<ProgramTestBench>,
    pub program_id: Pubkey,
}

impl CoreTest {
    pub fn program_id() -> Pubkey {
        Pubkey::from_str("CoREENxT6tW1HoK8ypY1SxRMZTcVPm7R94rH4PZNhX7d").unwrap()
    }

    #[allow(dead_code)]
    pub fn add_program(program_test: &mut ProgramTest) {
        program_test.add_program("mpl_core", Self::program_id(), None);
    }

    #[allow(dead_code)]
    pub fn new(bench: Arc<ProgramTestBench>) -> Self {
        CoreTest {
            bench,
            program_id: Self::program_id(),
        }
    }

    #[allow(dead_code)]
    pub async fn create_collection(
        &self,
        collection_size: Option<u64>,
    ) -> Result<CollectionCookie, TransportError> {
        let update_authority = self.bench.context.borrow().payer.pubkey();
        let payer = self.bench.context.borrow().payer.pubkey();

        // Create collection
        let coll_keypair = Keypair::new();
        let coll_authority = Keypair::new();

        let coll_name = "NFT_C".to_string();
        let coll_uri = "URI".to_string();

        // instruction args
        let args = mpl_core::instructions::CreateCollectionV2InstructionArgs {
            name: coll_name,
            uri: coll_uri,
            plugins: None,
            external_plugin_adapters: None,
        };

        // instruction accounts
        let create_coll_ix_accounts = mpl_core::instructions::CreateCollectionV2 {
            collection: coll_keypair.pubkey(),
            update_authority: Some(update_authority),
            payer,
            system_program: system_program::ID,
        };

        // creates the instruction
        let create_coll_ix = create_coll_ix_accounts.instruction(args);

        self.bench
            .process_transaction(&[create_coll_ix], Some(&[&coll_keypair]))
            .await?;

        if let Some(size) = collection_size {
            println!("Minting {} assets to collection", size);
            self.mint_assets_to_collection(
                &CollectionCookie {
                    collection: coll_keypair.pubkey(),
                    authority: coll_authority.insecure_clone(),
                },
                size,
            )
            .await;
        }

        Ok(CollectionCookie {
            collection: coll_keypair.pubkey(),
            authority: coll_authority,
        })
    }

    #[allow(dead_code)]
    pub async fn create_asset(
        &self,
        collection_cookie: &CollectionCookie,
        asset_owner_cookie: &WalletCookie,
    ) -> Result<AssetCookie, TransportError> {
        self.create_asset_with_attributes(collection_cookie, asset_owner_cookie, None)
            .await
    }

    #[allow(dead_code)]
    pub async fn create_asset_with_attributes(
        &self,
        collection_cookie: &CollectionCookie,
        asset_owner_cookie: &WalletCookie,
        attributes: Option<Vec<mpl_core::types::Attribute>>,
    ) -> Result<AssetCookie, TransportError> {
        let collection_authority = self.bench.context.borrow().payer.pubkey();
        let payer = self.bench.context.borrow().payer.pubkey();

        // Create Asset
        let asset_keypair = Keypair::new();

        let name = "TestAsset".to_string();
        let uri = "URI".to_string();

        let plugins = attributes.map(|attrs| {
            vec![mpl_core::types::PluginAuthorityPair {
                plugin: mpl_core::types::Plugin::Attributes(mpl_core::types::Attributes {
                    attribute_list: attrs,
                }),
                authority: Some(mpl_core::types::PluginAuthority::UpdateAuthority),
            }]
        });

        // instruction args
        let args = mpl_core::instructions::CreateV2InstructionArgs {
            data_state: mpl_core::types::DataState::AccountState,
            name,
            uri,
            plugins,
            external_plugin_adapters: None,
        };

        // instruction accounts
        let create_accounts = mpl_core::instructions::CreateV2 {
            asset: asset_keypair.pubkey(),
            collection: Some(collection_cookie.collection),
            authority: Some(collection_authority),
            payer,
            owner: Some(asset_owner_cookie.address),
            update_authority: None,
            system_program: system_program::ID,
            log_wrapper: None,
        };

        // creates the instruction
        let create_ix = create_accounts.instruction(args);

        self.bench
            .process_transaction(&[create_ix], Some(&[&asset_keypair]))
            .await?;

        Ok(AssetCookie {
            asset: asset_keypair.pubkey(),
        })
    }

    #[allow(dead_code)]
    pub async fn create_asset_with_weight(
        &self,
        collection_cookie: &CollectionCookie,
        asset_owner_cookie: &WalletCookie,
        weight: u64,
    ) -> Result<AssetCookie, TransportError> {
        self.create_asset_with_attributes(
            collection_cookie,
            asset_owner_cookie,
            Some(vec![mpl_core::types::Attribute {
                key: "weight".to_string(),
                value: weight.to_string(),
            }]),
        )
        .await
    }

    #[allow(dead_code)]
    pub async fn create_asset_with_named_weight(
        &self,
        collection_cookie: &CollectionCookie,
        asset_owner_cookie: &WalletCookie,
        weight_key: &str,
        weight_value: &str,
    ) -> Result<AssetCookie, TransportError> {
        self.create_asset_with_attributes(
            collection_cookie,
            asset_owner_cookie,
            Some(vec![mpl_core::types::Attribute {
                key: weight_key.to_string(),
                value: weight_value.to_string(),
            }]),
        )
        .await
    }

    pub async fn mint_assets_to_collection(&self, collection_cookie: &CollectionCookie, size: u64) {
        let asset_owner = self.bench.with_wallet().await;

        for _ in 0..size {
            let _ = self.create_asset(&collection_cookie, &asset_owner).await;
        }
    }
}
