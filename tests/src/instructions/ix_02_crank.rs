use anchor_client::{
    anchor_lang::{prelude::AccountMeta, InstructionData, ToAccountMetas},
    solana_sdk::{
        compute_budget::ComputeBudgetInstruction, instruction::Instruction,
        native_token::LAMPORTS_PER_SOL, pubkey::Pubkey, signer::Signer, system_program,
    },
};
use anchor_spl::{
    associated_token::{
        get_associated_token_address, get_associated_token_address_with_program_id,
        spl_associated_token_account::{
            self, instruction::create_associated_token_account_idempotent,
        },
    },
    token::spl_token,
};
use tollgate::{
    accounts::AccountCrank,
    constants::{
        damm_v2_constants, INVESTOR_FEE_POS_OWNER, POLICY_SEED, PROGRESS_SEED, TWENTY_FOUR_HOURS,
        VAULT_SEED,
    },
    error::TollgateError,
    state::Policy,
};

use crate::utils::{
    damm_v2::{
        get_pool_with_config_pda, get_position_nft_account_pda, get_position_pda,
        get_token_vault_pda, set_damm_v2_position_fees,
    },
    find_program_address, find_program_event_authority, log_policy_account, log_progress_account,
    svm::{
        demand_instruction_error, demand_logs_contain, get_ix_err, get_payer, TestContext, Token,
    },
};

#[allow(clippy::too_many_arguments)]
pub fn get_crank_ix_accs(
    ctx: &TestContext,
    vault: Pubkey,
    pool: Pubkey,
    position_nft_account: Pubkey,
    position: Pubkey,
    pool_authority: Pubkey,
    base_mint: Pubkey,
    quote_mint: Pubkey,
    payer: Pubkey,
    event_authority: Pubkey,
) -> AccountCrank {
    let policy = find_program_address(&[POLICY_SEED, vault.as_ref()], None).0;
    let owner = find_program_address(&[VAULT_SEED, vault.as_ref(), INVESTOR_FEE_POS_OWNER], None).0;

    let policy_program_acc = ctx.get_program_account::<Policy>(&policy);
    let base_mint_acc = ctx
        .svm
        .get_account(&base_mint)
        .expect("Base mint account not found");
    let quote_mint_acc = ctx
        .svm
        .get_account(&quote_mint)
        .expect("Quote mint account not found");

    AccountCrank {
        policy,
        progress: find_program_address(&[PROGRESS_SEED, vault.as_ref()], None).0,
        pool,
        position_nft_account,
        position,
        pool_authority,
        owner,
        base_account: get_associated_token_address_with_program_id(
            &owner,
            &base_mint,
            &base_mint_acc.owner,
        ),
        quote_account: get_associated_token_address_with_program_id(
            &owner,
            &quote_mint,
            &quote_mint_acc.owner,
        ),
        base_vault: get_token_vault_pda(base_mint, pool).0,
        quote_vault: get_token_vault_pda(quote_mint, pool).0,
        base_mint,
        quote_mint,
        base_program: base_mint_acc.owner,
        quote_program: quote_mint_acc.owner,
        creator_accoount: get_associated_token_address_with_program_id(
            &policy_program_acc.creator,
            &quote_mint,
            &quote_mint_acc.owner,
        ),
        payer,
        event_authority,
        amm_program: damm_v2::ID,
        associated_token_program: spl_associated_token_account::ID,
        system_program: system_program::ID,
    }
}

pub fn crank_ix(
    accounts: impl ToAccountMetas,
    args: tollgate::instruction::Crank,
    remaining_accounts: Vec<AccountMeta>,
) -> Instruction {
    let mut accounts = accounts.to_account_metas(None);
    accounts.extend(remaining_accounts);

    Instruction::new_with_bytes(tollgate::ID, &args.data(), accounts)
}

pub fn crank_with_init_ix(
    accounts: impl ToAccountMetas,
    args: tollgate::instruction::CrankWithInit,
    remaining_accounts: Vec<AccountMeta>,
) -> Instruction {
    let mut accounts = accounts.to_account_metas(None);
    accounts.extend(remaining_accounts);

    Instruction::new_with_bytes(tollgate::ID, &args.data(), accounts)
}

