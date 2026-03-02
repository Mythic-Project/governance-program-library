use crate::program_test::nft_voter_test::ConfigureCollectionArgs;
use gpl_nft_voter::error::NftVoterError;
use gpl_nft_voter::state::*;
use program_test::nft_voter_test::*;
use program_test::tools::{assert_gov_err, assert_nft_voter_err};

use solana_program_test::*;
use solana_sdk::transport::TransportError;
use spl_governance::error::GovernanceError;

mod program_test;

#[tokio::test]
async fn test_cast_nft_vote_sponsored() -> Result<(), TransportError> {
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

    // Fund sponsor with enough SOL for vote records
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

    nft_voter_test.bench.advance_clock().await;
    let clock = nft_voter_test.bench.get_clock().await;

    // Act
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

    // Assert
    let nft_vote_record = nft_voter_test
        .get_nft_vote_record_sponsored_account(&nft_vote_record_sponsored_cookies[0].address)
        .await;

    assert_eq!(
        nft_vote_record_sponsored_cookies[0].account,
        nft_vote_record
    );
    assert_eq!(nft_vote_record.proposal, proposal_cookie.address);
    assert_eq!(
        nft_vote_record.nft_mint,
        nft_cookie1.mint_cookie.address
    );
    assert_eq!(
        nft_vote_record.governing_token_owner,
        voter_cookie.address
    );
    assert_eq!(nft_vote_record.sponsor, sponsor_cookie.address);

    let voter_weight_record = nft_voter_test
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
async fn test_cast_nft_vote_sponsored_with_multiple_nfts() -> Result<(), TransportError> {
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

    let nft_cookie2 = nft_voter_test
        .token_metadata
        .with_nft_v2(&nft_collection_cookie, &voter_cookie, None)
        .await?;

    nft_voter_test.bench.advance_clock().await;
    let clock = nft_voter_test.bench.get_clock().await;

    // Act
    let nft_vote_record_sponsored_cookies = nft_voter_test
        .cast_nft_vote_sponsored(
            &registrar_cookie,
            &voter_weight_record_cookie,
            &max_voter_weight_record_cookie,
            &proposal_cookie,
            &sponsor_cookie,
            &voter_cookie,
            &voter_token_owner_record_cookie,
            &[&nft_cookie1, &nft_cookie2],
            None,
        )
        .await?;

    // Assert
    let nft_vote_record1 = nft_voter_test
        .get_nft_vote_record_sponsored_account(&nft_vote_record_sponsored_cookies[0].address)
        .await;

    assert_eq!(
        nft_vote_record_sponsored_cookies[0].account,
        nft_vote_record1
    );

    let nft_vote_record2 = nft_voter_test
        .get_nft_vote_record_sponsored_account(&nft_vote_record_sponsored_cookies[1].address)
        .await;

    assert_eq!(
        nft_vote_record_sponsored_cookies[1].account,
        nft_vote_record2
    );

    let voter_weight_record = nft_voter_test
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
async fn test_cast_nft_vote_sponsored_accumulative() -> Result<(), TransportError> {
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

    nft_voter_test.bench.advance_clock().await;

    // Cast first vote without spl-gov vote
    let args = CastNftVoteSponsoredArgs {
        cast_spl_gov_vote: false,
    };

    nft_voter_test
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

    let nft_cookie2 = nft_voter_test
        .token_metadata
        .with_nft_v2(&nft_collection_cookie, &voter_cookie, None)
        .await?;

    // Act - cast second vote with spl-gov vote, should accumulate
    nft_voter_test
        .cast_nft_vote_sponsored(
            &registrar_cookie,
            &voter_weight_record_cookie,
            &max_voter_weight_record_cookie,
            &proposal_cookie,
            &sponsor_cookie,
            &voter_cookie,
            &voter_token_owner_record_cookie,
            &[&nft_cookie2],
            None,
        )
        .await?;

    // Assert - weight should be accumulated (10 + 10 = 20)
    let voter_weight_record = nft_voter_test
        .get_voter_weight_record(&voter_weight_record_cookie.address)
        .await;

    assert_eq!(voter_weight_record.voter_weight, 20);
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
async fn test_cast_nft_vote_sponsored_with_nft_already_voted_error() -> Result<(), TransportError> {
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

    nft_voter_test
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

    // Act - try to vote again with same NFT
    let err = nft_voter_test
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
        .await
        .err()
        .unwrap();

    // Assert
    assert_nft_voter_err(err, NftVoterError::NftAlreadyVoted);

    Ok(())
}

#[tokio::test]
async fn test_cast_nft_vote_sponsored_with_invalid_voter_error() -> Result<(), TransportError> {
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

    // Use a different voter
    let voter_cookie2 = nft_voter_test.bench.with_wallet().await;

    // Act
    let err = nft_voter_test
        .cast_nft_vote_sponsored(
            &registrar_cookie,
            &voter_weight_record_cookie,
            &max_voter_weight_record_cookie,
            &proposal_cookie,
            &sponsor_cookie,
            &voter_cookie2,
            &voter_token_owner_record_cookie,
            &[&nft_cookie1],
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
async fn test_cast_nft_vote_sponsored_with_invalid_owner_error() -> Result<(), TransportError> {
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

    // Create NFT owned by a different wallet
    let voter_cookie2 = nft_voter_test.bench.with_wallet().await;

    let proposal_cookie = nft_voter_test
        .governance
        .with_proposal(&realm_cookie)
        .await?;

    let nft_cookie = nft_voter_test
        .token_metadata
        .with_nft_v2(&nft_collection_cookie, &voter_cookie2, None)
        .await?;

    // Act
    let err = nft_voter_test
        .cast_nft_vote_sponsored(
            &registrar_cookie,
            &voter_weight_record_cookie,
            &max_voter_weight_record_cookie,
            &proposal_cookie,
            &sponsor_cookie,
            &voter_cookie,
            &voter_token_owner_record_cookie,
            &[&nft_cookie],
            None,
        )
        .await
        .err()
        .unwrap();

    // Assert
    assert_nft_voter_err(err, NftVoterError::VoterDoesNotOwnNft);

    Ok(())
}

#[tokio::test]
async fn test_cast_nft_vote_sponsored_with_insufficient_sponsor_funds_error(
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

    // Don't fund the sponsor - it only has rent-exempt minimum

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

    nft_voter_test.bench.advance_clock().await;

    // Act
    let err = nft_voter_test
        .cast_nft_vote_sponsored(
            &registrar_cookie,
            &voter_weight_record_cookie,
            &max_voter_weight_record_cookie,
            &proposal_cookie,
            &sponsor_cookie,
            &voter_cookie,
            &voter_token_owner_record_cookie,
            &[&nft_cookie1],
            Some(CastNftVoteSponsoredArgs {
                cast_spl_gov_vote: false,
            }),
        )
        .await
        .err()
        .unwrap();

    // Assert
    assert_nft_voter_err(err, NftVoterError::InsufficientSponsorFunds);

    Ok(())
}

#[tokio::test]
async fn test_cast_nft_vote_sponsored_after_cast_nft_vote_with_same_nft_error(
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

    // First: cast vote via non-sponsored path
    nft_voter_test
        .cast_nft_vote(
            &registrar_cookie,
            &voter_weight_record_cookie,
            &max_voter_weight_record_cookie,
            &proposal_cookie,
            &voter_cookie,
            &voter_token_owner_record_cookie,
            &[&nft_cookie1],
            Some(CastNftVoteArgs {
                cast_spl_gov_vote: false,
            }),
        )
        .await?;

    nft_voter_test.bench.advance_clock().await;

    // Act: try to cast again via sponsored path with same NFT
    let err = nft_voter_test
        .cast_nft_vote_sponsored(
            &registrar_cookie,
            &voter_weight_record_cookie,
            &max_voter_weight_record_cookie,
            &proposal_cookie,
            &sponsor_cookie,
            &voter_cookie,
            &voter_token_owner_record_cookie,
            &[&nft_cookie1],
            Some(CastNftVoteSponsoredArgs {
                cast_spl_gov_vote: false,
            }),
        )
        .await
        .err()
        .unwrap();

    // Assert - should fail because the NftVoteRecord PDA already exists
    // (shared PDA namespace prevents cross-instruction double voting)
    assert_nft_voter_err(err, NftVoterError::NftAlreadyVoted);

    Ok(())
}

#[tokio::test]
async fn test_cast_nft_vote_after_cast_nft_vote_sponsored_with_same_nft_error(
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

    // First: cast vote via sponsored path
    nft_voter_test
        .cast_nft_vote_sponsored(
            &registrar_cookie,
            &voter_weight_record_cookie,
            &max_voter_weight_record_cookie,
            &proposal_cookie,
            &sponsor_cookie,
            &voter_cookie,
            &voter_token_owner_record_cookie,
            &[&nft_cookie1],
            Some(CastNftVoteSponsoredArgs {
                cast_spl_gov_vote: false,
            }),
        )
        .await?;

    nft_voter_test.bench.advance_clock().await;

    // Act: try to cast again via non-sponsored path with same NFT
    let err = nft_voter_test
        .cast_nft_vote(
            &registrar_cookie,
            &voter_weight_record_cookie,
            &max_voter_weight_record_cookie,
            &proposal_cookie,
            &voter_cookie,
            &voter_token_owner_record_cookie,
            &[&nft_cookie1],
            Some(CastNftVoteArgs {
                cast_spl_gov_vote: false,
            }),
        )
        .await
        .err()
        .unwrap();

    // Assert - should fail because the NftVoteRecordSponsored PDA already exists
    // at the same address as NftVoteRecord would use (shared PDA namespace)
    assert_nft_voter_err(err, NftVoterError::NftAlreadyVoted);

    Ok(())
}
