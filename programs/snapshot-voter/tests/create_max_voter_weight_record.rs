use crate::program_test::snapshot_voter_test::SnapshotVoterTest;
use program_test::tools::assert_ix_err;
use solana_program::instruction::InstructionError;
use solana_program_test::*;
use solana_sdk::transport::TransportError;

mod program_test;

#[tokio::test]
async fn test_create_max_voter_weight_record() -> Result<(), TransportError> {
    // Arrange
    let mut snapshot_voter_test = SnapshotVoterTest::start_new().await;

    let realm_cookie = snapshot_voter_test.governance.with_realm().await?;

    let registrar_cookie = snapshot_voter_test.with_registrar(&realm_cookie).await?;

    // Act
    let max_voter_weight_record_cookie = snapshot_voter_test
        .with_max_voter_weight_record(&registrar_cookie)
        .await?;

    // Assert
    let max_voter_weight_record = snapshot_voter_test
        .get_max_voter_weight_record(&max_voter_weight_record_cookie.address)
        .await;

    assert_eq!(
        max_voter_weight_record_cookie.account,
        max_voter_weight_record
    );

    Ok(())
}

#[tokio::test]
async fn test_create_max_voter_weight_record_with_already_exists_error(
) -> Result<(), TransportError> {
    // Arrange
    let mut snapshot_voter_test = SnapshotVoterTest::start_new().await;

    let realm_cookie = snapshot_voter_test.governance.with_realm().await?;

    let registrar_cookie = snapshot_voter_test.with_registrar(&realm_cookie).await?;

    snapshot_voter_test
        .with_max_voter_weight_record(&registrar_cookie)
        .await?;

    snapshot_voter_test.bench.advance_clock().await;

    // Act
    let err = snapshot_voter_test
        .with_max_voter_weight_record(&registrar_cookie)
        .await
        .err()
        .unwrap();

    // Assert

    // InstructionError::Custom(0) is returned for TransactionError::AccountInUse
    assert_ix_err(err, InstructionError::Custom(0));

    Ok(())
}
