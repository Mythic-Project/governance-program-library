use anchor_lang::prelude::*;
use anchor_lang::solana_program::{
    program::invoke_signed,
    system_instruction,
};
use borsh_1::BorshSerialize;

/// Creates and serializes a PDA account funded by a SystemAccount PDA
///
/// The payer PDA must be system-owned (no data) so that system_instruction::create_account works.
/// Both the payer PDA and new account PDA sign via invoke_signed.
///
/// # Arguments
/// * `payer_pda` - The system-owned PDA that will fund the new account
/// * `payer_seeds` - Seeds (with bump) for the payer PDA
/// * `new_account` - The new account to create (must be a PDA)
/// * `new_account_seeds` - Seeds (without bump) for the new account PDA
/// * `data` - Data to serialize into the new account
/// * `owner` - Program that will own the new account
/// * `system_program` - System program
/// * `rent` - Rent sysvar
pub fn create_and_serialize_account_from_pda<'a, T: BorshSerialize>(
    payer_pda: &AccountInfo<'a>,
    payer_seeds: &[&[u8]],
    new_account: &AccountInfo<'a>,
    new_account_seeds: &[&[u8]],
    data: &T,
    owner: &Pubkey,
    system_program: &AccountInfo<'a>,
    rent: &Rent,
) -> Result<()> {
    let serialized_data = borsh_1::to_vec(data).map_err(|e| ProgramError::BorshIoError(e.to_string()))?;
    let data_len = serialized_data.len();
    let lamports = rent.minimum_balance(data_len);

    // Verify PDA address
    let (expected_new_account, new_account_bump) =
        Pubkey::find_program_address(new_account_seeds, owner);
    require_keys_eq!(
        expected_new_account,
        *new_account.key,
        ErrorCode::ConstraintSeeds
    );

    // Create signer seeds with bump for new account
    let mut new_account_seeds_with_bump = new_account_seeds.to_vec();
    let bump_slice = &[new_account_bump];
    new_account_seeds_with_bump.push(bump_slice);

    // Create account via system program CPI, signed by both the payer PDA and new account PDA
    invoke_signed(
        &system_instruction::create_account(
            payer_pda.key,
            new_account.key,
            lamports,
            data_len as u64,
            owner,
        ),
        &[payer_pda.clone(), new_account.clone(), system_program.clone()],
        &[payer_seeds, &new_account_seeds_with_bump],
    )?;

    // Write data to the account
    new_account
        .try_borrow_mut_data()?
        .copy_from_slice(&serialized_data);

    Ok(())
}
