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

#[derive(Accounts)]
pub struct AccountInitialize<'info> {
    pub vault: Signer<'info>,

    #[account(
        init,
        payer = payer,
        space = Policy::SPACE,
        seeds = [POLICY_SEED, vault.key().as_ref()],
        bump,
    )]
    pub policy: Account<'info, Policy>,

    #[account(
        init,
        payer = payer,
        space = Progress::SPACE,
        seeds = [PROGRESS_SEED, vault.key().as_ref()],
        bump,
    )]
    pub progress: Account<'info, Progress>,

    #[account(mut, constraint = is_valid_pool(&pool.load().ok()) @ TollgateError::InvalidPool)]
    pub pool: AccountLoader<'info, damm_v2::accounts::Pool>,

    #[account(constraint = is_valid_pool_cfg(&pool_cfg.load().ok()) @ TollgateError::InvalidPoolConfig)]
    pub pool_cfg: AccountLoader<'info, damm_v2::accounts::Config>,

    #[account(mut)]
    pub position_nft_mint: Signer<'info>,

    /// CHECK:
    #[account(
        mut,
        seeds = [damm_v2_constants::seeds::POSITION_NFT_ACCOUNT_PREFIX, position_nft_mint.key().as_ref()],
        bump,
        seeds::program = damm_v2::ID,
    )]
    pub position_nft_account: UncheckedAccount<'info>,

    /// CHECK:
    #[account(
        mut,
        seeds = [damm_v2_constants::seeds::POSITION_PREFIX, position_nft_mint.key().as_ref()],
        bump,
        seeds::program = damm_v2::ID,
    )]
    pub position: UncheckedAccount<'info>,

    /// CHECK: pool authority
    #[account(address = damm_v2_constants::pool_authority::ID)]
    pub pool_authority: UncheckedAccount<'info>,

    #[account(
        seeds = [VAULT_SEED, vault.key().as_ref(), INVESTOR_FEE_POS_OWNER],
        bump,
    )]
    pub owner: SystemAccount<'info>,

    pub quote_mint: Box<InterfaceAccount<'info, Mint>>,

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

    pub token_2022_program: Program<'info, Token2022>,

    pub system_program: Program<'info, System>,
}