pub fn compute_crank_ix_accs<'a>(
    ctx: &'a TestContext,
    key: &str,
    pos_key: &str,
    init_mode: bool,
    payer: Pubkey,
    start_page: u32,
    end_page: u32,
) -> (&'a Token, (impl ToAccountMetas, Vec<AccountMeta>)) {
    let key = String::from(key);
    let token = ctx.tokens.get(&key).expect("Token not found in context");
    let base_mint = token.base_mint.pubkey();
    let quote_mint = token.quote_mint;
    let pos_mint = token
        .pos_mints
        .get(pos_key)
        .expect("Position mint not found in context");
    let (position_nft_account, _) = get_position_nft_account_pda(pos_mint.pubkey());
    let (pool, _) = get_pool_with_config_pda(token.pool_config, base_mint, quote_mint);
    let (position, _) = get_position_pda(pos_mint.pubkey());
    let pool_authority = damm_v2_constants::pool_authority::ID;

    let mut remaining_accounts = vec![];
    for idx in start_page..end_page {
        let investor = token
            .investors
            .get(idx as usize)
            .expect("Investor not found in token investors");
        if init_mode {
            remaining_accounts.push(AccountMeta::new_readonly(investor.key.pubkey(), false));
        }
        remaining_accounts.push(AccountMeta::new_readonly(investor.stream.pubkey(), false));
        remaining_accounts.push(AccountMeta::new(
            get_associated_token_address(&investor.key.pubkey(), &quote_mint),
            false,
        ));
    }

    let accounts = get_crank_ix_accs(
        ctx,
        token.vault.pubkey(),
        pool,
        position_nft_account,
        position,
        pool_authority,
        base_mint,
        quote_mint,
        payer,
        find_program_event_authority(&damm_v2::ID).0,
    );

    (token, (accounts, remaining_accounts))
}

#[test]
fn test_01_crank_below_min_payout() {
    let mut ctx = TestContext::default();
    let key = "tollgate";
    let pos_key = "initialize";
    let payer = get_payer();
    let (token, accs) = compute_crank_ix_accs(&ctx, key, pos_key, false, payer.pubkey(), 0, 10);
    let base_mint = token.base_mint.pubkey();
    let quote_mint = token.quote_mint;
    let pool_authority = damm_v2_constants::pool_authority::ID;

    let result = ctx.send_transaction(
        &[
            create_associated_token_account_idempotent(
                &payer.pubkey(),
                &pool_authority,
                &base_mint,
                &spl_token::ID,
            ),
            create_associated_token_account_idempotent(
                &payer.pubkey(),
                &pool_authority,
                &quote_mint,
                &spl_token::ID,
            ),
            crank_ix(
                accs.0,
                tollgate::instruction::Crank {
                    params: tollgate::instructions::CrankParams { cursor: 0 },
                },
                accs.1,
            ),
        ],
        Some(&payer.pubkey()),
        &[payer],
    );

    demand_logs_contain("Crank::Processing day state: New", &result);
    demand_logs_contain("Crank::Distributable amount after carry: 0", &result);
    demand_logs_contain(
        "Crank::Distributable below min payout, carrying over: 0",
        &result,
    );
}

#[test]
fn test_02_should_failed_base_fee_detected() {
    let mut ctx = TestContext::default();
    let key = "tollgate";
    let pos_key = "initialize";
    let payer = get_payer();
    let (_, accs) = compute_crank_ix_accs(&ctx, key, pos_key, false, payer.pubkey(), 0, 10);

    set_damm_v2_position_fees(&mut ctx, key, pos_key, Some(1), None);

    let result = ctx.send_transaction(
        &[crank_ix(
            accs.0,
            tollgate::instruction::Crank {
                params: tollgate::instructions::CrankParams { cursor: 0 },
            },
            accs.1,
        )],
        Some(&payer.pubkey()),
        &[payer],
    );

    demand_instruction_error(get_ix_err(TollgateError::BaseDenominatedFees), &result);
}

