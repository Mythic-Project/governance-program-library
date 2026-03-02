use crate::program_test::nft_voter_test::ConfigureCollectionArgs;
use gpl_nft_voter::error::NftVoterError;
use program_test::nft_voter_test::*;
use program_test::tools::{assert_gov_err, assert_nft_voter_err};

use solana_program_test::*;
use solana_sdk::transport::TransportError;
use spl_governance::error::GovernanceError;

mod program_test;

#[tokio::test]
async fn test_relinquish_nft_vote_sponsored() -> Result<(), TransportError> {
    // Arrange
    let mut nft_voter_test = NftVoterTest::start_new().await;

    let realm_cookie = nft_voter_test.governance.with_realm().await?;

    let registrar_cookie = nft_voter_test.with_registrar(&realm_cookie).await?;

    let nft_collection_cookie = nft_voter_test.token_metadata.with_nft_collection().await?;

    let max_voter_weight_record_cookie = nft_voter_test
        .with_max_voter_weight_record(&registrar_cookie)
        .await?;

    // Set Size == 1 to complete voting with just one vote
    nft_voter_test
        .with_collection(
            &registrar_cookie,
            &nft_collection_cookie,
            &max_voter_weight_record_cookie,
            Some(ConfigureCollectionArgs { weight: 1, size: 1 }),
        )
        .await?;

    let sponsor_cookie = nft_voter_test
        .with_sponsor(&registrar_cookie, &realm_cookie)
        .await?;

    nft_voter_test
        .fund_sponsor(&sponsor_cookie, 10_000_000_000)
        .await?;

    let voter_cookie = nft_voter_test.bench.with_wallet().await;

    let voter_token_owner_record_cookie = nft_voter_test
        .governance
        .with_token_owner_record(&realm_cookie, &voter_cookie)
        .await?;

    let voter_weight_record_cookie = nft_voter_test
        .with_voter_weight_record(&registrar_cookie, &voter_cookie)
        .await?;

    let proposal_cookie = nft_voter_test
        .governance
        .with_proposal(&realm_cookie)
        .await?;

    let nft_cookie1 = nft_voter_test
        .token_metadata
        .with_nft_v2(&nft_collection_cookie, &voter_cookie, None)
        .await?;

    let nft_vote_record_sponsored_cookies = nft_voter_test
        .cast_nft_vote_sponsored(
            &registrar_cookie,
            &voter_weight_record_cookie,
            &max_voter_weight_record_cookie,
            &proposal_cookie,
            &sponsor_cookie,
            &voter_cookie,
            &voter_token_owner_record_cookie,
            &[&nft_cookie1],
            None, // cast spl-gov vote = true
        )
        .await?;

    nft_voter_test.bench.advance_clock().await;

    // Act
    nft_voter_test
        .relinquish_nft_vote_sponsored(
            &registrar_cookie,
            &voter_weight_record_cookie,
            &proposal_cookie,
            &sponsor_cookie,
            &voter_cookie,
            &voter_token_owner_record_cookie,
            &nft_vote_record_sponsored_cookies,
        )
        .await?;

    // Assert
    let voter_weight_record = nft_voter_test
        .get_voter_weight_record(&voter_weight_record_cookie.address)
        .await;

    assert_eq!(voter_weight_record.voter_weight_expiry, Some(0));
    assert_eq!(voter_weight_record.voter_weight, 0);

    // Check NftVoteRecordSponsored was disposed
    let nft_vote_record = nft_voter_test
        .bench
        .get_account(&nft_vote_record_sponsored_cookies[0].address)
        .await;

    assert_eq!(None, nft_vote_record);

    Ok(())
}

