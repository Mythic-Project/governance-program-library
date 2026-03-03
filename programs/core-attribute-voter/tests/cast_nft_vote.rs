use crate::program_test::core_voter_test::ConfigureCollectionArgs;
use gpl_core_attribute_voter::error::CoreNftAttributeVoterError;
use gpl_core_attribute_voter::state::*;
use program_test::{
    core_voter_test::*,
    tools::{assert_gov_err, assert_nft_voter_err},
};

use solana_program_test::*;
use solana_sdk::transport::TransportError;
use spl_governance::error::GovernanceError;

mod program_test;

#[tokio::test]
async fn test_cast_asset_vote() -> Result<(), TransportError> {
    // Arrange
    let mut core_voter_test = CoreVoterTest::start_new().await;

    let realm_cookie = core_voter_test.governance.with_realm().await?;

    let registrar_cookie = core_voter_test.with_registrar(&realm_cookie).await?;

    let collection_cookie = core_voter_test.core.create_collection(None).await?;

    let max_voter_weight_record_cookie = core_voter_test
        .with_max_voter_weight_record(&registrar_cookie)
        .await?;

    let voter_cookie = core_voter_test.bench.with_wallet().await;

    let asset_cookie1 = core_voter_test
        .core.create_asset_with_weight(&collection_cookie, &voter_cookie, 10)
        .await?;

    core_voter_test
        .with_collection(
            &registrar_cookie,
            &collection_cookie,
            &max_voter_weight_record_cookie,
            Some(ConfigureCollectionArgs {
                max_weight: 10,
                ..Default::default()
            }),
        )
        .await?;

    let voter_token_owner_record_cookie = core_voter_test
        .governance
        .with_token_owner_record(&realm_cookie, &voter_cookie)
        .await?;

    let voter_weight_record_cookie = core_voter_test
        .with_voter_weight_record(&registrar_cookie, &voter_cookie)
        .await?;

    let proposal_cookie = core_voter_test
        .governance
        .with_proposal(&realm_cookie)
        .await?;

    core_voter_test.bench.advance_clock().await;
    let clock = core_voter_test.bench.get_clock().await;

    // Act
    let asset_vote_record_cookies = core_voter_test
        .cast_asset_vote(
            &registrar_cookie,
            &voter_weight_record_cookie,
            &max_voter_weight_record_cookie,
            &proposal_cookie,
            &voter_cookie,
            &voter_token_owner_record_cookie,
            &[&asset_cookie1],
            None,
        )
        .await?;

    // Assert
    let asset_vote_record = core_voter_test
        .get_asset_vote_record_account(&asset_vote_record_cookies[0].address)
        .await;

    assert_eq!(asset_vote_record_cookies[0].account, asset_vote_record);

    let voter_weight_record = core_voter_test
        .get_voter_weight_record(&voter_weight_record_cookie.address)
        .await;

    assert_eq!(voter_weight_record.voter_weight, 10);
    assert_eq!(voter_weight_record.voter_weight_expiry, Some(clock.slot));
    assert_eq!(
        voter_weight_record.weight_action,
        Some(VoterWeightAction::CastVote.into())
    );
    assert_eq!(
        voter_weight_record.weight_action_target,
        Some(proposal_cookie.address)
    );

    Ok(())
}

#[tokio::test]
async fn test_cast_asset_vote_with_multiple_nfts() -> Result<(), TransportError> {
    // Arrange
    let mut core_voter_test = CoreVoterTest::start_new().await;

    let realm_cookie = core_voter_test.governance.with_realm().await?;

    let registrar_cookie = core_voter_test.with_registrar(&realm_cookie).await?;

    let collection_cookie = core_voter_test.core.create_collection(None).await?;

    let max_voter_weight_record_cookie = core_voter_test
        .with_max_voter_weight_record(&registrar_cookie)
        .await?;

    let voter_cookie = core_voter_test.bench.with_wallet().await;

    let asset_cookie1 = core_voter_test
        .core.create_asset_with_weight(&collection_cookie, &voter_cookie, 10)
        .await?;

    let asset_cookie2 = core_voter_test
        .core.create_asset_with_weight(&collection_cookie, &voter_cookie, 10)
        .await?;

    core_voter_test
        .with_collection(
            &registrar_cookie,
            &collection_cookie,
            &max_voter_weight_record_cookie,
            Some(ConfigureCollectionArgs {
                max_weight: 10,
                ..Default::default()
            }),
        )
        .await?;

    let voter_token_owner_record_cookie = core_voter_test
        .governance
        .with_token_owner_record(&realm_cookie, &voter_cookie)
        .await?;

    let voter_weight_record_cookie = core_voter_test
        .with_voter_weight_record(&registrar_cookie, &voter_cookie)
        .await?;

    let proposal_cookie = core_voter_test
        .governance
        .with_proposal(&realm_cookie)
        .await?;

    core_voter_test.bench.advance_clock().await;
    let clock = core_voter_test.bench.get_clock().await;

    // Act
    let asset_vote_record_cookies = core_voter_test
        .cast_asset_vote(
            &registrar_cookie,
            &voter_weight_record_cookie,
            &max_voter_weight_record_cookie,
            &proposal_cookie,
            &voter_cookie,
            &voter_token_owner_record_cookie,
            &[&asset_cookie1, &asset_cookie2],
            None,
        )
        .await?;

    // Assert
    let asset_vote_record1 = core_voter_test
        .get_asset_vote_record_account(&asset_vote_record_cookies[0].address)
        .await;

    assert_eq!(asset_vote_record_cookies[0].account, asset_vote_record1);

    let asset_vote_record2 = core_voter_test
        .get_asset_vote_record_account(&asset_vote_record_cookies[1].address)
        .await;

    assert_eq!(asset_vote_record_cookies[1].account, asset_vote_record2);

    let voter_weight_record = core_voter_test
        .get_voter_weight_record(&voter_weight_record_cookie.address)
        .await;

    assert_eq!(voter_weight_record.voter_weight, 20);
    assert_eq!(voter_weight_record.voter_weight_expiry, Some(clock.slot));
    assert_eq!(
        voter_weight_record.weight_action,
        Some(VoterWeightAction::CastVote.into())
    );
    assert_eq!(
        voter_weight_record.weight_action_target,
        Some(proposal_cookie.address)
    );

    Ok(())
}

