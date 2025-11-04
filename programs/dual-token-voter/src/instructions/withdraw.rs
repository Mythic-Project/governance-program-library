use {
    crate::{error::*, state::*},
    anchor_lang::prelude::*,
    anchor_spl::token_interface::{transfer_checked, Mint, TokenAccount, TokenInterface, TransferChecked},
};

/// Withdraws token_a and/or token_b from vaults back to user
/// Users can withdraw only token_a, only token_b, or both in a single transaction
#[derive(Accounts)]
pub struct Withdraw<'info> {
    pub registrar: Account<'info, Registrar>,

    #[account(
        mut,
        seeds = [b"voter", registrar.key().as_ref(), voter_authority.key().as_ref()],
        bump = voter.voter_bump,
        has_one = voter_authority,
    )]
    pub voter: Account<'info, Voter>,

    /// User's token_a account to withdraw to (if withdrawing token_a)
    #[account(mut)]
    pub voter_token_a_account: Option<Box<InterfaceAccount<'info, TokenAccount>>>,

    /// User's token_b account to withdraw to (if withdrawing token_b)
    #[account(mut)]
    pub voter_token_b_account: Option<Box<InterfaceAccount<'info, TokenAccount>>>,

    /// Vault for token_a (owned by voter PDA)
    #[account(
        mut,
        constraint = vault_a.mint == registrar.token_a_mint
            @ DualTokenVoterError::WrongTokenMint,
    )]
    pub vault_a: Box<InterfaceAccount<'info, TokenAccount>>,

    /// Vault for token_b (owned by voter PDA)
    #[account(
        mut,
        constraint = vault_b.mint == registrar.token_b_mint
            @ DualTokenVoterError::WrongTokenMint,
    )]
    pub vault_b: Box<InterfaceAccount<'info, TokenAccount>>,

    /// Token_a mint (for Token-2022 compatibility)
    #[account(
        constraint = token_a_mint.key() == registrar.token_a_mint
            @ DualTokenVoterError::WrongTokenMint,
    )]
    pub token_a_mint: Box<InterfaceAccount<'info, Mint>>,

    /// Token_b mint (for Token-2022 compatibility)
    #[account(
        constraint = token_b_mint.key() == registrar.token_b_mint
            @ DualTokenVoterError::WrongTokenMint,
    )]
    pub token_b_mint: Box<InterfaceAccount<'info, Mint>>,

    pub voter_authority: Signer<'info>,

    pub token_program: Interface<'info, TokenInterface>,
}

/// Withdraws token_a and/or token_b from vaults back to user
/// Users can withdraw one or both tokens in a single transaction
/// Updates tokens_update_timestamp which resets the eligibility clock
pub fn withdraw(
    ctx: Context<Withdraw>,
    token_a_amount: u64,
    token_b_amount: u64,
) -> Result<()> {
    let clock = Clock::get()?;

    // Require at least one withdrawal
    require!(
        token_a_amount > 0 || token_b_amount > 0,
        DualTokenVoterError::InvalidAmount
    );

    // Extract values for PDA seeds before CPI to avoid borrow conflicts
    let registrar_key = ctx.accounts.registrar.key();
    let voter_authority = ctx.accounts.voter.voter_authority;
    let voter_bump = ctx.accounts.voter.voter_bump;

    let voter_seeds = &[
        b"voter".as_ref(),
        registrar_key.as_ref(),
        voter_authority.as_ref(),
        &[voter_bump],
    ];

    // Handle token_a withdrawal
    if token_a_amount > 0 {
        // Validate sufficient deposit
        require_gte!(
            ctx.accounts.voter.token_a_deposited_amount,
            token_a_amount,
            DualTokenVoterError::InsufficientDeposit
        );

        // Validate required accounts are present
        require!(
            ctx.accounts.voter_token_a_account.is_some(),
            DualTokenVoterError::MissingTokenAccount
        );

        let voter_token_a = ctx.accounts.voter_token_a_account.as_ref().unwrap();

        // Validate token account mint
        require!(
            voter_token_a.mint == ctx.accounts.token_a_mint.key(),
            DualTokenVoterError::WrongTokenMint
        );

        // Transfer tokens from vault to user using voter PDA as signer
        transfer_checked(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                TransferChecked {
                    from: ctx.accounts.vault_a.to_account_info(),
                    to: voter_token_a.to_account_info(),
                    authority: ctx.accounts.voter.to_account_info(),
                    mint: ctx.accounts.token_a_mint.to_account_info(),
                },
                &[voter_seeds],
            ),
            token_a_amount,
            ctx.accounts.token_a_mint.decimals,
        )?;

        // Update token_a deposit tracking
        let voter = &mut ctx.accounts.voter;
        voter.token_a_deposited_amount = voter
            .token_a_deposited_amount
            .checked_sub(token_a_amount)
            .ok_or(DualTokenVoterError::InsufficientDeposit)?;
    }

    // Handle token_b withdrawal
    if token_b_amount > 0 {
        // Validate sufficient deposit
        require_gte!(
            ctx.accounts.voter.token_b_deposited_amount,
            token_b_amount,
            DualTokenVoterError::InsufficientDeposit
        );

        // Validate required accounts are present
        require!(
            ctx.accounts.voter_token_b_account.is_some(),
            DualTokenVoterError::MissingTokenAccount
        );

        let voter_token_b = ctx.accounts.voter_token_b_account.as_ref().unwrap();

        // Validate token account mint
        require!(
            voter_token_b.mint == ctx.accounts.token_b_mint.key(),
            DualTokenVoterError::WrongTokenMint
        );

        // Transfer tokens from vault to user using voter PDA as signer
        transfer_checked(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                TransferChecked {
                    from: ctx.accounts.vault_b.to_account_info(),
                    to: voter_token_b.to_account_info(),
                    authority: ctx.accounts.voter.to_account_info(),
                    mint: ctx.accounts.token_b_mint.to_account_info(),
                },
                &[voter_seeds],
            ),
            token_b_amount,
            ctx.accounts.token_b_mint.decimals,
        )?;

        // Update token_b deposit tracking
        let voter = &mut ctx.accounts.voter;
        voter.token_b_deposited_amount = voter
            .token_b_deposited_amount
            .checked_sub(token_b_amount)
            .ok_or(DualTokenVoterError::InsufficientDeposit)?;
    }

    // Update timestamp - resets eligibility clock for both tokens
    let voter = &mut ctx.accounts.voter;
    voter.tokens_update_timestamp = clock.unix_timestamp;

    Ok(())
}
