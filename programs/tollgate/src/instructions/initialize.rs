use std::cell::Ref;

use anchor_lang::prelude::*;

use crate::{
    constants::MAX_BPS, error::TollgateError, events::HonoraryPositionInitialized, utils,
    AccountInitialize,
};

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, Default)]
pub struct InitializeParams {
    pub investor_count: u32,
    pub init_investor_ata: bool,
    pub investor_fee_share_bps: u16,
    pub min_payout_lamports: u64,
    pub daily_cap: Option<u64>,
    pub y0: u64,
}

impl InitializeParams {
    pub fn assert(&self) -> Result<()> {
        // assert investor count is greater than 0
        require_gt!(self.investor_count, 0, TollgateError::InvalidInvestors);

        // assert inveestor fee share bps is less than or equal to 100%
        require_gte!(
            MAX_BPS,
            self.investor_fee_share_bps,
            TollgateError::InvalidInvestorFeeShareBps
        );

        // assert min payout lamports is greater than 0
        require_gt!(
            self.min_payout_lamports,
            0,
            TollgateError::InvalidMinPayoutLamports
        );

        // assert daily cap is either None or greater than 0
        if let Some(daily_cap) = self.daily_cap {
            require_gt!(daily_cap, 0, TollgateError::InvalidDailyCap);
        }

        // assert y0 is greater than 0
        require_gt!(self.y0, 0, TollgateError::InvalidY0Allocation);

        Ok(())
    }
}

pub fn initialize(ctx: Context<AccountInitialize>, params: InitializeParams) -> Result<()> {
    msg!("Initialize::Starting initialization with params: init_investor_ata={}, investor_fee_share_bps={}, min_payout_lamports={}, daily_cap={:?}, y0={}", 
         params.init_investor_ata, params.investor_fee_share_bps, params.min_payout_lamports, params.daily_cap, params.y0);

    // Validate the initialize parameters
    params.assert()?;

    // Load the pool and pool config accounts
    let (base_mint, quote_mint) = {
        msg!("Initialize::Loading pool and pool config accounts");
        // Load pool and pool config accounts
        let pool = ctx.accounts.pool.load()?;
        let pool_cfg = ctx.accounts.pool_cfg.load()?;

        // Determine base/quote mints
        let (base_mint, quote_mint) = if ctx.accounts.quote_mint.key() == pool.token_a_mint {
            (pool.token_b_mint, pool.token_a_mint)
        } else {
            (pool.token_a_mint, pool.token_b_mint)
        };

        // Validate pool
        assert_pool(&pool, &pool_cfg, &base_mint, &quote_mint)?;

        (base_mint, quote_mint)
    };

    // Initialize the policy account
    msg!("Initialize::Initializing policy account");
    let policy = &mut ctx.accounts.policy;
    policy.initialize(
        ctx.accounts.vault.key(),
        ctx.accounts.pool.load()?.creator,
        ctx.accounts.quote_mint.key(),
        params.clone(),
        ctx.bumps.owner,
        ctx.bumps.policy,
    )?;

    // Initialize the progress account
    msg!("Initialize::Initializing progress account");
    let progress = &mut ctx.accounts.progress;
    progress.initialize(ctx.accounts.vault.key(), ctx.bumps.progress)?;

    // Create a DAMM v2 position
    msg!("Initialize::Creating DAMM v2 position");
    damm_v2::cpi::create_position(CpiContext::new(
        ctx.accounts.amm_program.to_account_info(),
        damm_v2::cpi::accounts::CreatePosition {
            owner: ctx.accounts.owner.to_account_info(),
            position_nft_mint: ctx.accounts.position_nft_mint.to_account_info(),
            position_nft_account: ctx.accounts.position_nft_account.to_account_info(),
            pool: ctx.accounts.pool.to_account_info(),
            position: ctx.accounts.position.to_account_info(),
            pool_authority: ctx.accounts.pool_authority.to_account_info(),
            payer: ctx.accounts.payer.to_account_info(),
            token_program: ctx.accounts.token_2022_program.to_account_info(),
            system_program: ctx.accounts.system_program.to_account_info(),
            event_authority: ctx.accounts.event_authority.to_account_info(),
            program: ctx.accounts.amm_program.to_account_info(),
        },
    ))?;

    // Emit a HonoraryPositionInitialized event
    emit!(HonoraryPositionInitialized {
        vault: ctx.accounts.vault.key(),
        policy: ctx.accounts.policy.key(),
        progress: ctx.accounts.progress.key(),
        pool: ctx.accounts.pool.key(),
        pool_cfg: ctx.accounts.pool_cfg.key(),
        position: ctx.accounts.position.key(),
        owner: ctx.accounts.owner.key(),
        base_mint,
        quote_mint,
        investor_fee_share_bps: params.investor_fee_share_bps,
        min_payout_lamports: params.min_payout_lamports,
        daily_cap: params.daily_cap,
        y0: params.y0,
    });

    msg!("Initialize::Initialization completed successfully");
    Ok(())
}

fn assert_pool(
    pool: &Ref<'_, damm_v2::accounts::Pool>,
    pool_cfg: &Ref<'_, damm_v2::accounts::Config>,
    base_mint: &Pubkey,
    quote_mint: &Pubkey,
) -> Result<()> {
    // Determine base/quote token order
    let base_token_order = utils::token::get_token_order(pool, base_mint);
    let quote_token_order = utils::token::get_token_order(pool, quote_mint);

    // Check base/quote token order is valid
    require!(base_token_order.is_some(), TollgateError::BaseMintNotInPool);
    require!(
        quote_token_order.is_some(),
        TollgateError::QuoteMintNotInPool
    );

    // Ensure base/quote mints are not the same
    // This should be guaranteed by the token order checks above, but we check again for safety
    require!(
        base_token_order != quote_token_order,
        TollgateError::BaseAndQuoteMintsAreSame
    );

    // Ensure quote only fees are enabled
    let collect_fee_mode = match quote_token_order.unwrap() {
        utils::token::TokenOrder::A => 1,
        utils::token::TokenOrder::B => 1,
    };
    require_eq!(
        collect_fee_mode,
        pool.collect_fee_mode,
        TollgateError::PoolNotQuoteOnlyFees
    );
    require_eq!(
        collect_fee_mode,
        pool_cfg.collect_fee_mode,
        TollgateError::PoolConfigNotQuoteOnlyFees
    );

    Ok(())
}
