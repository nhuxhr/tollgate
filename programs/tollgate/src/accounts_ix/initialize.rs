use anchor_lang::prelude::*;
use anchor_spl::{token_2022::Token2022, token_interface::Mint};

use crate::{
    constants::{
        damm_v2_constants, INVESTOR_FEE_POS_OWNER, POLICY_SEED, PROGRESS_SEED, VAULT_SEED,
    },
    error::TollgateError,
    state::{Policy, Progress},
    utils::pool::{is_valid_pool, is_valid_pool_cfg},
};

/// Accounts required for the initialization of a honorary position
#[derive(Accounts)]
pub struct AccountInitialize<'info> {
    /// The signer account that will be used to create the policy and progress accounts.
    pub vault: Signer<'info>,

    /// The policy account that will be initialized.
    #[account(
        init,
        payer = payer,
        space = Policy::SPACE,
        seeds = [POLICY_SEED, vault.key().as_ref()],
        bump,
    )]
    pub policy: Account<'info, Policy>,

    /// The progress account that will be initialized.
    #[account(
        init,
        payer = payer,
        space = Progress::SPACE,
        seeds = [PROGRESS_SEED, vault.key().as_ref()],
        bump,
    )]
    pub progress: Account<'info, Progress>,

    /// The DAMM v2 pool account that must be valid and will be used for validations.
    #[account(mut, constraint = is_valid_pool(&pool.load().ok()) @ TollgateError::InvalidPool)]
    pub pool: AccountLoader<'info, damm_v2::accounts::Pool>,

    /// The pool configuration account that must be valid and will be used for validations.
    #[account(constraint = is_valid_pool_cfg(&pool_cfg.load().ok()) @ TollgateError::InvalidPoolConfig)]
    pub pool_cfg: AccountLoader<'info, damm_v2::accounts::Config>,

    /// The mint account for the position NFT.
    #[account(mut)]
    pub position_nft_mint: Signer<'info>,

    /// The account that will hold the position NFT (unchecked).
    /// CHECK: This account will be initialized by the DAMM v2 program during the CPI.
    #[account(
        mut,
        seeds = [damm_v2_constants::seeds::POSITION_NFT_ACCOUNT_PREFIX, position_nft_mint.key().as_ref()],
        bump,
        seeds::program = damm_v2::ID,
    )]
    pub position_nft_account: UncheckedAccount<'info>,

    /// The DAMM v2 pool position account (unchecked).
    /// CHECK: This account will be initialized by the DAMM v2 program during the CPI.
    #[account(
        mut,
        seeds = [damm_v2_constants::seeds::POSITION_PREFIX, position_nft_mint.key().as_ref()],
        bump,
        seeds::program = damm_v2::ID,
    )]
    pub position: UncheckedAccount<'info>,

    /// The pool authority account (unchecked).
    /// CHECK: DAMM v2 pool authority.
    #[account(address = damm_v2_constants::pool_authority::ID)]
    pub pool_authority: UncheckedAccount<'info>,

    /// The system account that owns the vault.
    #[account(
        seeds = [VAULT_SEED, vault.key().as_ref(), INVESTOR_FEE_POS_OWNER],
        bump,
    )]
    pub owner: SystemAccount<'info>,

    /// The quote mint account.
    pub quote_mint: Box<InterfaceAccount<'info, Mint>>,

    /// The signer account that will pay for the initialization.
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

    /// The Token 2022 program account.
    pub token_2022_program: Program<'info, Token2022>,

    /// The system program account.
    pub system_program: Program<'info, System>,
}