#[tokio::test]
async fn test_cast_asset_vote_with_nft_already_voted_error() -> Result<(), TransportError> {
    // Arrange
    let mut core_voter_test = CoreVoterTest::start_new().await;

    let realm_cookie = core_voter_test.governance.with_realm().await?;

    let registrar_cookie = core_voter_test.with_registrar(&realm_cookie).await?;

    let collection_cookie = core_voter_test.core.create_collection(None).await?;

    let max_voter_weight_record_cookie = core_voter_test
        .with_max_voter_weight_record(&registrar_cookie)
        .await?;

    let voter_cookie: program_test::program_test_bench::WalletCookie = core_voter_test.bench.with_wallet().await;

    let asset_cookie1 = core_voter_test
        .core.create_asset_with_weight(&collection_cookie, &voter_cookie, 1)
        .await?;

    core_voter_test
        .with_collection(
            &registrar_cookie,
            &collection_cookie,
            &max_voter_weight_record_cookie,
            None,
        )
        .await?;

    let voter_token_owner_record_cookie = core_voter_test
        .governance
        .with_token_owner_record(&realm_cookie, &voter_cookie)
        .await?;

    let voter_weight_record_cookie = core_voter_test
        .with_voter_weight_record(&registrar_cookie, &voter_cookie)
        .await?;

    let proposal_cookie = core_voter_test
        .governance
        .with_proposal(&realm_cookie)
        .await?;

    core_voter_test
        .cast_asset_vote(
            &registrar_cookie,
            &voter_weight_record_cookie,
            &max_voter_weight_record_cookie,
            &proposal_cookie,
            &voter_cookie,
            &voter_token_owner_record_cookie,
            &[&asset_cookie1],
            Some(CastAssetVoteArgs {
                cast_spl_gov_vote: false,
            }),
        )
        .await?;

    core_voter_test.bench.advance_clock().await;

    // Act
    let err = core_voter_test
        .cast_asset_vote(
            &registrar_cookie,
            &voter_weight_record_cookie,
            &max_voter_weight_record_cookie,
            &proposal_cookie,
            &voter_cookie,
            &voter_token_owner_record_cookie,
            &[&asset_cookie1],
            Some(CastAssetVoteArgs {
                cast_spl_gov_vote: false,
            }),
        )
        .await
        .err()
        .unwrap();

    // Assert
    assert_nft_voter_err(err, CoreNftAttributeVoterError::NftAlreadyVoted);

    Ok(())
}

#[tokio::test]
async fn test_cast_asset_vote_with_invalid_voter_error() -> Result<(), TransportError> {
    // Arrange
    let mut core_voter_test = CoreVoterTest::start_new().await;

    let realm_cookie = core_voter_test.governance.with_realm().await?;

    let registrar_cookie = core_voter_test.with_registrar(&realm_cookie).await?;

    let collection_cookie = core_voter_test.core.create_collection(None).await?;

    let max_voter_weight_record_cookie = core_voter_test
        .with_max_voter_weight_record(&registrar_cookie)
        .await?;

    let voter_cookie = core_voter_test.bench.with_wallet().await;

    let asset_cookie1 = core_voter_test
        .core.create_asset_with_weight(&collection_cookie, &voter_cookie, 1)
        .await?;

    core_voter_test
        .with_collection(
            &registrar_cookie,
            &collection_cookie,
            &max_voter_weight_record_cookie,
            None,
        )
        .await?;

    let voter_token_owner_record_cookie = core_voter_test
        .governance
        .with_token_owner_record(&realm_cookie, &voter_cookie)
        .await?;

    let voter_weight_record_cookie = core_voter_test
        .with_voter_weight_record(&registrar_cookie, &voter_cookie)
        .await?;

    let proposal_cookie = core_voter_test
        .governance
        .with_proposal(&realm_cookie)
        .await?;

    let voter_cookie2 = core_voter_test.bench.with_wallet().await;

    // Act

    let err = core_voter_test
        .cast_asset_vote(
            &registrar_cookie,
            &voter_weight_record_cookie,
            &max_voter_weight_record_cookie,
            &proposal_cookie,
            &voter_cookie2,
            &voter_token_owner_record_cookie,
            &[&asset_cookie1],
            None,
        )
        .await
        .err()
        .unwrap();

    // Assert
    assert_gov_err(err, GovernanceError::GoverningTokenOwnerOrDelegateMustSign);

    Ok(())
}

