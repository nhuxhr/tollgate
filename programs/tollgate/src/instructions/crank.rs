use anchor_lang::{prelude::*, solana_program::borsh::try_from_slice_unchecked};
use anchor_spl::{associated_token::get_associated_token_address, token, token_interface};
use streamflow_sdk::state::Contract;

use crate::{
    constants::{INVESTOR_FEE_POS_OWNER, MAX_BPS, VAULT_SEED},
    error::TollgateError,
    events::{CreatorPayoutDayClosed, InvestorPayoutPage, QuoteFeesClaimed},
    state::DayState,
    utils, AccountCrank,
};

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, Default)]
pub struct CrankParams {
    pub cursor: u32,    // Pagination cursor
    pub page_size: u32, //
}

impl CrankParams {
    pub fn assert(&self, investors: u32) -> Result<()> {
        // assert cursor is not excessively investors
        require!(
            self.cursor < investors,
            TollgateError::CursorExceedsInvestors
        );
        Ok(())
    }
}

pub fn crank<'info>(
    ctx: Context<'_, '_, '_, 'info, AccountCrank<'info>>,
    params: CrankParams,
) -> Result<()> {
    msg!(
        "Crank::Starting crank with cursor={} and page_size={}",
        params.cursor,
        params.page_size
    );

    let policy = &mut ctx.accounts.policy;
    let progress = &mut ctx.accounts.progress;
    let clock = Clock::get()?;
    let timestamp = clock.unix_timestamp;

    // Paged investor accounts: Streamflow stream pubkey and investor quote ATA
    let investor_accounts = ctx.remaining_accounts;
    require_eq!(
        0,
        investor_accounts.len() % 2,
        TollgateError::InvalidInvestorAccounts
    );

    // Paginated pro-rata via remaining_accounts (pairs: stream, investor_ata)
    let investors = investor_accounts.len() as u32 / 2;

    // Validate params
    params.assert(investors)?;

    // Validate progress cursor
    if params.cursor < progress.cursor {
        // Idempotent: nothing to do
        msg!("Crank::Cursor behind progress, skipping");
        return Ok(());
    } else if params.cursor > progress.cursor {
        // Cannot skip ahead
        return Err(TollgateError::PaginationCursorTooLarge.into());
    }

    if investors.saturating_sub(params.cursor) == 0 {
        msg!("Crank::No investors to process, exiting");
        return Ok(());
    }

    let day = if progress.is_new_day(timestamp) {
        // New day
        if !matches!(progress.day_state, DayState::New) {
            progress.start_new_day(timestamp)?;
        }
        DayState::New
    } else if progress.is_same_day(timestamp) {
        // Same day
        if !matches!(progress.day_state, DayState::Same) {
            progress.continue_same_day()?;
        }
        DayState::Same
    } else {
        // Should not happen
        return Err(TollgateError::InvalidDayState.into());
    };

    msg!("Crank::Processing day state: {:?}", day);

    // Load pool and position accounts
    let pool = &ctx.accounts.pool.load()?;
    let position = &ctx.accounts.position.load()?;

    // Determine base/quote mints
    let (base_mint, quote_mint) = if policy.quote_mint == pool.token_a_mint {
        (&pool.token_b_mint, &pool.token_a_mint)
    } else {
        (&pool.token_a_mint, &pool.token_b_mint)
    };

    // Determine base/quote token order
    let base_token_order = utils::token::get_token_order(pool, base_mint);
    let quote_token_order = utils::token::get_token_order(pool, quote_mint);

    // Check base/quote token order is valid
    require!(base_token_order.is_some(), TollgateError::BaseMintNotInPool);
    require!(
        quote_token_order.is_some(),
        TollgateError::QuoteMintNotInPool
    );

    let mut distributable = if matches!(day, DayState::New) {
        let (base_fee, quote_fee) = match quote_token_order.unwrap() {
            utils::token::TokenOrder::A => (position.fee_b_pending, position.fee_a_pending),
            utils::token::TokenOrder::B => (position.fee_a_pending, position.fee_b_pending),
        };

        require_eq!(base_fee, 0, TollgateError::BaseDenominatedFees);

        if quote_fee > 0 {
            let (token_a_account, token_b_account) = match quote_token_order.unwrap() {
                utils::token::TokenOrder::A => (
                    ctx.accounts.quote_account.to_account_info(),
                    ctx.accounts.base_account.to_account_info(),
                ),
                utils::token::TokenOrder::B => (
                    ctx.accounts.base_account.to_account_info(),
                    ctx.accounts.quote_account.to_account_info(),
                ),
            };

            let (token_a_vault, token_b_vault) = match quote_token_order.unwrap() {
                utils::token::TokenOrder::A => (
                    ctx.accounts.quote_vault.to_account_info(),
                    ctx.accounts.base_vault.to_account_info(),
                ),
                utils::token::TokenOrder::B => (
                    ctx.accounts.base_vault.to_account_info(),
                    ctx.accounts.quote_vault.to_account_info(),
                ),
            };

            let (token_a_mint, token_b_mint) = match quote_token_order.unwrap() {
                utils::token::TokenOrder::A => (
                    ctx.accounts.quote_mint.to_account_info(),
                    ctx.accounts.base_mint.to_account_info(),
                ),
                utils::token::TokenOrder::B => (
                    ctx.accounts.base_mint.to_account_info(),
                    ctx.accounts.quote_mint.to_account_info(),
                ),
            };

            let (token_a_program, token_b_program) = match quote_token_order.unwrap() {
                utils::token::TokenOrder::A => (
                    ctx.accounts.quote_program.to_account_info(),
                    ctx.accounts.base_program.to_account_info(),
                ),
                utils::token::TokenOrder::B => (
                    ctx.accounts.base_program.to_account_info(),
                    ctx.accounts.quote_program.to_account_info(),
                ),
            };

            // Claim DAMM v2 position fee
            msg!(
                "Crank::Claiming DAMM v2 position fee: quote_fee={}",
                quote_fee
            );
            damm_v2::cpi::claim_position_fee(CpiContext::new(
                ctx.accounts.amm_program.to_account_info(),
                damm_v2::cpi::accounts::ClaimPositionFee {
                    pool_authority: ctx.accounts.pool_authority.to_account_info(),
                    pool: ctx.accounts.pool.to_account_info(),
                    position: ctx.accounts.position.to_account_info(),
                    token_a_account,
                    token_b_account,
                    token_a_vault,
                    token_b_vault,
                    token_a_mint,
                    token_b_mint,
                    position_nft_account: ctx.accounts.position_nft_account.to_account_info(),
                    owner: ctx.accounts.owner.to_account_info(),
                    token_a_program,
                    token_b_program,
                    event_authority: ctx.accounts.event_authority.to_account_info(),
                    program: ctx.accounts.amm_program.to_account_info(),
                },
            ))?;

            // Emit QuoteFeesClaimed event
            emit!(QuoteFeesClaimed {
                vault: policy.vault,
                policy: policy.key(),
                progress: progress.key(),
                pool: ctx.accounts.pool.key(),
                position: ctx.accounts.position.key(),
                owner: ctx.accounts.owner.key(),
                base_fee_claimed: base_fee,
                quote_fee_claimed: quote_fee
            });
        }

        quote_fee.saturating_add(progress.carry)
    } else {
        progress.carry
    };

    msg!("Crank::Distributable amount after carry: {}", distributable);

    // Optional daily cap
    if let Some(cap) = policy.daily_cap {
        let remaining_cap = cap.saturating_sub(progress.daily_spent);
        distributable = distributable.min(remaining_cap);
        msg!(
            "Crank::Applied daily cap, remaining_cap={}, distributable={}",
            remaining_cap,
            distributable
        );
    }

    if distributable < policy.min_payout_lamports {
        progress.carry = distributable;
        msg!(
            "Crank::Distributable below min payout, carrying over: {}",
            distributable
        );
        return Ok(());
    }

    let locked_total = {
        let mut total = 0u64;

        for idx in 0..investors as usize {
            let investor_idx = idx * 2;
            let stream = &ctx.remaining_accounts[investor_idx];
            let contract = match try_from_slice_unchecked::<Contract>(&stream.data.borrow()) {
                Ok(v) => v,
                Err(_) => continue,
            };
            let net = contract.ix.net_amount_deposited;
            let avail = contract.available_to_claim(timestamp as u64, 0.0);
            let locked = net.saturating_sub(avail);
            total = total.saturating_add(locked);
        }

        total
    };
    let f_locked = (locked_total * MAX_BPS as u64) / policy.y0;
    let eligible_investor_share_bps = (policy.investor_fee_share_bps as u64).min(f_locked);
    let investor_fee_quote = distributable * eligible_investor_share_bps / MAX_BPS as u64;

    msg!(
        "Crank::Locked total: {}, eligible bps: {}, investor fee: {}",
        locked_total,
        eligible_investor_share_bps,
        investor_fee_quote
    );

    let page_start = (params.cursor / 2) as usize;
    let page_end = (page_start + params.page_size as usize).min(investors as usize);

    let vault_seeds = &[
        VAULT_SEED,
        policy.vault.as_ref(),
        INVESTOR_FEE_POS_OWNER,
        &[policy.owner_bump],
    ];
    let vault_signer = &[&vault_seeds[..]];

    let mut page_payouts = 0u64;
    for idx in page_start..page_end {
        let investor_idx = idx * 2;
        let stream_ai = &ctx.remaining_accounts[investor_idx];
        let investor_ata_ai = &ctx.remaining_accounts[investor_idx + 1];

        let contract_data = stream_ai.data.borrow();
        let contract = match try_from_slice_unchecked::<Contract>(&contract_data) {
            Ok(v) => v,
            Err(_) => continue,
        };

        let recipient = contract.recipient;
        let expected_ata = get_associated_token_address(&recipient, &policy.quote_mint);
        require_keys_eq!(
            investor_ata_ai.key(),
            expected_ata,
            TollgateError::InvalidInvestorAta
        );

        let ata_data = investor_ata_ai.data.borrow();
        let _token_account = match token::TokenAccount::try_deserialize(&mut &ata_data[..]) {
            Ok(t) => t,
            Err(_) => {
                if policy.init_investor_ata {
                    // TODO: init invesstor ATA
                    // return Err(TollgateError::UninitializedInvestorAta.into());
                    continue;
                } else {
                    continue;
                }
            }
        };

        let net = contract.ix.net_amount_deposited;
        let avail = contract.available_to_claim(timestamp as u64, 0.0);
        let locked = net.saturating_sub(avail);

        let investor_share = if locked_total > 0 {
            (investor_fee_quote * locked) / locked_total
        } else {
            0
        };
        if investor_share >= policy.min_payout_lamports {
            let cpi_accounts = token_interface::Transfer {
                from: ctx.accounts.quote_account.to_account_info(),
                to: investor_ata_ai.clone(),
                authority: ctx.accounts.owner.to_account_info(),
            };
            let cpi_ctx = CpiContext::new_with_signer(
                ctx.accounts.quote_program.to_account_info(),
                cpi_accounts,
                vault_signer,
            );
            anchor_spl::token_interface::transfer(cpi_ctx, investor_share)?;
            page_payouts = page_payouts.saturating_add(investor_share);
        }
    }

    progress.daily_spent += page_payouts;
    progress.cursor += ((page_end - page_start) as u32) * 2;

    msg!(
        "Crank::Processed page {} to {}, payouts: {}",
        page_start,
        page_end,
        page_payouts
    );

    emit!(InvestorPayoutPage {
        vault: policy.vault,
        policy: policy.key(),
        progress: progress.key(),
        pool: ctx.accounts.pool.key(),
        position: ctx.accounts.position.key(),
        owner: ctx.accounts.owner.key(),
        cursor: params.cursor,
        investors,
        page_start: page_start as u32,
        page_end: page_end as u32,
        payout: page_payouts
    });

    if progress.cursor >= (investors * 2) {
        let creator_share = distributable.saturating_sub(investor_fee_quote);
        if creator_share >= policy.min_payout_lamports {
            let cpi_accounts = token_interface::Transfer {
                from: ctx.accounts.quote_account.to_account_info(),
                to: ctx.accounts.creator_accoount.to_account_info(),
                authority: ctx.accounts.owner.to_account_info(),
            };
            let cpi_ctx = CpiContext::new_with_signer(
                ctx.accounts.quote_program.to_account_info(),
                cpi_accounts,
                vault_signer,
            );
            anchor_spl::token_interface::transfer(cpi_ctx, creator_share)?;
            msg!("Crank::Transferred creator share: {}", creator_share);
        } else {
            progress.carry += creator_share;
            msg!(
                "Crank::Creator share below min, carrying over: {}",
                creator_share
            );
        }

        emit!(CreatorPayoutDayClosed {
            vault: policy.vault,
            policy: policy.key(),
            progress: progress.key(),
            pool: ctx.accounts.pool.key(),
            position: ctx.accounts.position.key(),
            owner: ctx.accounts.owner.key(),
            timestamp,
            total_distributed: distributable,
            creator_payout: creator_share,
            carry: progress.carry
        });

        msg!(
            "Crank::Day closed, total distributed: {}, carry: {}",
            distributable,
            progress.carry
        );
    }

    msg!("Crank::Completed successfully");
    Ok(())
}