#[test]
fn test_03_crank_claim_quote_fees() {
    let mut ctx = TestContext::default();
    let key = "tollgate";
    let pos_key = "initialize";
    let payer = get_payer();
    let quote_fee = LAMPORTS_PER_SOL;

    ctx.time_travel_by_secs(TWENTY_FOUR_HOURS as u64);
    set_damm_v2_position_fees(&mut ctx, key, pos_key, Some(0), Some(quote_fee));
    let (_, accs) = compute_crank_ix_accs(&ctx, key, pos_key, false, payer.pubkey(), 0, 0);

    let result = ctx.send_transaction(
        &[crank_ix(
            accs.0,
            tollgate::instruction::Crank {
                params: tollgate::instructions::CrankParams { cursor: 0 },
            },
            accs.1,
        )],
        Some(&payer.pubkey()),
        &[payer],
    );

    demand_logs_contain(
        "Crank::Starting crank with cursor=0 and page_size=0",
        &result,
    );
    demand_logs_contain("Crank::Processing day state: New", &result);
    demand_logs_contain(
        format!(
            "Crank::Claiming DAMM v2 position fee: quote_fee={}",
            quote_fee
        )
        .as_str(),
        &result,
    );
    demand_logs_contain(
        format!("Crank::Distributable amount after carry: {}", quote_fee).as_str(),
        &result,
    );
    demand_logs_contain("Crank::No investors to process, exiting", &result);

    log_policy_account(&ctx, key);
    log_progress_account(&ctx, key);
}

#[test]
fn test_04_create_investors_ata() {
    let mut ctx = TestContext::default();
    let key = "tollgate";
    let payer = get_payer();
    let tokens = ctx.tokens.clone();
    let token = tokens.get(key).expect("Token not found in context");

    // Manually init first ten investors ATA to test normal crank IX
    let mut create_ata_ixs = vec![];
    for investor in token.investors.split_at(10).0.iter() {
        create_ata_ixs.push(create_associated_token_account_idempotent(
            &payer.pubkey(),
            &investor.key.pubkey(),
            &token.quote_mint,
            &spl_token::ID,
        ));
    }
    ctx.send_transaction(create_ata_ixs.as_slice(), Some(&payer.pubkey()), &[payer])
        .expect("Creating investors ATA should succeed");
}

#[test]
fn test_05_crank_page_0_to_10() {
    let mut ctx = TestContext::default();
    let key = "tollgate";
    let pos_key = "initialize";
    let payer = get_payer();

    set_damm_v2_position_fees(&mut ctx, key, pos_key, Some(0), Some(LAMPORTS_PER_SOL / 2));
    let (_, accs) = compute_crank_ix_accs(&ctx, key, pos_key, false, payer.pubkey(), 0, 10);

    let result = ctx.send_transaction(
        &[
            ComputeBudgetInstruction::set_compute_unit_price(1), // Use as a nonce
            crank_ix(
                accs.0,
                tollgate::instruction::Crank {
                    params: tollgate::instructions::CrankParams { cursor: 0 },
                },
                accs.1,
            ),
        ],
        Some(&payer.pubkey()),
        &[payer],
    );

    demand_logs_contain("Crank::Processing day state: Same", &result);
    demand_logs_contain(
        format!(
            "Crank::Distributable amount after carry: {}",
            LAMPORTS_PER_SOL
        )
        .as_str(),
        &result,
    );
    demand_logs_contain("Crank::Processed page 0 to 10", &result);
    demand_logs_contain("Crank::Completed successfully", &result);

    log_progress_account(&ctx, key);
}

#[test]
fn test_06_crank_page_0_to_10_idempotent() {
    let mut ctx = TestContext::default();
    let key = "tollgate";
    let pos_key = "initialize";
    let payer = get_payer();

    let (_, accs) = compute_crank_ix_accs(&ctx, key, pos_key, false, payer.pubkey(), 0, 10);

    let result = ctx.send_transaction(
        &[
            ComputeBudgetInstruction::set_compute_unit_price(2), // Use as a nonce
            crank_ix(
                accs.0,
                tollgate::instruction::Crank {
                    params: tollgate::instructions::CrankParams { cursor: 0 },
                },
                accs.1,
            ),
        ],
        Some(&payer.pubkey()),
        &[payer],
    );

    demand_logs_contain("Crank::Processing day state: Same", &result);
    demand_logs_contain("Crank::Cursor behind progress, skipping", &result);

    log_progress_account(&ctx, key);
}