#[tokio::test]
async fn test_cast_asset_vote_with_invalid_owner_error() -> Result<(), TransportError> {
    // Arrange
    let mut core_voter_test = CoreVoterTest::start_new().await;

    let realm_cookie = core_voter_test.governance.with_realm().await?;

    let registrar_cookie = core_voter_test.with_registrar(&realm_cookie).await?;

    let collection_cookie = core_voter_test.core.create_collection(None).await?;

    let max_voter_weight_record_cookie = core_voter_test
        .with_max_voter_weight_record(&registrar_cookie)
        .await?;

    let voter_cookie = core_voter_test.bench.with_wallet().await;

    let voter_cookie2 = core_voter_test.bench.with_wallet().await;

    let asset_cookie = core_voter_test
        .core.create_asset_with_weight(&collection_cookie, &voter_cookie2, 10)
        .await?;

    core_voter_test
        .with_collection(
            &registrar_cookie,
            &collection_cookie,
            &max_voter_weight_record_cookie,
            Some(ConfigureCollectionArgs {
                max_weight: 10,
                ..Default::default()
            }),
        )
        .await?;


    let voter_token_owner_record_cookie = core_voter_test
        .governance
        .with_token_owner_record(&realm_cookie, &voter_cookie)
        .await?;

    let voter_weight_record_cookie = core_voter_test
        .with_voter_weight_record(&registrar_cookie, &voter_cookie)
        .await?;

    let proposal_cookie = core_voter_test
        .governance
        .with_proposal(&realm_cookie)
        .await?;

    // Act
    let err = core_voter_test
        .cast_asset_vote(
            &registrar_cookie,
            &voter_weight_record_cookie,
            &max_voter_weight_record_cookie,
            &proposal_cookie,
            &voter_cookie,
            &voter_token_owner_record_cookie,
            &[&asset_cookie],
            None,
        )
        .await
        .err()
        .unwrap();

    // Assert
    assert_nft_voter_err(err, CoreNftAttributeVoterError::VoterDoesNotOwnNft);

    Ok(())
}

#[tokio::test]
async fn test_cast_asset_vote_with_invalid_collection_error() -> Result<(), TransportError> {
    // Arrange
    let mut core_voter_test = CoreVoterTest::start_new().await;

    let realm_cookie: program_test::governance_test::RealmCookie = core_voter_test.governance.with_realm().await?;

    let registrar_cookie = core_voter_test.with_registrar(&realm_cookie).await?;

    let collection_cookie = core_voter_test.core.create_collection(None).await?;

    let collection_cookie2 = core_voter_test.core.create_collection(None).await?;

    let max_voter_weight_record_cookie = core_voter_test
        .with_max_voter_weight_record(&registrar_cookie)
        .await?;

    let voter_cookie: program_test::program_test_bench::WalletCookie = core_voter_test.bench.with_wallet().await;

    let _random_asset_cookie = core_voter_test
        .core.create_asset_with_weight(&collection_cookie, &voter_cookie, 10)
        .await?;

    let asset_cookie = core_voter_test
        .core.create_asset_with_weight(&collection_cookie2, &voter_cookie, 10)
        .await?;

    core_voter_test
        .with_collection(
            &registrar_cookie,
            &collection_cookie,
            &max_voter_weight_record_cookie,
            Some(ConfigureCollectionArgs {
                max_weight: 10,
                ..Default::default()
            }),
        )
        .await?;

    let proposal_cookie = core_voter_test
        .governance
        .with_proposal(&realm_cookie)
        .await?;


    let voter_token_owner_record_cookie = core_voter_test
        .governance
        .with_token_owner_record(&realm_cookie, &voter_cookie)
        .await?;

    let voter_weight_record_cookie = core_voter_test
        .with_voter_weight_record(&registrar_cookie, &voter_cookie)
        .await?;

    // Act
    let err = core_voter_test
        .cast_asset_vote(
            &registrar_cookie,
            &voter_weight_record_cookie,
            &max_voter_weight_record_cookie,
            &proposal_cookie,
            &voter_cookie,
            &voter_token_owner_record_cookie,
            &[&asset_cookie],
            None,
        )
        .await
        .err()
        .unwrap();

    // Assert
    assert_nft_voter_err(err, CoreNftAttributeVoterError::CollectionNotFound);

    Ok(())
}

#[tokio::test]
async fn test_cast_asset_vote_with_same_nft_error() -> Result<(), TransportError> {
    // Arrange
    let mut core_voter_test = CoreVoterTest::start_new().await;

    let realm_cookie = core_voter_test.governance.with_realm().await?;

    let registrar_cookie = core_voter_test.with_registrar(&realm_cookie).await?;

    let collection_cookie = core_voter_test.core.create_collection(None).await?;

    let max_voter_weight_record_cookie = core_voter_test
        .with_max_voter_weight_record(&registrar_cookie)
        .await?;

    let voter_cookie = core_voter_test.bench.with_wallet().await;

    let asset_cookie = core_voter_test
        .core.create_asset_with_weight(&collection_cookie, &voter_cookie, 1)
        .await?;

    core_voter_test
        .with_collection(
            &registrar_cookie,
            &collection_cookie,
            &max_voter_weight_record_cookie,
            None,
        )
        .await?;

    let proposal_cookie = core_voter_test
        .governance
        .with_proposal(&realm_cookie)
        .await?;

    let voter_token_owner_record_cookie = core_voter_test
        .governance
        .with_token_owner_record(&realm_cookie, &voter_cookie)
        .await?;

    let voter_weight_record_cookie = core_voter_test
        .with_voter_weight_record(&registrar_cookie, &voter_cookie)
        .await?;

    // Act
    let err = core_voter_test
        .cast_asset_vote(
            &registrar_cookie,
            &voter_weight_record_cookie,
            &max_voter_weight_record_cookie,
            &proposal_cookie,
            &voter_cookie,
            &voter_token_owner_record_cookie,
            &[&asset_cookie, &asset_cookie],
            None,
        )
        .await
        .err()
        .unwrap();

    // Assert

    assert_nft_voter_err(err, CoreNftAttributeVoterError::DuplicatedNftDetected);

    Ok(())
}

