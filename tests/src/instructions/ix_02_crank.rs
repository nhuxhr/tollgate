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
        spl_associated_token_account::{self, instruction::create_associated_token_account},
    },
    token::spl_token,
};
use tollgate::{
    accounts::AccountCrank,
    constants::{
        damm_v2_constants, INVESTOR_FEE_POS_OWNER, POLICY_SEED, PROGRESS_SEED, TWENTY_FOUR_HOURS,
        VAULT_SEED,
    },
    state::Policy,
};

use crate::utils::{
    damm_v2::{
        get_pool_with_config_pda, get_position_nft_account_pda, get_position_pda,
        get_token_vault_pda, set_damm_v2_position_fees,
    },
    find_program_address, find_program_event_authority, log_policy_account, log_progress_account,
    svm::{get_payer, TestContext, Token},
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
    let base_mint_acc = ctx.svm.get_account(&base_mint).unwrap();
    let quote_mint_acc = ctx.svm.get_account(&quote_mint).unwrap();

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

pub fn compute_crank_ix_accs<'a>(
    ctx: &'a TestContext,
    key: &str,
    pos_key: &str,
    payer: Pubkey,
) -> (&'a Token, (impl ToAccountMetas, Vec<AccountMeta>)) {
    let key = String::from(key);
    let token = ctx.tokens.get(&key).expect("");
    let base_mint = token.base_mint.pubkey();
    let quote_mint = token.quote_mint;
    let pos_mint = token.pos_mints.get(pos_key).unwrap();
    let (position_nft_account, _) = get_position_nft_account_pda(pos_mint.pubkey());
    let (pool, _) = get_pool_with_config_pda(token.pool_config, base_mint, quote_mint);
    let (position, _) = get_position_pda(pos_mint.pubkey());
    let pool_authority = damm_v2_constants::pool_authority::ID;

    let mut remaining_accounts = vec![];
    for i in token.investors.iter() {
        remaining_accounts.push(AccountMeta::new_readonly(i.stream.pubkey(), false));
        remaining_accounts.push(AccountMeta::new(
            get_associated_token_address(&i.key.pubkey(), &quote_mint),
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
    let (token, accounts) = compute_crank_ix_accs(&ctx, key, pos_key, payer.pubkey());
    let base_mint = token.base_mint.pubkey();
    let quote_mint = token.quote_mint;
    let pool_authority = damm_v2_constants::pool_authority::ID;

    ctx.send_transaction(
        &[
            create_associated_token_account(
                &payer.pubkey(),
                &pool_authority,
                &base_mint,
                &spl_token::ID,
            ),
            create_associated_token_account(
                &payer.pubkey(),
                &pool_authority,
                &quote_mint,
                &spl_token::ID,
            ),
            crank_ix(
                accounts.0,
                tollgate::instruction::Crank {
                    params: tollgate::instructions::CrankParams {
                        cursor: 0,
                        page_size: 2,
                    },
                },
                accounts.1,
            ),
        ],
        Some(&payer.pubkey()),
        &[payer],
    )
    .expect("");
}

#[test]
fn test_02_should_failed_base_fee_detected() {
    let mut ctx = TestContext::default();
    let key = "tollgate";
    let pos_key = "initialize";
    let payer = get_payer();
    let (_, accounts) = compute_crank_ix_accs(&ctx, key, pos_key, payer.pubkey());

    set_damm_v2_position_fees(&mut ctx, key, pos_key, Some(1), None);

    ctx.send_transaction(
        &[crank_ix(
            accounts.0,
            tollgate::instruction::Crank {
                params: tollgate::instructions::CrankParams {
                    cursor: 0,
                    page_size: 2,
                },
            },
            accounts.1,
        )],
        Some(&payer.pubkey()),
        &[payer],
    )
    .expect_err("");
}

#[test]
fn test_03_crank_claim_quote_fees() {
    let mut ctx = TestContext::default();
    let key = "tollgate";
    let pos_key = "initialize";
    let payer = get_payer();

    ctx.time_travel_by_secs(TWENTY_FOUR_HOURS as u64);
    set_damm_v2_position_fees(&mut ctx, key, pos_key, Some(0), Some(LAMPORTS_PER_SOL));
    let (_, accounts) = compute_crank_ix_accs(&ctx, key, pos_key, payer.pubkey());

    ctx.send_transaction(
        &[crank_ix(
            accounts.0,
            tollgate::instruction::Crank {
                params: tollgate::instructions::CrankParams {
                    cursor: 0,
                    page_size: 0,
                },
            },
            accounts.1,
        )],
        Some(&payer.pubkey()),
        &[payer],
    )
    .expect("");

    log_policy_account(&ctx, key);
    log_progress_account(&ctx, key);
}

#[test]
fn test_04_create_investors_ata() {
    let mut ctx = TestContext::default();
    let key = "tollgate";
    let payer = get_payer();
    let token = ctx.tokens.get(key).expect("");

    // TODO: let program init ATA when needed per policy (this will require additional investor pubkey)
    let mut create_ata_ixs = vec![];
    for i in token.investors.iter() {
        create_ata_ixs.push(create_associated_token_account(
            &payer.pubkey(),
            &i.key.pubkey(),
            &token.quote_mint,
            &spl_token::ID,
        ));
    }
    ctx.send_transaction(create_ata_ixs.as_slice(), Some(&payer.pubkey()), &[payer])
        .expect("");
}

#[test]
fn test_05_crank_page_0_to_3() {
    let mut ctx = TestContext::default();
    let key = "tollgate";
    let pos_key = "initialize";
    let payer = get_payer();

    set_damm_v2_position_fees(&mut ctx, key, pos_key, Some(0), Some(LAMPORTS_PER_SOL / 2));
    let (_, accounts) = compute_crank_ix_accs(&ctx, key, pos_key, payer.pubkey());

    ctx.send_transaction(
        &[crank_ix(
            accounts.0,
            tollgate::instruction::Crank {
                params: tollgate::instructions::CrankParams {
                    cursor: 0,
                    page_size: 3,
                },
            },
            accounts.1,
        )],
        Some(&payer.pubkey()),
        &[payer],
    )
    .expect("");

    log_progress_account(&ctx, key);
}

#[test]
fn test_06_crank_page_0_to_3_idempotent() {
    let mut ctx = TestContext::default();
    let key = "tollgate";
    let pos_key = "initialize";
    let payer = get_payer();

    let (_, accounts) = compute_crank_ix_accs(&ctx, key, pos_key, payer.pubkey());

    ctx.send_transaction(
        &[
            ComputeBudgetInstruction::set_compute_unit_price(1), // Use as a nonce
            crank_ix(
                accounts.0,
                tollgate::instruction::Crank {
                    params: tollgate::instructions::CrankParams {
                        cursor: 0,
                        page_size: 3,
                    },
                },
                accounts.1,
            ),
        ],
        Some(&payer.pubkey()),
        &[payer],
    )
    .expect("");

    log_progress_account(&ctx, key);
}

#[test]
fn test_07_crank_page_3_to_5() {
    let mut ctx = TestContext::default();
    let key = "tollgate";
    let pos_key = "initialize";
    let payer = get_payer();

    let (_, accounts) = compute_crank_ix_accs(&ctx, key, pos_key, payer.pubkey());

    ctx.send_transaction(
        &[crank_ix(
            accounts.0,
            tollgate::instruction::Crank {
                params: tollgate::instructions::CrankParams {
                    cursor: 3,
                    page_size: 2,
                },
            },
            accounts.1,
        )],
        Some(&payer.pubkey()),
        &[payer],
    )
    .expect("");

    log_progress_account(&ctx, key);
}

#[test]
fn test_08_crank_day_two_page_2_to_5_invalid_cursor() {
    let mut ctx = TestContext::default();
    let key = "tollgate";
    let pos_key = "initialize";
    let payer = get_payer();

    ctx.time_travel_by_secs(TWENTY_FOUR_HOURS as u64 + 1);
    let (_, accounts) = compute_crank_ix_accs(&ctx, key, pos_key, payer.pubkey());

    ctx.send_transaction(
        &[crank_ix(
            accounts.0,
            tollgate::instruction::Crank {
                params: tollgate::instructions::CrankParams {
                    cursor: 2,
                    page_size: 3,
                },
            },
            accounts.1,
        )],
        Some(&payer.pubkey()),
        &[payer],
    )
    .expect_err("");

    log_progress_account(&ctx, key);
}

#[test]
fn test_09_crank_day_two_page_0_to_4() {
    let mut ctx = TestContext::default();
    let key = "tollgate";
    let pos_key = "initialize";
    let payer = get_payer();

    let (_, accounts) = compute_crank_ix_accs(&ctx, key, pos_key, payer.pubkey());

    ctx.send_transaction(
        &[
            ComputeBudgetInstruction::set_compute_unit_limit(500_000),
            crank_ix(
                accounts.0,
                tollgate::instruction::Crank {
                    params: tollgate::instructions::CrankParams {
                        cursor: 0,
                        page_size: 4,
                    },
                },
                accounts.1,
            ),
        ],
        Some(&payer.pubkey()),
        &[payer],
    )
    .expect("");

    log_progress_account(&ctx, key);
}

#[test]
fn test_10_crank_day_two_full() {
    let mut ctx = TestContext::default();
    let key = "tollgate";
    let pos_key = "initialize";
    let payer = get_payer();

    let (token, accounts) = compute_crank_ix_accs(&ctx, key, pos_key, payer.pubkey());

    ctx.send_transaction(
        &[
            ComputeBudgetInstruction::set_compute_unit_limit(5_000_000),
            crank_ix(
                accounts.0,
                tollgate::instruction::Crank {
                    params: tollgate::instructions::CrankParams {
                        cursor: 4,
                        page_size: token.investors.len() as u32 - 4,
                    },
                },
                accounts.1,
            ),
        ],
        Some(&payer.pubkey()),
        &[payer],
    )
    .expect("");

    log_progress_account(&ctx, key);
}