#[test]
fn test_07_crank_page_10_to_20() {
    let mut ctx = TestContext::default();
    let key = "tollgate";
    let pos_key = "initialize";
    let payer = get_payer();

    let (_, accs) = compute_crank_ix_accs(&ctx, key, pos_key, true, payer.pubkey(), 10, 20);

    let result = ctx.send_transaction(
        &[
            ComputeBudgetInstruction::set_compute_unit_limit(500_000),
            crank_with_init_ix(
                accs.0,
                tollgate::instruction::CrankWithInit {
                    params: tollgate::instructions::CrankParams { cursor: 10 },
                },
                accs.1,
            ),
        ],
        Some(&payer.pubkey()),
        &[payer],
    );

    demand_logs_contain("Crank::Processing day state: Same", &result);
    demand_logs_contain("Crank::Distributable amount after carry: ", &result);
    result.expect("Crank page 10 to 20 should succeed");

    log_progress_account(&ctx, key);
}

#[test]
fn test_08_crank_day_two_page_1_to_5_invalid_cursor() {
    let mut ctx = TestContext::default();
    let key = "tollgate";
    let pos_key = "initialize";
    let payer = get_payer();

    ctx.time_travel_by_secs(TWENTY_FOUR_HOURS as u64);
    let (_, accs) = compute_crank_ix_accs(&ctx, key, pos_key, true, payer.pubkey(), 1, 6);

    let result = ctx.send_transaction(
        &[crank_with_init_ix(
            accs.0,
            tollgate::instruction::CrankWithInit {
                params: tollgate::instructions::CrankParams { cursor: 1 },
            },
            accs.1,
        )],
        Some(&payer.pubkey()),
        &[payer],
    );

    demand_logs_contain(
        "Crank::Starting crank with cursor=1 and page_size=5",
        &result,
    );
    demand_logs_contain(
        "Crank::Transferred previous day remainder to creator: ",
        &result,
    );
    demand_logs_contain("Crank::Processing day state: New", &result);
    demand_instruction_error(get_ix_err(TollgateError::PaginationCursorTooLarge), &result);

    log_progress_account(&ctx, key);
}

#[test]
fn test_09_crank_day_two_page_0_to_8() {
    let mut ctx = TestContext::default();
    let key = "tollgate";
    let pos_key = "initialize";
    let payer = get_payer();

    let (_, accs) = compute_crank_ix_accs(&ctx, key, pos_key, true, payer.pubkey(), 0, 8);

    let result = ctx.send_transaction(
        &[
            ComputeBudgetInstruction::set_compute_unit_limit(600_000),
            crank_with_init_ix(
                accs.0,
                tollgate::instruction::CrankWithInit {
                    params: tollgate::instructions::CrankParams { cursor: 0 },
                },
                accs.1,
            ),
        ],
        Some(&payer.pubkey()),
        &[payer],
    );

    let quote_fee = LAMPORTS_PER_SOL / 2;
    demand_logs_contain(
        "Crank::Starting crank with cursor=0 and page_size=8",
        &result,
    );
    demand_logs_contain(
        "Crank::Transferred previous day remainder to creator: ",
        &result,
    );
    demand_logs_contain("Crank::Processing day state: New", &result);
    demand_logs_contain(
        format!(
            "Crank::Claiming DAMM v2 position fee: quote_fee={}",
            quote_fee
        )
        .as_str(),
        &result,
    );
    demand_logs_contain(
        format!("Crank::Distributable amount after carry: {}", quote_fee).as_str(),
        &result,
    );
    demand_logs_contain("Crank::Processed page 0 to 8, payouts: ", &result);
    demand_logs_contain("Crank::Completed successfully", &result);

    log_progress_account(&ctx, key);
}

