use gpl_nft_voter::error::NftVoterError;
use gpl_nft_voter::state::get_sponsor_address;
use program_test::nft_voter_test::NftVoterTest;
use program_test::tools::assert_nft_voter_err;

use solana_program_test::*;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;
use solana_sdk::transport::TransportError;

mod program_test;

#[tokio::test]
async fn test_create_sponsor() -> Result<(), TransportError> {
    // Arrange
    let mut nft_voter_test = NftVoterTest::start_new().await;

    let realm_cookie = nft_voter_test.governance.with_realm().await?;

    let registrar_cookie = nft_voter_test.with_registrar(&realm_cookie).await?;

    // Act
    let sponsor_cookie = nft_voter_test
        .with_sponsor(&registrar_cookie, &realm_cookie)
        .await?;

    // Assert - verify the PDA address is correct
    let expected_address = get_sponsor_address(&registrar_cookie.address);
    assert_eq!(sponsor_cookie.address, expected_address);

    Ok(())
}

#[tokio::test]
async fn test_create_sponsor_with_invalid_realm_authority_error() -> Result<(), TransportError> {
    // Arrange
    let mut nft_voter_test = NftVoterTest::start_new().await;

    let realm_cookie = nft_voter_test.governance.with_realm().await?;

    let registrar_cookie = nft_voter_test.with_registrar(&realm_cookie).await?;

    let fake_authority = Keypair::new();

    // Act
    let err = nft_voter_test
        .with_sponsor_using_ix(
            &registrar_cookie,
            &realm_cookie,
            |i| {
                i.accounts[3].pubkey = fake_authority.pubkey(); // realm_authority
            },
            Some(&[&fake_authority]),
        )
        .await
        .err()
        .unwrap();

    // Assert
    assert_nft_voter_err(err, NftVoterError::InvalidRealmAuthority);

    Ok(())
}
