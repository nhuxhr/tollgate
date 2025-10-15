use anchor_client::{
    anchor_lang::{InstructionData, ToAccountMetas},
    solana_sdk::{instruction::Instruction, pubkey::Pubkey, signer::Signer, system_program},
};
use anchor_spl::token_2022;
use tollgate::{
    accounts::AccountInitialize,
    constants::{
        damm_v2_constants, INVESTOR_FEE_POS_OWNER, POLICY_SEED, PROGRESS_SEED, VAULT_SEED,
    },
};

use crate::utils::{
    damm_v2::{get_pool_with_config_pda, get_position_nft_account_pda, get_position_pda},
    find_program_address, find_program_event_authority,
    svm::{get_payer, TestContext},
};

#[allow(clippy::too_many_arguments)]
pub fn get_initialize_ix_accs(
    vault: Pubkey,
    pool: Pubkey,
    pool_cfg: Pubkey,
    position_nft_mint: Pubkey,
    position_nft_account: Pubkey,
    position: Pubkey,
    pool_authority: Pubkey,
    quote_mint: Pubkey,
    payer: Pubkey,
    event_authority: Pubkey,
) -> AccountInitialize {
    AccountInitialize {
        vault,
        policy: find_program_address(&[POLICY_SEED, vault.as_ref()], None).0,
        progress: find_program_address(&[PROGRESS_SEED, vault.as_ref()], None).0,
        pool,
        pool_cfg,
        position_nft_mint,
        position_nft_account,
        position,
        pool_authority,
        owner: find_program_address(&[VAULT_SEED, vault.as_ref(), INVESTOR_FEE_POS_OWNER], None).0,
        quote_mint,
        payer,
        event_authority,
        amm_program: damm_v2::ID,
        token_2022_program: token_2022::ID,
        system_program: system_program::ID,
    }
}

pub fn initialize_ix(
    accounts: impl ToAccountMetas,
    args: tollgate::instruction::Initialize,
) -> Instruction {
    Instruction::new_with_bytes(tollgate::ID, &args.data(), accounts.to_account_metas(None))
}

#[test]
fn test_01_should_failed_base_fee_detected() {
    let mut ctx = TestContext::default();
    let payer = get_payer();
    let key = String::from("coh");
    let token = ctx.tokens.get(&key).expect("Token not found in context");
    let base_mint = token.base_mint.pubkey();
    let quote_mint = token.quote_mint;
    let pos_mint = token
        .pos_mints
        .get("initialize")
        .expect("Position mint not found in context");
    let (position_nft_account, _) = get_position_nft_account_pda(pos_mint.pubkey());
    let (pool, _) = get_pool_with_config_pda(token.pool_config, base_mint, quote_mint);
    let (position, _) = get_position_pda(pos_mint.pubkey());

    ctx.send_transaction(
        &[initialize_ix(
            get_initialize_ix_accs(
                token.vault.pubkey(),
                pool,
                token.pool_config,
                pos_mint.pubkey(),
                position_nft_account,
                position,
                damm_v2_constants::pool_authority::ID,
                quote_mint,
                payer.pubkey(),
                find_program_event_authority(&damm_v2::ID).0,
            ),
            tollgate::instruction::Initialize {
                params: tollgate::instructions::InitializeParams {
                    investor_count: token.investors.len() as u32,
                    init_investor_ata: false,
                    investor_fee_share_bps: 10,
                    min_payout_lamports: 1,
                    daily_cap: None,
                    y0: 100,
                },
            },
        )],
        Some(&payer.pubkey()),
        &[
            &token.vault.insecure_clone(),
            &pos_mint.insecure_clone(),
            payer,
        ],
    )
    .expect_err("Transaction should fail due to base fee detection");
}

#[test]
fn test_02_initialize() {
    let mut ctx = TestContext::default();
    let payer = get_payer();
    let key = String::from("tollgate");
    let token = ctx.tokens.get(&key).expect("Token not found in context");
    let base_mint = token.base_mint.pubkey();
    let quote_mint = token.quote_mint;
    let pos_mint = token
        .pos_mints
        .get("initialize")
        .expect("Position mint not found in context");
    let (position_nft_account, _) = get_position_nft_account_pda(pos_mint.pubkey());
    let (pool, _) = get_pool_with_config_pda(token.pool_config, base_mint, quote_mint);
    let (position, _) = get_position_pda(pos_mint.pubkey());

    ctx.send_transaction(
        &[initialize_ix(
            get_initialize_ix_accs(
                token.vault.pubkey(),
                pool,
                token.pool_config,
                pos_mint.pubkey(),
                position_nft_account,
                position,
                damm_v2_constants::pool_authority::ID,
                quote_mint,
                payer.pubkey(),
                find_program_event_authority(&damm_v2::ID).0,
            ),
            tollgate::instruction::Initialize {
                params: tollgate::instructions::InitializeParams {
                    investor_count: token.investors.len() as u32,
                    init_investor_ata: true,
                    investor_fee_share_bps: 10,
                    min_payout_lamports: 1,
                    daily_cap: None,
                    y0: 100,
                },
            },
        )],
        Some(&payer.pubkey()),
        &[
            &token.vault.insecure_clone(),
            &pos_mint.insecure_clone(),
            payer,
        ],
    )
    .expect("Initialization should succeed");
}
