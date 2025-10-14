#![allow(deprecated)]

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
    pub cursor: u32, // Pagination cursor
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

fn claim_position_fees<'info>(
    ctx: &Context<'_, '_, '_, 'info, AccountCrank<'info>>,
    quote_token_order: utils::token::TokenOrder,
    fee_a_pending: u64,
    fee_b_pending: u64,
    vault_signer: &[&[&[u8]]],
) -> Result<u64> {
    let (base_fee, quote_fee) = match quote_token_order {
        utils::token::TokenOrder::A => (fee_b_pending, fee_a_pending),
        utils::token::TokenOrder::B => (fee_a_pending, fee_b_pending),
    };

    require_eq!(base_fee, 0, TollgateError::BaseDenominatedFees);

    if quote_fee > 0 {
        let token_a_account = match quote_token_order {
            utils::token::TokenOrder::A => ctx.accounts.quote_account.to_account_info(),
            utils::token::TokenOrder::B => ctx.accounts.base_account.to_account_info(),
        };

        let token_b_account = match quote_token_order {
            utils::token::TokenOrder::A => ctx.accounts.base_account.to_account_info(),
            utils::token::TokenOrder::B => ctx.accounts.quote_account.to_account_info(),
        };

        let token_a_vault = match quote_token_order {
            utils::token::TokenOrder::A => ctx.accounts.quote_vault.to_account_info(),
            utils::token::TokenOrder::B => ctx.accounts.base_vault.to_account_info(),
        };

        let token_b_vault = match quote_token_order {
            utils::token::TokenOrder::A => ctx.accounts.base_vault.to_account_info(),
            utils::token::TokenOrder::B => ctx.accounts.quote_vault.to_account_info(),
        };

        let token_a_mint = match quote_token_order {
            utils::token::TokenOrder::A => ctx.accounts.quote_mint.to_account_info(),
            utils::token::TokenOrder::B => ctx.accounts.base_mint.to_account_info(),
        };

        let token_b_mint = match quote_token_order {
            utils::token::TokenOrder::A => ctx.accounts.base_mint.to_account_info(),
            utils::token::TokenOrder::B => ctx.accounts.quote_mint.to_account_info(),
        };

        let token_a_program = match quote_token_order {
            utils::token::TokenOrder::A => ctx.accounts.quote_program.to_account_info(),
            utils::token::TokenOrder::B => ctx.accounts.base_program.to_account_info(),
        };

        let token_b_program = match quote_token_order {
            utils::token::TokenOrder::A => ctx.accounts.base_program.to_account_info(),
            utils::token::TokenOrder::B => ctx.accounts.quote_program.to_account_info(),
        };

        // Claim DAMM v2 position fee
        msg!(
            "Crank::Claiming DAMM v2 position fee: quote_fee={}",
            quote_fee
        );
        damm_v2::cpi::claim_position_fee(CpiContext::new_with_signer(
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
            vault_signer,
        ))?;

        // Emit QuoteFeesClaimed event
        emit!(QuoteFeesClaimed {
            vault: ctx.accounts.policy.vault,
            policy: ctx.accounts.policy.key(),
            progress: ctx.accounts.progress.key(),
            pool: ctx.accounts.pool.key(),
            position: ctx.accounts.position.key(),
            owner: ctx.accounts.owner.key(),
            base_fee_claimed: base_fee,
            quote_fee_claimed: quote_fee
        });
    }

    Ok(quote_fee)
}

