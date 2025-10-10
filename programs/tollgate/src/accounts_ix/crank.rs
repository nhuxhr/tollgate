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

#[derive(Accounts)]
pub struct AccountCrank<'info> {
    #[account(
        seeds = [POLICY_SEED, policy.vault.as_ref()],
        bump = policy.bump,
    )]
    pub policy: Account<'info, Policy>,

    #[account(
        seeds = [PROGRESS_SEED, policy.vault.as_ref()],
        bump = progress.bump,
    )]
    pub progress: Account<'info, Progress>,

    #[account(constraint = is_valid_pool(&pool.load().ok()) @ TollgateError::InvalidPool)]
    pub pool: AccountLoader<'info, damm_v2::accounts::Pool>,

    #[account(
        constraint = position_nft_account.mint == position.load()?.nft_mint,
        constraint = position_nft_account.amount == 1,
        token::authority = owner,
    )]
    pub position_nft_account: Box<InterfaceAccount<'info, token_interface::TokenAccount>>,

    #[account(
        mut,
        has_one = pool @ TollgateError::InvalidPosition,
    )]
    pub position: AccountLoader<'info, damm_v2::accounts::Position>,

    /// CHECK: pool authority
    #[account(address = damm_v2_constants::pool_authority::ID)]
    pub pool_authority: UncheckedAccount<'info>,

    #[account(
        seeds = [VAULT_SEED, policy.vault.as_ref(), INVESTOR_FEE_POS_OWNER],
        bump = policy.owner_bump,
    )]
    pub owner: SystemAccount<'info>,

    #[account(
        init_if_needed,
        payer = payer,
        associated_token::mint = quote_mint,
        associated_token::authority = owner,
        associated_token::token_program = associated_token_program,
    )]
    pub treasury: Box<InterfaceAccount<'info, token_interface::TokenAccount>>,

    #[account(
        init_if_needed,
        payer = payer,
        associated_token::mint = base_mint,
        associated_token::authority = owner,
        associated_token::token_program = associated_token_program,
    )]
    pub base_account: Box<InterfaceAccount<'info, token_interface::TokenAccount>>,

    #[account(
        init_if_needed,
        payer = payer,
        associated_token::mint = quote_mint,
        associated_token::authority = owner,
        associated_token::token_program = associated_token_program,
    )]
    pub quote_account: Box<InterfaceAccount<'info, token_interface::TokenAccount>>,

    #[account(mut, token::token_program = base_program, token::mint = base_mint)]
    pub base_vault: Box<InterfaceAccount<'info, token_interface::TokenAccount>>,

    #[account(mut, token::token_program = quote_program, token::mint = quote_mint)]
    pub quote_vault: Box<InterfaceAccount<'info, token_interface::TokenAccount>>,

    pub base_mint: Box<InterfaceAccount<'info, token_interface::Mint>>,

    pub quote_mint: Box<InterfaceAccount<'info, token_interface::Mint>>,

    pub base_program: Interface<'info, token_interface::TokenInterface>,

    pub quote_program: Interface<'info, token_interface::TokenInterface>,

    #[account(
        mut,
        associated_token::mint = quote_mint,
        associated_token::authority = policy.creator,
        associated_token::token_program = associated_token_program,
    )]
    pub creator_accoount: Box<InterfaceAccount<'info, token_interface::TokenAccount>>,

    #[account(mut)]
    pub payer: Signer<'info>,

    /// CHECK: DAMM v2 event authority
    #[account(
        seeds = [b"__event_authority"],
        bump,
        seeds::program = damm_v2::ID,
    )]
    pub event_authority: UncheckedAccount<'info>,

    #[account(address = damm_v2::ID @ TollgateError::AMMProgramMismatch)]
    pub amm_program: Program<'info, damm_v2::program::CpAmm>,

    pub associated_token_program: Program<'info, AssociatedToken>,

    pub system_program: Program<'info, System>,
}