#[tokio::test]
async fn test_cast_asset_vote_with_max_5_nfts() -> Result<(), TransportError> {
    // Arrange
    let mut core_voter_test = CoreVoterTest::start_new().await;

    let realm_cookie = core_voter_test.governance.with_realm().await?;

    let registrar_cookie = core_voter_test.with_registrar(&realm_cookie).await?;

    let collection_cookie = core_voter_test.core.create_collection(None).await?;

    let max_voter_weight_record_cookie = core_voter_test
        .with_max_voter_weight_record(&registrar_cookie)
        .await?;

    let voter_cookie = core_voter_test.bench.with_wallet().await;

    let mut asset_cookies = vec![];

    for _ in 0..5 {
        core_voter_test.bench.advance_clock().await;
        let asset_cookie = core_voter_test
            .core
            .create_asset_with_weight(&collection_cookie, &voter_cookie, 10)
            .await?;

        asset_cookies.push(asset_cookie)
    }

    core_voter_test
        .with_collection(
            &registrar_cookie,
            &collection_cookie,
            &max_voter_weight_record_cookie,
            Some(ConfigureCollectionArgs {
                max_weight: 10,
                ..Default::default()
            }),
        )
        .await?;

    let voter_token_owner_record_cookie = core_voter_test
        .governance
        .with_token_owner_record(&realm_cookie, &voter_cookie)
        .await?;

    let voter_weight_record_cookie = core_voter_test
        .with_voter_weight_record(&registrar_cookie, &voter_cookie)
        .await?;

    let proposal_cookie = core_voter_test
        .governance
        .with_proposal(&realm_cookie)
        .await?;

    core_voter_test.bench.advance_clock().await;
    let clock = core_voter_test.bench.get_clock().await;

    // Act
    let asset_vote_record_cookies = core_voter_test
        .cast_asset_vote(
            &registrar_cookie,
            &voter_weight_record_cookie,
            &max_voter_weight_record_cookie,
            &proposal_cookie,
            &voter_cookie,
            &voter_token_owner_record_cookie,
            &asset_cookies.iter().collect::<Vec<_>>(),
            None,
        )
        .await?;

    // Assert
    let asset_vote_record1 = core_voter_test
        .get_asset_vote_record_account(&asset_vote_record_cookies[0].address)
        .await;

    assert_eq!(asset_vote_record_cookies[0].account, asset_vote_record1);

    let asset_vote_record2 = core_voter_test
        .get_asset_vote_record_account(&asset_vote_record_cookies[1].address)
        .await;

    assert_eq!(asset_vote_record_cookies[1].account, asset_vote_record2);

    let voter_weight_record = core_voter_test
        .get_voter_weight_record(&voter_weight_record_cookie.address)
        .await;

    assert_eq!(voter_weight_record.voter_weight, 50);
    assert_eq!(voter_weight_record.voter_weight_expiry, Some(clock.slot));
    assert_eq!(
        voter_weight_record.weight_action,
        Some(VoterWeightAction::CastVote.into())
    );
    assert_eq!(
        voter_weight_record.weight_action_target,
        Some(proposal_cookie.address)
    );

    Ok(())
}

#[tokio::test]
async fn test_cast_asset_vote_using_multiple_instructions() -> Result<(), TransportError> {
    // Arrange
    let mut core_voter_test = CoreVoterTest::start_new().await;

    let realm_cookie = core_voter_test.governance.with_realm().await?;

    let registrar_cookie = core_voter_test.with_registrar(&realm_cookie).await?;

    let collection_cookie = core_voter_test.core.create_collection(None).await?;

    let max_voter_weight_record_cookie = core_voter_test
        .with_max_voter_weight_record(&registrar_cookie)
        .await?;

    let voter_cookie = core_voter_test.bench.with_wallet().await;

    let asset_cookie1 = core_voter_test
        .core.create_asset_with_weight(&collection_cookie, &voter_cookie, 10)
        .await?;

    let asset_cookie2 = core_voter_test
        .core.create_asset_with_weight(&collection_cookie, &voter_cookie, 10)
        .await?;

    core_voter_test
        .with_collection(
            &registrar_cookie,
            &collection_cookie,
            &max_voter_weight_record_cookie,
            Some(ConfigureCollectionArgs {
                max_weight: 10,
                ..Default::default()
            }),
        )
        .await?;

    let voter_token_owner_record_cookie = core_voter_test
        .governance
        .with_token_owner_record(&realm_cookie, &voter_cookie)
        .await?;

    let voter_weight_record_cookie = core_voter_test
        .with_voter_weight_record(&registrar_cookie, &voter_cookie)
        .await?;

    let proposal_cookie = core_voter_test
        .governance
        .with_proposal(&realm_cookie)
        .await?;

    core_voter_test.bench.advance_clock().await;
    let clock = core_voter_test.bench.get_clock().await;

    let args = CastAssetVoteArgs {
        cast_spl_gov_vote: false,
    };

    core_voter_test
        .cast_asset_vote(
            &registrar_cookie,
            &voter_weight_record_cookie,
            &max_voter_weight_record_cookie,
            &proposal_cookie,
            &voter_cookie,
            &voter_token_owner_record_cookie,
            &[&asset_cookie1],
            Some(args),
        )
        .await?;

    // Act
    core_voter_test
        .cast_asset_vote(
            &registrar_cookie,
            &voter_weight_record_cookie,
            &max_voter_weight_record_cookie,
            &proposal_cookie,
            &voter_cookie,
            &voter_token_owner_record_cookie,
            &[&asset_cookie2],
            None,
        )
        .await?;

    // Assert

    let voter_weight_record = core_voter_test
        .get_voter_weight_record(&voter_weight_record_cookie.address)
        .await;

    assert_eq!(voter_weight_record.voter_weight, 20);
    assert_eq!(voter_weight_record.voter_weight_expiry, Some(clock.slot));
    assert_eq!(
        voter_weight_record.weight_action,
        Some(VoterWeightAction::CastVote.into())
    );
    assert_eq!(
        voter_weight_record.weight_action_target,
        Some(proposal_cookie.address)
    );

    Ok(())
}