#[tokio::test]
async fn test_relinquish_nft_vote_sponsored_returns_rent_to_sponsor() -> Result<(), TransportError>
{
    // Arrange
    let mut nft_voter_test = NftVoterTest::start_new().await;

    let realm_cookie = nft_voter_test.governance.with_realm().await?;

    let registrar_cookie = nft_voter_test.with_registrar(&realm_cookie).await?;

    let nft_collection_cookie = nft_voter_test.token_metadata.with_nft_collection().await?;

    let max_voter_weight_record_cookie = nft_voter_test
        .with_max_voter_weight_record(&registrar_cookie)
        .await?;

    // Set Size == 1 to complete voting with just one vote
    nft_voter_test
        .with_collection(
            &registrar_cookie,
            &nft_collection_cookie,
            &max_voter_weight_record_cookie,
            Some(ConfigureCollectionArgs { weight: 1, size: 1 }),
        )
        .await?;

    let sponsor_cookie = nft_voter_test
        .with_sponsor(&registrar_cookie, &realm_cookie)
        .await?;

    nft_voter_test
        .fund_sponsor(&sponsor_cookie, 10_000_000_000)
        .await?;

    let voter_cookie = nft_voter_test.bench.with_wallet().await;

    let voter_token_owner_record_cookie = nft_voter_test
        .governance
        .with_token_owner_record(&realm_cookie, &voter_cookie)
        .await?;

    let voter_weight_record_cookie = nft_voter_test
        .with_voter_weight_record(&registrar_cookie, &voter_cookie)
        .await?;

    let proposal_cookie = nft_voter_test
        .governance
        .with_proposal(&realm_cookie)
        .await?;

    let nft_cookie1 = nft_voter_test
        .token_metadata
        .with_nft_v2(&nft_collection_cookie, &voter_cookie, None)
        .await?;

    let nft_vote_record_sponsored_cookies = nft_voter_test
        .cast_nft_vote_sponsored(
            &registrar_cookie,
            &voter_weight_record_cookie,
            &max_voter_weight_record_cookie,
            &proposal_cookie,
            &sponsor_cookie,
            &voter_cookie,
            &voter_token_owner_record_cookie,
            &[&nft_cookie1],
            None,
        )
        .await?;

    nft_voter_test.bench.advance_clock().await;

    // Get sponsor balance after casting vote (rent was deducted)
    let sponsor_balance_after_cast = nft_voter_test
        .bench
        .get_account(&sponsor_cookie.address)
        .await
        .unwrap()
        .lamports;

    // Act
    nft_voter_test
        .relinquish_nft_vote_sponsored(
            &registrar_cookie,
            &voter_weight_record_cookie,
            &proposal_cookie,
            &sponsor_cookie,
            &voter_cookie,
            &voter_token_owner_record_cookie,
            &nft_vote_record_sponsored_cookies,
        )
        .await?;

    // Assert - sponsor should have received rent back
    let sponsor_balance_after_relinquish = nft_voter_test
        .bench
        .get_account(&sponsor_cookie.address)
        .await
        .unwrap()
        .lamports;

    assert!(sponsor_balance_after_relinquish > sponsor_balance_after_cast);

    Ok(())
}

#[tokio::test]
async fn test_relinquish_nft_vote_sponsored_for_proposal_in_voting_state(
) -> Result<(), TransportError> {
    // Arrange
    let mut nft_voter_test = NftVoterTest::start_new().await;

    let realm_cookie = nft_voter_test.governance.with_realm().await?;

    let registrar_cookie = nft_voter_test.with_registrar(&realm_cookie).await?;

    let nft_collection_cookie = nft_voter_test.token_metadata.with_nft_collection().await?;

    let max_voter_weight_record_cookie = nft_voter_test
        .with_max_voter_weight_record(&registrar_cookie)
        .await?;

    nft_voter_test
        .with_collection(
            &registrar_cookie,
            &nft_collection_cookie,
            &max_voter_weight_record_cookie,
            None,
        )
        .await?;

    let sponsor_cookie = nft_voter_test
        .with_sponsor(&registrar_cookie, &realm_cookie)
        .await?;

    nft_voter_test
        .fund_sponsor(&sponsor_cookie, 10_000_000_000)
        .await?;

    let voter_cookie = nft_voter_test.bench.with_wallet().await;

    let voter_token_owner_record_cookie = nft_voter_test
        .governance
        .with_token_owner_record(&realm_cookie, &voter_cookie)
        .await?;

    let voter_weight_record_cookie = nft_voter_test
        .with_voter_weight_record(&registrar_cookie, &voter_cookie)
        .await?;

    let proposal_cookie = nft_voter_test
        .governance
        .with_proposal(&realm_cookie)
        .await?;

    let nft_cookie1 = nft_voter_test
        .token_metadata
        .with_nft_v2(&nft_collection_cookie, &voter_cookie, None)
        .await?;

    let nft_vote_record_sponsored_cookies = nft_voter_test
        .cast_nft_vote_sponsored(
            &registrar_cookie,
            &voter_weight_record_cookie,
            &max_voter_weight_record_cookie,
            &proposal_cookie,
            &sponsor_cookie,
            &voter_cookie,
            &voter_token_owner_record_cookie,
            &[&nft_cookie1],
            None,
        )
        .await?;

    // Relinquish Vote from spl-gov first
    nft_voter_test
        .governance
        .relinquish_vote(
            &proposal_cookie,
            &voter_cookie,
            &voter_token_owner_record_cookie,
        )
        .await?;

    nft_voter_test.bench.advance_clock().await;

    // Act
    nft_voter_test
        .relinquish_nft_vote_sponsored(
            &registrar_cookie,
            &voter_weight_record_cookie,
            &proposal_cookie,
            &sponsor_cookie,
            &voter_cookie,
            &voter_token_owner_record_cookie,
            &nft_vote_record_sponsored_cookies,
        )
        .await?;

    // Assert
    let voter_weight_record = nft_voter_test
        .get_voter_weight_record(&voter_weight_record_cookie.address)
        .await;

    assert_eq!(voter_weight_record.voter_weight_expiry, Some(0));
    assert_eq!(voter_weight_record.voter_weight, 0);

    // Check NftVoteRecordSponsored was disposed
    let nft_vote_record = nft_voter_test
        .bench
        .get_account(&nft_vote_record_sponsored_cookies[0].address)
        .await;

    assert_eq!(None, nft_vote_record);

    Ok(())
}

