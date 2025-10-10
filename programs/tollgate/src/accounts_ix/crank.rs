use anchor_lang::prelude::*;
use anchor_spl::{associated_token::AssociatedToken, token_interface};

use crate::{
    constants::{
        damm_v2_constants, INVESTOR_FEE_POS_OWNER, POLICY_SEED, PROGRESS_SEED, VAULT_SEED,
    },
    error::TollgateError,
    state::{Policy, Progress},
    utils::pool::is_valid_pool,
};

/// Accounts required for the daily crank
#[derive(Accounts)]
pub struct AccountCrank<'info> {
    /// The policy account.
    #[account(
        seeds = [POLICY_SEED, policy.vault.as_ref()],
        bump = policy.bump,
    )]
    pub policy: Account<'info, Policy>,

    /// The progress account.
    #[account(
        seeds = [PROGRESS_SEED, policy.vault.as_ref()],
        bump = progress.bump,
    )]
    pub progress: Account<'info, Progress>,

    /// The DAMM v2 pool account that must be valid.
    #[account(constraint = is_valid_pool(&pool.load().ok()) @ TollgateError::InvalidPool)]
    pub pool: AccountLoader<'info, damm_v2::accounts::Pool>,

    /// The position NFT account.
    #[account(
        constraint = position_nft_account.mint == position.load()?.nft_mint,
        constraint = position_nft_account.amount == 1,
        token::authority = owner,
    )]
    pub position_nft_account: Box<InterfaceAccount<'info, token_interface::TokenAccount>>,

    /// The DAMM v2 pool position account.
    #[account(
        mut,
        has_one = pool @ TollgateError::InvalidPosition,
    )]
    pub position: AccountLoader<'info, damm_v2::accounts::Position>,

    /// The pool authority account (unchecked).
    /// CHECK: DAMM v2 pool authority.
    #[account(address = damm_v2_constants::pool_authority::ID)]
    pub pool_authority: UncheckedAccount<'info>,

    /// The system account that owns the vault.
    #[account(
        seeds = [VAULT_SEED, policy.vault.as_ref(), INVESTOR_FEE_POS_OWNER],
        bump = policy.owner_bump,
    )]
    pub owner: SystemAccount<'info>,

    /// The treasury account.
    #[account(
        init_if_needed,
        payer = payer,
        associated_token::mint = quote_mint,
        associated_token::authority = owner,
        associated_token::token_program = associated_token_program,
    )]
    pub treasury: Box<InterfaceAccount<'info, token_interface::TokenAccount>>,

    /// The owner base account.
    #[account(
        init_if_needed,
        payer = payer,
        associated_token::mint = base_mint,
        associated_token::authority = owner,
        associated_token::token_program = associated_token_program,
    )]
    pub base_account: Box<InterfaceAccount<'info, token_interface::TokenAccount>>,

    /// The owner quote account.
    #[account(
        init_if_needed,
        payer = payer,
        associated_token::mint = quote_mint,
        associated_token::authority = owner,
        associated_token::token_program = associated_token_program,
    )]
    pub quote_account: Box<InterfaceAccount<'info, token_interface::TokenAccount>>,

    /// The base vault account.
    #[account(mut, token::token_program = base_program, token::mint = base_mint)]
    pub base_vault: Box<InterfaceAccount<'info, token_interface::TokenAccount>>,

    /// The quote vault account.
    #[account(mut, token::token_program = quote_program, token::mint = quote_mint)]
    pub quote_vault: Box<InterfaceAccount<'info, token_interface::TokenAccount>>,

    /// The base mint account.
    pub base_mint: Box<InterfaceAccount<'info, token_interface::Mint>>,

    /// The quote mint account.
    pub quote_mint: Box<InterfaceAccount<'info, token_interface::Mint>>,

    /// The base token program account.
    pub base_program: Interface<'info, token_interface::TokenInterface>,

    /// The quote token program account.
    pub quote_program: Interface<'info, token_interface::TokenInterface>,

    /// The creator account.
    #[account(
        mut,
        associated_token::mint = quote_mint,
        associated_token::authority = policy.creator,
        associated_token::token_program = associated_token_program,
    )]
    pub creator_accoount: Box<InterfaceAccount<'info, token_interface::TokenAccount>>,

    /// The signer account that will pay for the instruction.
    #[account(mut)]
    pub payer: Signer<'info>,

    /// The event authority account (unchecked).
    /// CHECK: DAMM v2 event authority.
    #[account(
        seeds = [b"__event_authority"],
        bump,
        seeds::program = damm_v2::ID,
    )]
    pub event_authority: UncheckedAccount<'info>,

    /// The DAMM v2 AMM program account.
    #[account(address = damm_v2::ID @ TollgateError::AMMProgramMismatch)]
    pub amm_program: Program<'info, damm_v2::program::CpAmm>,

    /// The associated token program account.
    pub associated_token_program: Program<'info, AssociatedToken>,

    /// The system program account.
    pub system_program: Program<'info, System>,
}