#[tokio::test]
async fn test_cast_asset_vote_using_multiple_instructions_with_nft_already_voted_error() -> Result<(), TransportError> {
    // Arrange
    let mut core_voter_test = CoreVoterTest::start_new().await;

    let realm_cookie = core_voter_test.governance.with_realm().await?;

    let registrar_cookie = core_voter_test.with_registrar(&realm_cookie).await?;

    let collection_cookie = core_voter_test.core.create_collection(None).await?;

    let max_voter_weight_record_cookie = core_voter_test
        .with_max_voter_weight_record(&registrar_cookie)
        .await?;

    let voter_cookie = core_voter_test.bench.with_wallet().await;

    let asset_cookie1 = core_voter_test
        .core.create_asset_with_weight(&collection_cookie, &voter_cookie, 10)
        .await?;

    core_voter_test
        .with_collection(
            &registrar_cookie,
            &collection_cookie,
            &max_voter_weight_record_cookie,
            Some(ConfigureCollectionArgs {
                max_weight: 10,
                ..Default::default()
            }),
        )
        .await?;

    let voter_token_owner_record_cookie = core_voter_test
        .governance
        .with_token_owner_record(&realm_cookie, &voter_cookie)
        .await?;

    let voter_weight_record_cookie = core_voter_test
        .with_voter_weight_record(&registrar_cookie, &voter_cookie)
        .await?;

    let proposal_cookie = core_voter_test
        .governance
        .with_proposal(&realm_cookie)
        .await?;

    let args = CastAssetVoteArgs {
        cast_spl_gov_vote: false,
    };

    core_voter_test
        .cast_asset_vote(
            &registrar_cookie,
            &voter_weight_record_cookie,
            &max_voter_weight_record_cookie,
            &proposal_cookie,
            &voter_cookie,
            &voter_token_owner_record_cookie,
            &[&asset_cookie1],
            Some(args),
        )
        .await?;

    // Act
    let err = core_voter_test
        .cast_asset_vote(
            &registrar_cookie,
            &voter_weight_record_cookie,
            &max_voter_weight_record_cookie,
            &proposal_cookie,
            &voter_cookie,
            &voter_token_owner_record_cookie,
            &[&asset_cookie1],
            None,
        )
        .await
        .err()
        .unwrap();

    // Assert
    assert_nft_voter_err(err, CoreNftAttributeVoterError::NftAlreadyVoted);

    Ok(())
}

#[tokio::test]
async fn test_cast_asset_vote_using_multiple_instructions_with_attempted_sandwiched_relinquish() -> Result<(), TransportError> {
    // Arrange
    let mut core_voter_test = CoreVoterTest::start_new().await;

    let realm_cookie = core_voter_test.governance.with_realm().await?;

    let registrar_cookie = core_voter_test.with_registrar(&realm_cookie).await?;

    let collection_cookie = core_voter_test.core.create_collection(None).await?;

    let max_voter_weight_record_cookie = core_voter_test
        .with_max_voter_weight_record(&registrar_cookie)
        .await?;

    let voter_cookie = core_voter_test.bench.with_wallet().await;

    let asset_cookie1 = core_voter_test
        .core.create_asset_with_weight(&collection_cookie, &voter_cookie, 10)
        .await?;

    core_voter_test
        .with_collection(
            &registrar_cookie,
            &collection_cookie,
            &max_voter_weight_record_cookie,
            Some(ConfigureCollectionArgs {
                max_weight: 10,
                ..Default::default()
            }),
        )
        .await?;

    let voter_token_owner_record_cookie = core_voter_test
        .governance
        .with_token_owner_record(&realm_cookie, &voter_cookie)
        .await?;

    let voter_weight_record_cookie = core_voter_test
        .with_voter_weight_record(&registrar_cookie, &voter_cookie)
        .await?;

    let proposal_cookie = core_voter_test
        .governance
        .with_proposal(&realm_cookie)
        .await?;

    let args = CastAssetVoteArgs {
        cast_spl_gov_vote: false,
    };

    // Cast vote with NFT
    let asset_vote_record_cookies = core_voter_test
        .cast_asset_vote(
            &registrar_cookie,
            &voter_weight_record_cookie,
            &max_voter_weight_record_cookie,
            &proposal_cookie,
            &voter_cookie,
            &voter_token_owner_record_cookie,
            &[&asset_cookie1],
            Some(args),
        )
        .await?;

    core_voter_test.bench.advance_clock().await;

    // Try relinquish NftVoteRecords to accumulate vote
    core_voter_test
        .relinquish_nft_vote(
            &registrar_cookie,
            &voter_weight_record_cookie,
            &proposal_cookie,
            &voter_cookie,
            &voter_token_owner_record_cookie,
            &asset_vote_record_cookies,
        )
        .await?;

    // Act

    core_voter_test
        .cast_asset_vote(
            &registrar_cookie,
            &voter_weight_record_cookie,
            &max_voter_weight_record_cookie,
            &proposal_cookie,
            &voter_cookie,
            &voter_token_owner_record_cookie,
            &[&asset_cookie1],
            None,
        )
        .await?;

    // Assert

    let voter_weight_record = core_voter_test
        .get_voter_weight_record(&voter_weight_record_cookie.address)
        .await;

    assert_eq!(voter_weight_record.voter_weight, 10);

    Ok(())
}