#[test]
fn test_10_crank_day_two_full() {
    let mut ctx = TestContext::default();
    let key = "tollgate";
    let pos_key = "initialize";
    let payer = get_payer();

    let tokens = ctx.tokens.clone();
    let token = tokens.get(key).expect("Token not found in context");
    let investors = token.investors.split_at(8).1.chunks(10);

    for (idx, chunk) in investors.clone().enumerate() {
        let len = chunk.len();
        let start_page = if idx + 1 == investors.len() {
            8 + ((investors.len() - 1) * 10)
        } else {
            8 + (idx * len)
        };
        let end_page = start_page + len;

        let (_, accs) = compute_crank_ix_accs(
            &ctx,
            key,
            pos_key,
            true,
            payer.pubkey(),
            start_page as u32,
            end_page as u32,
        );

        let result = ctx.send_transaction(
            &[
                ComputeBudgetInstruction::set_compute_unit_limit(700_000),
                crank_with_init_ix(
                    accs.0,
                    tollgate::instruction::CrankWithInit {
                        params: tollgate::instructions::CrankParams {
                            cursor: start_page as u32,
                        },
                    },
                    accs.1,
                ),
            ],
            Some(&payer.pubkey()),
            &[payer],
        );

        demand_logs_contain(
            format!(
                "Crank::Starting crank with cursor={} and page_size={}",
                start_page, len
            )
            .as_str(),
            &result,
        );
        demand_logs_contain("Crank::Processing day state: Same", &result);
        demand_logs_contain("Crank::Distributable amount after carry: ", &result);
        result.expect("Crank day two full should succeed");
    }

    log_progress_account(&ctx, key);
}

#[test]
fn test_11_crank_day_two_full_idempotent() {
    let mut ctx = TestContext::default();
    let key = "tollgate";
    let pos_key = "initialize";
    let payer = get_payer();

    let tokens = ctx.tokens.clone();
    let token = tokens.get(key).expect("Token not found in context");
    let start_page = token.investors.len() as u32 - 10;
    let end_page = token.investors.len() as u32;
    let (_, accs) = compute_crank_ix_accs(
        &ctx,
        key,
        pos_key,
        true,
        payer.pubkey(),
        start_page,
        end_page,
    );

    let result = ctx.send_transaction(
        &[
            ComputeBudgetInstruction::set_compute_unit_limit(800_000),
            crank_with_init_ix(
                accs.0,
                tollgate::instruction::CrankWithInit {
                    params: tollgate::instructions::CrankParams { cursor: start_page },
                },
                accs.1,
            ),
        ],
        Some(&payer.pubkey()),
        &[payer],
    );

    demand_logs_contain(
        format!(
            "Crank::Starting crank with cursor={} and page_size=10",
            start_page
        )
        .as_str(),
        &result,
    );
    demand_logs_contain("Crank::Day is closed, skipping", &result);

    log_progress_account(&ctx, key);
}

#[test]
fn test_12_crank_day_three_full() {
    let mut ctx = TestContext::default();
    let key = "tollgate";
    let pos_key = "initialize";
    let payer = get_payer();
    let quote_fee = (LAMPORTS_PER_SOL as f64 / 1.6) as u64;

    ctx.time_travel_by_secs(TWENTY_FOUR_HOURS as u64);
    set_damm_v2_position_fees(&mut ctx, key, pos_key, Some(0), Some(quote_fee));

    let tokens = ctx.tokens.clone();
    let token = tokens.get(key).expect("Token not found in context");
    let investors = token.investors.chunks(10);

    for (idx, chunk) in investors.clone().enumerate() {
        let len = chunk.len();
        let start_page = if idx + 1 == investors.len() {
            (investors.len() - 1) * 10
        } else {
            idx * len
        };
        let end_page = start_page + len;

        let (_, accs) = compute_crank_ix_accs(
            &ctx,
            key,
            pos_key,
            true,
            payer.pubkey(),
            start_page as u32,
            end_page as u32,
        );

        let result = ctx.send_transaction(
            &[
                ComputeBudgetInstruction::set_compute_unit_limit(900_000),
                crank_with_init_ix(
                    accs.0,
                    tollgate::instruction::CrankWithInit {
                        params: tollgate::instructions::CrankParams {
                            cursor: start_page as u32,
                        },
                    },
                    accs.1,
                ),
            ],
            Some(&payer.pubkey()),
            &[payer],
        );

        demand_logs_contain(
            format!(
                "Crank::Starting crank with cursor={} and page_size={}",
                start_page, len
            )
            .as_str(),
            &result,
        );
        if idx == 0 {
            demand_logs_contain("Crank::Processing day state: New", &result);
        } else {
            demand_logs_contain("Crank::Processing day state: Same", &result);
        }
        demand_logs_contain("Crank::Distributable amount after carry: ", &result);
        result.expect("Crank day three full should succeed");
    }

    log_progress_account(&ctx, key);
}
