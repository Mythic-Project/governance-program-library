use {
    crate::{error::*, state::*},
    anchor_lang::prelude::*,
    anchor_spl::{
        associated_token::AssociatedToken,
        token_interface::{transfer_checked, Mint, TokenAccount, TokenInterface, TransferChecked},
    },
};

/// Deposits token_a and/or token_b into vaults for voting power
/// Users can deposit only token_a, only token_b, or both in a single transaction
#[derive(Accounts)]
pub struct Deposit<'info> {
    pub registrar: Account<'info, Registrar>,

    #[account(
        mut,
        seeds = [b"voter", registrar.key().as_ref(), voter_authority.key().as_ref()],
        bump = voter.voter_bump,
        has_one = voter_authority,
    )]
    pub voter: Account<'info, Voter>,

    /// User's token_a account to deposit from (if depositing token_a)
    #[account(mut)]
    pub voter_token_a_account: Option<Box<InterfaceAccount<'info, TokenAccount>>>,

    /// User's token_b account to deposit from (if depositing token_b)
    #[account(mut)]
    pub voter_token_b_account: Option<Box<InterfaceAccount<'info, TokenAccount>>>,

    /// Vault for token_a (owned by voter PDA)
    #[account(
        init_if_needed,
        associated_token::authority = voter,
        associated_token::mint = token_a_mint,
        associated_token::token_program = token_program,
        payer = payer
    )]
    pub vault_a: Box<InterfaceAccount<'info, TokenAccount>>,

    /// Vault for token_b (owned by voter PDA)
    #[account(
        init_if_needed,
        associated_token::authority = voter,
        associated_token::mint = token_b_mint,
        associated_token::token_program = token_program,
        payer = payer
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

    #[account(mut)]
    pub payer: Signer<'info>,

    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

/// Deposits token_a and/or token_b into vaults for voting power
/// Users can deposit one or both tokens in a single transaction
/// Updates tokens_update_timestamp which resets the eligibility clock
pub fn deposit(
    ctx: Context<Deposit>,
    token_a_amount: u64,
    token_b_amount: u64,
) -> Result<()> {
    let clock = Clock::get()?;
    let voter = &mut ctx.accounts.voter;

    // Require at least one deposit
    require!(
        token_a_amount > 0 || token_b_amount > 0,
        DualTokenVoterError::InvalidAmount
    );

    // Handle token_a deposit
    if token_a_amount > 0 {
        // Validate required accounts are present
        require!(
            ctx.accounts.voter_token_a_account.is_some(),
            DualTokenVoterError::MissingTokenAccount
        );

        let voter_token_a = ctx.accounts.voter_token_a_account.as_ref().unwrap();

        // Validate token account mint matches registrar's token_a_mint
        require!(
            voter_token_a.mint == ctx.accounts.token_a_mint.key(),
            DualTokenVoterError::WrongTokenMint
        );

        // Transfer tokens to vault using Token-2022 compatible transfer
        transfer_checked(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                TransferChecked {
                    from: voter_token_a.to_account_info(),
                    to: ctx.accounts.vault_a.to_account_info(),
                    authority: ctx.accounts.voter_authority.to_account_info(),
                    mint: ctx.accounts.token_a_mint.to_account_info(),
                },
            ),
            token_a_amount,
            ctx.accounts.token_a_mint.decimals,
        )?;

        // Update token_a deposit tracking
        voter.token_a_deposited_amount = voter
            .token_a_deposited_amount
            .checked_add(token_a_amount)
            .ok_or(DualTokenVoterError::Overflow)?;
    }

    // Handle token_b deposit
    if token_b_amount > 0 {
        // Validate required accounts are present
        require!(
            ctx.accounts.voter_token_b_account.is_some(),
            DualTokenVoterError::MissingTokenAccount
        );

        let voter_token_b = ctx.accounts.voter_token_b_account.as_ref().unwrap();

        // Validate token account mint matches registrar's token_b_mint
        require!(
            voter_token_b.mint == ctx.accounts.token_b_mint.key(),
            DualTokenVoterError::WrongTokenMint
        );

        // Transfer tokens to vault using Token-2022 compatible transfer
        transfer_checked(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                TransferChecked {
                    from: voter_token_b.to_account_info(),
                    to: ctx.accounts.vault_b.to_account_info(),
                    authority: ctx.accounts.voter_authority.to_account_info(),
                    mint: ctx.accounts.token_b_mint.to_account_info(),
                },
            ),
            token_b_amount,
            ctx.accounts.token_b_mint.decimals,
        )?;

        // Update token_b deposit tracking
        voter.token_b_deposited_amount = voter
            .token_b_deposited_amount
            .checked_add(token_b_amount)
            .ok_or(DualTokenVoterError::Overflow)?;
    }

    // Update timestamp - resets eligibility clock for both tokens
    voter.tokens_update_timestamp = clock.unix_timestamp;

    Ok(())
}