#[tokio::test]
async fn test_cast_asset_vote_using_delegate() -> Result<(), TransportError> {
    // Arrange
    let mut core_voter_test = CoreVoterTest::start_new().await;

    let realm_cookie = core_voter_test.governance.with_realm().await?;

    let registrar_cookie = core_voter_test.with_registrar(&realm_cookie).await?;

    let collection_cookie = core_voter_test.core.create_collection(None).await?;

    let max_voter_weight_record_cookie = core_voter_test
        .with_max_voter_weight_record(&registrar_cookie)
        .await?;

    let voter_cookie = core_voter_test.bench.with_wallet().await;

    let asset_cookie1 = core_voter_test
        .core.create_asset_with_weight(&collection_cookie, &voter_cookie, 1)
        .await?;

    core_voter_test
        .with_collection(
            &registrar_cookie,
            &collection_cookie,
            &max_voter_weight_record_cookie,
            None,
        )
        .await?;

    let voter_token_owner_record_cookie = core_voter_test
        .governance
        .with_token_owner_record(&realm_cookie, &voter_cookie)
        .await?;

    let voter_weight_record_cookie = core_voter_test
        .with_voter_weight_record(&registrar_cookie, &voter_cookie)
        .await?;

    let proposal_cookie = core_voter_test
        .governance
        .with_proposal(&realm_cookie)
        .await?;

    core_voter_test.bench.advance_clock().await;

    let delegate_cookie = core_voter_test.bench.with_wallet().await;
    core_voter_test
        .governance
        .set_governance_delegate(
            &realm_cookie,
            &voter_token_owner_record_cookie,
            &voter_cookie,
            &Some(delegate_cookie.address),
        )
        .await;

    // Act
    let asset_vote_record_cookies = core_voter_test
        .cast_asset_vote(
            &registrar_cookie,
            &voter_weight_record_cookie,
            &max_voter_weight_record_cookie,
            &proposal_cookie,
            &delegate_cookie,
            &voter_token_owner_record_cookie,
            &[&asset_cookie1],
            None,
        )
        .await?;

    // Assert
    let asset_vote_record = core_voter_test
        .get_asset_vote_record_account(&asset_vote_record_cookies[0].address)
        .await;

    assert_eq!(asset_vote_record_cookies[0].account, asset_vote_record);

    Ok(())
}

#[tokio::test]
async fn test_cast_asset_vote_with_invalid_voter_weight_token_owner_error() -> Result<(), TransportError> {
    // Arrange
    let mut core_voter_test = CoreVoterTest::start_new().await;

    let realm_cookie = core_voter_test.governance.with_realm().await?;

    let registrar_cookie = core_voter_test.with_registrar(&realm_cookie).await?;

    let collection_cookie = core_voter_test.core.create_collection(None).await?;

    let max_voter_weight_record_cookie = core_voter_test
        .with_max_voter_weight_record(&registrar_cookie)
        .await?;

    let voter_cookie = core_voter_test.bench.with_wallet().await;

    let asset_cookie1 = core_voter_test
        .core
        .create_asset_with_weight(&collection_cookie, &voter_cookie, 1)
        .await?;

    core_voter_test
        .with_collection(
            &registrar_cookie,
            &collection_cookie,
            &max_voter_weight_record_cookie,
            None,
        )
        .await?;

    let voter_token_owner_record_cookie = core_voter_test
        .governance
        .with_token_owner_record(&realm_cookie, &voter_cookie)
        .await?;

    // Try to update VoterWeightRecord for different governing_token_owner
    let voter_cookie2 = core_voter_test.bench.with_wallet().await;

    let voter_weight_record_cookie2 = core_voter_test
        .with_voter_weight_record(&registrar_cookie, &voter_cookie2)
        .await?;

    let proposal_cookie = core_voter_test
        .governance
        .with_proposal(&realm_cookie)
        .await?;

    // Act

    let err = core_voter_test
        .cast_asset_vote(
            &registrar_cookie,
            &voter_weight_record_cookie2,
            &max_voter_weight_record_cookie,
            &proposal_cookie,
            &voter_cookie,
            &voter_token_owner_record_cookie,
            &[&asset_cookie1],
            None,
        )
        .await
        .err()
        .unwrap();

    // Assert
    assert_nft_voter_err(err, CoreNftAttributeVoterError::InvalidTokenOwnerForVoterWeightRecord);

    Ok(())
}