#[tokio::test]
async fn test_relinquish_nft_vote_sponsored_with_vote_record_exists_error(
) -> Result<(), TransportError> {
    // Arrange
    let mut nft_voter_test = NftVoterTest::start_new().await;

    let realm_cookie = nft_voter_test.governance.with_realm().await?;

    let registrar_cookie = nft_voter_test.with_registrar(&realm_cookie).await?;

    let nft_collection_cookie = nft_voter_test.token_metadata.with_nft_collection().await?;

    let max_voter_weight_record_cookie = nft_voter_test
        .with_max_voter_weight_record(&registrar_cookie)
        .await?;

    nft_voter_test
        .with_collection(
            &registrar_cookie,
            &nft_collection_cookie,
            &max_voter_weight_record_cookie,
            None,
        )
        .await?;

    let sponsor_cookie = nft_voter_test
        .with_sponsor(&registrar_cookie, &realm_cookie)
        .await?;

    nft_voter_test
        .fund_sponsor(&sponsor_cookie, 10_000_000_000)
        .await?;

    let voter_cookie = nft_voter_test.bench.with_wallet().await;

    let voter_token_owner_record_cookie = nft_voter_test
        .governance
        .with_token_owner_record(&realm_cookie, &voter_cookie)
        .await?;

    let voter_weight_record_cookie = nft_voter_test
        .with_voter_weight_record(&registrar_cookie, &voter_cookie)
        .await?;

    let proposal_cookie = nft_voter_test
        .governance
        .with_proposal(&realm_cookie)
        .await?;

    let nft_cookie1 = nft_voter_test
        .token_metadata
        .with_nft_v2(&nft_collection_cookie, &voter_cookie, None)
        .await?;

    let nft_vote_record_sponsored_cookies = nft_voter_test
        .cast_nft_vote_sponsored(
            &registrar_cookie,
            &voter_weight_record_cookie,
            &max_voter_weight_record_cookie,
            &proposal_cookie,
            &sponsor_cookie,
            &voter_cookie,
            &voter_token_owner_record_cookie,
            &[&nft_cookie1],
            None, // This casts the spl-gov vote
        )
        .await?;

    // Act - try to relinquish without first relinquishing the spl-gov vote
    // The spl-gov VoteRecord still exists, so this should fail
    let err = nft_voter_test
        .relinquish_nft_vote_sponsored(
            &registrar_cookie,
            &voter_weight_record_cookie,
            &proposal_cookie,
            &sponsor_cookie,
            &voter_cookie,
            &voter_token_owner_record_cookie,
            &nft_vote_record_sponsored_cookies,
        )
        .await
        .err()
        .unwrap();

    // Assert
    assert_nft_voter_err(err, NftVoterError::VoteRecordMustBeWithdrawn);

    Ok(())
}