pub fn crank<'info>(
    ctx: Context<'_, '_, '_, 'info, AccountCrank<'info>>,
    params: CrankParams,
) -> Result<()> {
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
    let page_size = investor_accounts.len() as u32 / 2;

    msg!(
        "Crank::Starting crank with cursor={} and page_size={}",
        params.cursor,
        page_size
    );

    // Validate params
    params.assert(ctx.accounts.policy.investor_count)?;

    let vault_seeds = &[
        VAULT_SEED,
        ctx.accounts.policy.vault.as_ref(),
        INVESTOR_FEE_POS_OWNER,
        &[ctx.accounts.policy.owner_bump],
    ];
    let vault_signer = &[&vault_seeds[..]];

    let prev_remainder = ctx
        .accounts
        .quote_account
        .amount
        .saturating_sub(ctx.accounts.progress.carry);
    let day = if ctx.accounts.progress.is_new_day(timestamp) {
        // New day
        if ctx.accounts.progress.last_distribution_ts == 0
            || !matches!(ctx.accounts.progress.day_state, DayState::New)
        {
            if prev_remainder != 0 {
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
                anchor_spl::token_interface::transfer(cpi_ctx, prev_remainder)?;
                msg!(
                    "Crank::Transferred previous day remainder to creator: {}",
                    prev_remainder
                );
            }
            ctx.accounts.progress.start_new_day(timestamp)?;
        }
        DayState::New
    } else if matches!(ctx.accounts.progress.day_state, DayState::Closed)
        && ctx.accounts.progress.last_distribution_ts != 0
    {
        // Closed day and not the first time crank is called
        msg!("Crank::Day is closed, skipping");
        return Ok(());
    } else if ctx.accounts.progress.is_same_day(timestamp) {
        // Same day
        if !matches!(ctx.accounts.progress.day_state, DayState::Same) {
            ctx.accounts.progress.continue_same_day()?;
        }
        DayState::Same
    } else {
        // Should not happen
        return Err(TollgateError::InvalidDayState.into());
    };

    msg!("Crank::Processing day state: {:?}", day);

    // Validate progress cursor
    if params.cursor < ctx.accounts.progress.cursor {
        // Idempotent: nothing to do
        msg!("Crank::Cursor behind progress, skipping");
        return Ok(());
    } else if params.cursor > ctx.accounts.progress.cursor {
        // Cannot skip ahead
        return Err(TollgateError::PaginationCursorTooLarge.into());
    }

    // Load the pool and pool config accounts
    let (fee_a_pending, fee_b_pending, _, quote_token_order) = {
        // Load pool and position accounts
        let pool = &ctx.accounts.pool.load()?;
        let position = &ctx.accounts.position.load()?;

        // Determine base/quote mints
        let (base_mint, quote_mint) = if ctx.accounts.policy.quote_mint == pool.token_a_mint {
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

        (
            position.fee_a_pending,
            position.fee_b_pending,
            base_token_order,
            quote_token_order,
        )
    };

    let mut distributable = if matches!(day, DayState::New) {
        let quote_fee = claim_position_fees(
            &ctx,
            quote_token_order.unwrap(),
            fee_a_pending,
            fee_b_pending,
            vault_signer,
        )?;

        quote_fee.saturating_add(
            ctx.accounts
                .quote_account
                .amount
                .saturating_sub(prev_remainder),
        )
    } else {
        ctx.accounts.quote_account.amount
    };

    msg!("Crank::Distributable amount after carry: {}", distributable);

    // Optional daily cap
    if let Some(cap) = ctx.accounts.policy.daily_cap {
        let remaining_cap = cap.saturating_sub(ctx.accounts.progress.daily_spent);
        distributable = distributable.min(remaining_cap);
        msg!(
            "Crank::Applied daily cap, remaining_cap={}, distributable={}",
            remaining_cap,
            distributable
        );
    }

    if matches!(day, DayState::New) && distributable < ctx.accounts.policy.min_payout_lamports {
        ctx.accounts.progress.carry = distributable;
        msg!(
            "Crank::Distributable below min payout, carrying over: {}",
            distributable
        );
        return Ok(());
    }

    if page_size == 0 {
        msg!("Crank::No investors to process, exiting");
        return Ok(());
    }

    let locked_total = {
        let mut total = 0u64;

        for idx in 0..page_size as usize {
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
    let f_locked = (locked_total * MAX_BPS as u64) / ctx.accounts.policy.y0;
    let eligible_investor_share_bps =
        (ctx.accounts.policy.investor_fee_share_bps as u64).min(f_locked);
    let investor_fee_quote = distributable * eligible_investor_share_bps / MAX_BPS as u64;

    msg!(
        "Crank::Locked total: {}, eligible bps: {}, investor fee: {}",
        locked_total,
        eligible_investor_share_bps,
        investor_fee_quote
    );

    let mut page_payouts = 0u64;
    for idx in 0..page_size as usize {
        let investor_idx = idx * 2;
        let stream_ai = &ctx.remaining_accounts[investor_idx];
        let investor_ata_ai = &ctx.remaining_accounts[investor_idx + 1];

        let contract_data = stream_ai.data.borrow();
        let contract = match try_from_slice_unchecked::<Contract>(&contract_data) {
            Ok(v) => v,
            Err(_) => continue,
        };

        let recipient = contract.recipient;
        let expected_ata =
            get_associated_token_address(&recipient, &ctx.accounts.policy.quote_mint);
        require_keys_eq!(
            investor_ata_ai.key(),
            expected_ata,
            TollgateError::InvalidInvestorAta
        );

        if investor_ata_ai.data_len() != token::TokenAccount::LEN {
            if ctx.accounts.policy.init_investor_ata {
                // TODO: init invesstor ATA
                continue;

                // let cpi_accounts = associated_token::Create {
                //     payer: ctx.accounts.payer.to_account_info(),
                //     associated_token: investor_ata_ai.clone(), // Mut
                //     authority: authority.clone(),              // Read
                //     mint: ctx.accounts.quote_mint.to_account_info(),
                //     system_program: ctx.accounts.system_program.to_account_info(),
                //     token_program: ctx.accounts.quote_program.to_account_info(),
                //     // rent: Rent::default(), // Sysvar, but Anchor CPI handles it implicitly if not passed
                // };
                // let cpi_program = ctx.accounts.associated_token_program.to_account_info();
                // let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
                // associated_token::create_idempotent(cpi_ctx)?; // Create idempotent for safety
                //
                // let ata_data = investor_ata_ai.data.borrow();
                // token::TokenAccount::try_deserialize(&mut &ata_data[..])?
            } else {
                continue;
            }
        }

        let net = contract.ix.net_amount_deposited;
        let avail = contract.available_to_claim(timestamp as u64, 0.0);
        let locked = net.saturating_sub(avail);

        let investor_share = if locked_total > 0 {
            (investor_fee_quote * locked) / locked_total
        } else {
            0
        };
        if investor_share >= ctx.accounts.policy.min_payout_lamports {
            let cpi_accounts = token_interface::Transfer {
                from: ctx.accounts.quote_account.to_account_info(),
                to: investor_ata_ai.to_account_info(),
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

    ctx.accounts.progress.daily_spent += page_payouts;
    ctx.accounts.progress.cursor += page_size;

    let page_start = params.cursor as usize;
    let page_end =
        (page_start + page_size as usize).min(ctx.accounts.policy.investor_count as usize);

    msg!(
        "Crank::Processed page {} to {}, payouts: {}",
        page_start,
        page_end,
        page_payouts
    );

    emit!(InvestorPayoutPage {
        vault: ctx.accounts.policy.vault,
        policy: ctx.accounts.policy.key(),
        progress: ctx.accounts.progress.key(),
        pool: ctx.accounts.pool.key(),
        position: ctx.accounts.position.key(),
        owner: ctx.accounts.owner.key(),
        cursor: params.cursor,
        investors: page_size,
        page_start: page_start as u32,
        page_end: page_end as u32,
        payout: page_payouts
    });

    if ctx.accounts.progress.cursor >= ctx.accounts.policy.investor_count {
        let creator_share = distributable.saturating_sub(investor_fee_quote);
        if creator_share >= ctx.accounts.policy.min_payout_lamports {
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
            ctx.accounts.progress.carry += creator_share;
            msg!(
                "Crank::Creator share below min, carrying over: {}",
                creator_share
            );
        }

        let total_distributed = distributable.saturating_add(
            ctx.accounts
                .progress
                .daily_spent
                .saturating_sub(page_payouts),
        );

        emit!(CreatorPayoutDayClosed {
            vault: ctx.accounts.policy.vault,
            policy: ctx.accounts.policy.key(),
            progress: ctx.accounts.progress.key(),
            pool: ctx.accounts.pool.key(),
            position: ctx.accounts.position.key(),
            owner: ctx.accounts.owner.key(),
            timestamp,
            total_distributed,
            creator_payout: creator_share,
            carry: ctx.accounts.progress.carry
        });

        msg!(
            "Crank::Day closed, total distributed: {}, carry: {}",
            total_distributed,
            ctx.accounts.progress.carry
        );

        ctx.accounts.progress.close_day()?;
    }

    msg!("Crank::Completed successfully");
    Ok(())
}