#[tokio::test]
async fn test_cast_asset_vote_with_weight_capped_to_max() -> Result<(), TransportError> {
    // Arrange
    let mut core_voter_test = CoreVoterTest::start_new().await;

    let realm_cookie = core_voter_test.governance.with_realm().await?;

    let registrar_cookie = core_voter_test.with_registrar(&realm_cookie).await?;

    let collection_cookie = core_voter_test.core.create_collection(None).await?;

    let max_voter_weight_record_cookie = core_voter_test
        .with_max_voter_weight_record(&registrar_cookie)
        .await?;

    let voter_cookie = core_voter_test.bench.with_wallet().await;

    // Create an NFT with weight 100 but configure collection with max_weight 5
    let asset_cookie = core_voter_test
        .core
        .create_asset_with_weight(&collection_cookie, &voter_cookie, 100)
        .await?;

    core_voter_test
        .with_collection(
            &registrar_cookie,
            &collection_cookie,
            &max_voter_weight_record_cookie,
            Some(ConfigureCollectionArgs {
                max_weight: 5,
                ..Default::default()
            }),
        )
        .await?;

    let voter_token_owner_record_cookie = core_voter_test
        .governance
        .with_token_owner_record(&realm_cookie, &voter_cookie)
        .await?;

    let voter_weight_record_cookie = core_voter_test
        .with_voter_weight_record(&registrar_cookie, &voter_cookie)
        .await?;

    let proposal_cookie = core_voter_test
        .governance
        .with_proposal(&realm_cookie)
        .await?;

    core_voter_test.bench.advance_clock().await;

    // Act
    core_voter_test
        .cast_asset_vote(
            &registrar_cookie,
            &voter_weight_record_cookie,
            &max_voter_weight_record_cookie,
            &proposal_cookie,
            &voter_cookie,
            &voter_token_owner_record_cookie,
            &[&asset_cookie],
            None,
        )
        .await?;

    // Assert — weight should be capped to max_weight (5), not the NFT's actual weight (100)
    let voter_weight_record = core_voter_test
        .get_voter_weight_record(&voter_weight_record_cookie.address)
        .await;

    assert_eq!(voter_weight_record.voter_weight, 5);

    Ok(())
}

#[tokio::test]
async fn test_cast_asset_vote_with_nft_missing_attributes_plugin_error(
) -> Result<(), TransportError> {
    // Arrange
    let mut core_voter_test = CoreVoterTest::start_new().await;

    let realm_cookie = core_voter_test.governance.with_realm().await?;

    let registrar_cookie = core_voter_test.with_registrar(&realm_cookie).await?;

    let collection_cookie = core_voter_test.core.create_collection(None).await?;

    let max_voter_weight_record_cookie = core_voter_test
        .with_max_voter_weight_record(&registrar_cookie)
        .await?;

    let voter_cookie = core_voter_test.bench.with_wallet().await;

    // Create an NFT without any attributes plugin
    let asset_cookie = core_voter_test
        .core
        .create_asset(&collection_cookie, &voter_cookie)
        .await?;

    core_voter_test
        .with_collection(
            &registrar_cookie,
            &collection_cookie,
            &max_voter_weight_record_cookie,
            None,
        )
        .await?;

    let voter_token_owner_record_cookie = core_voter_test
        .governance
        .with_token_owner_record(&realm_cookie, &voter_cookie)
        .await?;

    let voter_weight_record_cookie = core_voter_test
        .with_voter_weight_record(&registrar_cookie, &voter_cookie)
        .await?;

    let proposal_cookie = core_voter_test
        .governance
        .with_proposal(&realm_cookie)
        .await?;

    // Act
    let err = core_voter_test
        .cast_asset_vote(
            &registrar_cookie,
            &voter_weight_record_cookie,
            &max_voter_weight_record_cookie,
            &proposal_cookie,
            &voter_cookie,
            &voter_token_owner_record_cookie,
            &[&asset_cookie],
            None,
        )
        .await
        .err()
        .unwrap();

    // Assert
    assert_nft_voter_err(err, CoreNftAttributeVoterError::AttributeNotFound);

    Ok(())
}

#[tokio::test]
async fn test_cast_asset_vote_with_missing_weight_attribute_error() -> Result<(), TransportError> {
    // Arrange
    let mut core_voter_test = CoreVoterTest::start_new().await;

    let realm_cookie = core_voter_test.governance.with_realm().await?;

    let registrar_cookie = core_voter_test.with_registrar(&realm_cookie).await?;

    let collection_cookie = core_voter_test.core.create_collection(None).await?;

    let max_voter_weight_record_cookie = core_voter_test
        .with_max_voter_weight_record(&registrar_cookie)
        .await?;

    let voter_cookie = core_voter_test.bench.with_wallet().await;

    // Create an NFT with a different attribute key than what the collection expects
    let asset_cookie = core_voter_test
        .core
        .create_asset_with_named_weight(
            &collection_cookie,
            &voter_cookie,
            "other_key",
            "10",
        )
        .await?;

    core_voter_test
        .with_collection(
            &registrar_cookie,
            &collection_cookie,
            &max_voter_weight_record_cookie,
            None,
        )
        .await?;

    let voter_token_owner_record_cookie = core_voter_test
        .governance
        .with_token_owner_record(&realm_cookie, &voter_cookie)
        .await?;

    let voter_weight_record_cookie = core_voter_test
        .with_voter_weight_record(&registrar_cookie, &voter_cookie)
        .await?;

    let proposal_cookie = core_voter_test
        .governance
        .with_proposal(&realm_cookie)
        .await?;

    // Act
    let err = core_voter_test
        .cast_asset_vote(
            &registrar_cookie,
            &voter_weight_record_cookie,
            &max_voter_weight_record_cookie,
            &proposal_cookie,
            &voter_cookie,
            &voter_token_owner_record_cookie,
            &[&asset_cookie],
            None,
        )
        .await
        .err()
        .unwrap();

    // Assert
    assert_nft_voter_err(err, CoreNftAttributeVoterError::AttributeNotFound);

    Ok(())
}

