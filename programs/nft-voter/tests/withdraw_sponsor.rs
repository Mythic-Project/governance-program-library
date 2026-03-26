use gpl_nft_voter::error::NftVoterError;
use program_test::nft_voter_test::NftVoterTest;
use program_test::tools::assert_nft_voter_err;

use solana_program_test::*;
use solana_sdk::transport::TransportError;

mod program_test;

#[tokio::test]
async fn test_withdraw_sponsor() -> Result<(), TransportError> {
    // Arrange
    let mut nft_voter_test = NftVoterTest::start_new().await;

    let realm_cookie = nft_voter_test.governance.with_realm().await?;

    let registrar_cookie = nft_voter_test.with_registrar(&realm_cookie).await?;

    let sponsor_cookie = nft_voter_test
        .with_sponsor(&registrar_cookie, &realm_cookie)
        .await?;

    // Fund the sponsor with 10 SOL
    let fund_amount = 10_000_000_000u64;
    nft_voter_test
        .fund_sponsor(&sponsor_cookie, fund_amount)
        .await?;

    let destination = nft_voter_test.bench.with_wallet().await;
    let destination_before = nft_voter_test
        .bench
        .get_account(&destination.address)
        .await
        .unwrap()
        .lamports;

    let withdraw_amount = 5_000_000_000u64;

    // Act
    nft_voter_test
        .withdraw_sponsor(
            &sponsor_cookie,
            &registrar_cookie,
            &destination.address,
            withdraw_amount,
        )
        .await?;

    // Assert
    let destination_after = nft_voter_test
        .bench
        .get_account(&destination.address)
        .await
        .unwrap()
        .lamports;

    assert_eq!(destination_after - destination_before, withdraw_amount);

    Ok(())
}

#[tokio::test]
async fn test_withdraw_sponsor_with_insufficient_funds_error() -> Result<(), TransportError> {
    // Arrange
    let mut nft_voter_test = NftVoterTest::start_new().await;

    let realm_cookie = nft_voter_test.governance.with_realm().await?;

    let registrar_cookie = nft_voter_test.with_registrar(&realm_cookie).await?;

    let sponsor_cookie = nft_voter_test
        .with_sponsor(&registrar_cookie, &realm_cookie)
        .await?;

    // Fund with a small amount
    nft_voter_test
        .fund_sponsor(&sponsor_cookie, 1_000_000)
        .await?;

    let destination = nft_voter_test.bench.with_wallet().await;

    // Try to withdraw more than available (beyond rent-exempt minimum)
    let withdraw_amount = 100_000_000_000u64;

    // Act
    let err = nft_voter_test
        .withdraw_sponsor(
            &sponsor_cookie,
            &registrar_cookie,
            &destination.address,
            withdraw_amount,
        )
        .await
        .err()
        .unwrap();

    // Assert
    assert_nft_voter_err(err, NftVoterError::InsufficientSponsorFunds);

    Ok(())
}

#[tokio::test]
async fn test_withdraw_sponsor_with_invalid_authority_error() -> Result<(), TransportError> {
    // Arrange
    let mut nft_voter_test = NftVoterTest::start_new().await;

    let realm_cookie = nft_voter_test.governance.with_realm().await?;

    let registrar_cookie = nft_voter_test.with_registrar(&realm_cookie).await?;

    let sponsor_cookie = nft_voter_test
        .with_sponsor(&registrar_cookie, &realm_cookie)
        .await?;

    nft_voter_test
        .fund_sponsor(&sponsor_cookie, 10_000_000_000)
        .await?;

    let destination = nft_voter_test.bench.with_wallet().await;
    let fake_authority = nft_voter_test.bench.with_wallet().await;

    // Build instruction manually with wrong authority
    let data = anchor_lang::InstructionData::data(
        &gpl_nft_voter::instruction::WithdrawSponsor {
            amount: 1_000_000_000,
        },
    );

    let accounts = gpl_nft_voter::accounts::WithdrawSponsor {
        registrar: registrar_cookie.address,
        sponsor: sponsor_cookie.address,
        realm: registrar_cookie.account.realm,
        realm_authority: fake_authority.address,
        destination: destination.address,
        system_program: solana_sdk::system_program::id(),
    };

    let withdraw_ix = solana_sdk::instruction::Instruction {
        program_id: gpl_nft_voter::id(),
        accounts: anchor_lang::ToAccountMetas::to_account_metas(&accounts, None),
        data,
    };

    // Act
    let err = nft_voter_test
        .bench
        .process_transaction(&[withdraw_ix], Some(&[&fake_authority.signer]))
        .await
        .err()
        .unwrap();

    // Assert
    assert_nft_voter_err(err, NftVoterError::InvalidSponsorAuthority);

    Ok(())
}