#[tokio::test]
async fn test_relinquish_nft_vote_sponsored_with_unexpired_voter_weight_error(
) -> Result<(), TransportError> {
    // Arrange
    let mut nft_voter_test = NftVoterTest::start_new().await;

    let realm_cookie = nft_voter_test.governance.with_realm().await?;

    let registrar_cookie = nft_voter_test.with_registrar(&realm_cookie).await?;

    let nft_collection_cookie = nft_voter_test.token_metadata.with_nft_collection().await?;

    let max_voter_weight_record_cookie = nft_voter_test
        .with_max_voter_weight_record(&registrar_cookie)
        .await?;

    nft_voter_test
        .with_collection(
            &registrar_cookie,
            &nft_collection_cookie,
            &max_voter_weight_record_cookie,
            Some(ConfigureCollectionArgs {
                weight: 10,
                size: 20,
            }),
        )
        .await?;

    let sponsor_cookie = nft_voter_test
        .with_sponsor(&registrar_cookie, &realm_cookie)
        .await?;

    nft_voter_test
        .fund_sponsor(&sponsor_cookie, 10_000_000_000)
        .await?;

    let voter_cookie = nft_voter_test.bench.with_wallet().await;

    let voter_token_owner_record_cookie = nft_voter_test
        .governance
        .with_token_owner_record(&realm_cookie, &voter_cookie)
        .await?;

    let voter_weight_record_cookie = nft_voter_test
        .with_voter_weight_record(&registrar_cookie, &voter_cookie)
        .await?;

    let proposal_cookie = nft_voter_test
        .governance
        .with_proposal(&realm_cookie)
        .await?;

    let nft_cookie1 = nft_voter_test
        .token_metadata
        .with_nft_v2(&nft_collection_cookie, &voter_cookie, None)
        .await?;

    // Cast vote without spl-gov vote so VoterWeightRecord is not expired by spl-gov
    let args = CastNftVoteSponsoredArgs {
        cast_spl_gov_vote: false,
    };

    let nft_vote_record_sponsored_cookies = nft_voter_test
        .cast_nft_vote_sponsored(
            &registrar_cookie,
            &voter_weight_record_cookie,
            &max_voter_weight_record_cookie,
            &proposal_cookie,
            &sponsor_cookie,
            &voter_cookie,
            &voter_token_owner_record_cookie,
            &[&nft_cookie1],
            Some(args),
        )
        .await?;

    // Act - try to relinquish immediately (VoterWeightRecord not yet expired)
    let err = nft_voter_test
        .relinquish_nft_vote_sponsored(
            &registrar_cookie,
            &voter_weight_record_cookie,
            &proposal_cookie,
            &sponsor_cookie,
            &voter_cookie,
            &voter_token_owner_record_cookie,
            &nft_vote_record_sponsored_cookies,
        )
        .await
        .err()
        .unwrap();

    // Assert
    assert_nft_voter_err(err, NftVoterError::VoterWeightRecordMustBeExpired);

    Ok(())
}

#[tokio::test]
async fn test_relinquish_nft_vote_sponsored_with_invalid_voter_error() -> Result<(), TransportError>
{
    // Arrange
    let mut nft_voter_test = NftVoterTest::start_new().await;

    let realm_cookie = nft_voter_test.governance.with_realm().await?;

    let registrar_cookie = nft_voter_test.with_registrar(&realm_cookie).await?;

    let nft_collection_cookie = nft_voter_test.token_metadata.with_nft_collection().await?;

    let max_voter_weight_record_cookie = nft_voter_test
        .with_max_voter_weight_record(&registrar_cookie)
        .await?;

    // Set Size == 1 to complete voting with just one vote
    nft_voter_test
        .with_collection(
            &registrar_cookie,
            &nft_collection_cookie,
            &max_voter_weight_record_cookie,
            Some(ConfigureCollectionArgs { weight: 1, size: 1 }),
        )
        .await?;

    let sponsor_cookie = nft_voter_test
        .with_sponsor(&registrar_cookie, &realm_cookie)
        .await?;

    nft_voter_test
        .fund_sponsor(&sponsor_cookie, 10_000_000_000)
        .await?;

    let voter_cookie = nft_voter_test.bench.with_wallet().await;

    let voter_token_owner_record_cookie = nft_voter_test
        .governance
        .with_token_owner_record(&realm_cookie, &voter_cookie)
        .await?;

    let voter_weight_record_cookie = nft_voter_test
        .with_voter_weight_record(&registrar_cookie, &voter_cookie)
        .await?;

    let proposal_cookie = nft_voter_test
        .governance
        .with_proposal(&realm_cookie)
        .await?;

    let nft_cookie1 = nft_voter_test
        .token_metadata
        .with_nft_v2(&nft_collection_cookie, &voter_cookie, None)
        .await?;

    let nft_vote_record_sponsored_cookies = nft_voter_test
        .cast_nft_vote_sponsored(
            &registrar_cookie,
            &voter_weight_record_cookie,
            &max_voter_weight_record_cookie,
            &proposal_cookie,
            &sponsor_cookie,
            &voter_cookie,
            &voter_token_owner_record_cookie,
            &[&nft_cookie1],
            None,
        )
        .await?;

    // Try to use a different voter
    let voter_cookie2 = nft_voter_test.bench.with_wallet().await;

    // Act
    let err = nft_voter_test
        .relinquish_nft_vote_sponsored(
            &registrar_cookie,
            &voter_weight_record_cookie,
            &proposal_cookie,
            &sponsor_cookie,
            &voter_cookie2,
            &voter_token_owner_record_cookie,
            &nft_vote_record_sponsored_cookies,
        )
        .await
        .err()
        .unwrap();

    // Assert
    assert_gov_err(err, GovernanceError::GoverningTokenOwnerOrDelegateMustSign);

    Ok(())
}