#[tokio::test]
async fn test_cast_asset_vote_with_invalid_attribute_value_error() -> Result<(), TransportError> {
    // Arrange
    let mut core_voter_test = CoreVoterTest::start_new().await;

    let realm_cookie = core_voter_test.governance.with_realm().await?;

    let registrar_cookie = core_voter_test.with_registrar(&realm_cookie).await?;

    let collection_cookie = core_voter_test.core.create_collection(None).await?;

    let max_voter_weight_record_cookie = core_voter_test
        .with_max_voter_weight_record(&registrar_cookie)
        .await?;

    let voter_cookie = core_voter_test.bench.with_wallet().await;

    // Create an NFT with a non-numeric weight attribute value
    let asset_cookie = core_voter_test
        .core
        .create_asset_with_named_weight(
            &collection_cookie,
            &voter_cookie,
            "weight",
            "not_a_number",
        )
        .await?;

    core_voter_test
        .with_collection(
            &registrar_cookie,
            &collection_cookie,
            &max_voter_weight_record_cookie,
            None,
        )
        .await?;

    let voter_token_owner_record_cookie = core_voter_test
        .governance
        .with_token_owner_record(&realm_cookie, &voter_cookie)
        .await?;

    let voter_weight_record_cookie = core_voter_test
        .with_voter_weight_record(&registrar_cookie, &voter_cookie)
        .await?;

    let proposal_cookie = core_voter_test
        .governance
        .with_proposal(&realm_cookie)
        .await?;

    // Act
    let err = core_voter_test
        .cast_asset_vote(
            &registrar_cookie,
            &voter_weight_record_cookie,
            &max_voter_weight_record_cookie,
            &proposal_cookie,
            &voter_cookie,
            &voter_token_owner_record_cookie,
            &[&asset_cookie],
            None,
        )
        .await
        .err()
        .unwrap();

    // Assert
    assert_nft_voter_err(err, CoreNftAttributeVoterError::InvalidAttributeValue);

    Ok(())
}

#[tokio::test]
async fn test_cast_asset_vote_with_multiple_collections() -> Result<(), TransportError> {
    // Arrange
    let mut core_voter_test = CoreVoterTest::start_new().await;

    let realm_cookie = core_voter_test.governance.with_realm().await?;

    let registrar_cookie = core_voter_test.with_registrar(&realm_cookie).await?;

    let collection_cookie1 = core_voter_test.core.create_collection(None).await?;
    let collection_cookie2 = core_voter_test.core.create_collection(None).await?;

    let max_voter_weight_record_cookie = core_voter_test
        .with_max_voter_weight_record(&registrar_cookie)
        .await?;

    let voter_cookie = core_voter_test.bench.with_wallet().await;

    // Create assets in different collections
    let asset_cookie1 = core_voter_test
        .core
        .create_asset_with_weight(&collection_cookie1, &voter_cookie, 10)
        .await?;

    let asset_cookie2 = core_voter_test
        .core
        .create_asset_with_weight(&collection_cookie2, &voter_cookie, 20)
        .await?;

    // Register both collections
    core_voter_test
        .with_collection(
            &registrar_cookie,
            &collection_cookie1,
            &max_voter_weight_record_cookie,
            Some(ConfigureCollectionArgs {
                max_weight: 10,
                ..Default::default()
            }),
        )
        .await?;

    core_voter_test
        .with_collection(
            &registrar_cookie,
            &collection_cookie2,
            &max_voter_weight_record_cookie,
            Some(ConfigureCollectionArgs {
                max_weight: 20,
                ..Default::default()
            }),
        )
        .await?;

    let voter_token_owner_record_cookie = core_voter_test
        .governance
        .with_token_owner_record(&realm_cookie, &voter_cookie)
        .await?;

    let voter_weight_record_cookie = core_voter_test
        .with_voter_weight_record(&registrar_cookie, &voter_cookie)
        .await?;

    let proposal_cookie = core_voter_test
        .governance
        .with_proposal(&realm_cookie)
        .await?;

    core_voter_test.bench.advance_clock().await;

    // Act — vote with NFTs from both collections in a single instruction
    core_voter_test
        .cast_asset_vote(
            &registrar_cookie,
            &voter_weight_record_cookie,
            &max_voter_weight_record_cookie,
            &proposal_cookie,
            &voter_cookie,
            &voter_token_owner_record_cookie,
            &[&asset_cookie1, &asset_cookie2],
            None,
        )
        .await?;

    // Assert — weight should be sum of both NFT weights (10 + 20 = 30)
    let voter_weight_record = core_voter_test
        .get_voter_weight_record(&voter_weight_record_cookie.address)
        .await;

    assert_eq!(voter_weight_record.voter_weight, 30);

    Ok(())
}
