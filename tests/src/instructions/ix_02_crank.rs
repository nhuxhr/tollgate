use anchor_client::{
    anchor_lang::{InstructionData, ToAccountMetas},
    solana_sdk::{instruction::Instruction, pubkey::Pubkey, signer::Signer, system_program},
};
use anchor_spl::{
    associated_token::{
        get_associated_token_address,
        spl_associated_token_account::{self, instruction::create_associated_token_account},
    },
    token::spl_token,
};
use tollgate::{
    accounts::AccountCrank,
    constants::{
        damm_v2_constants, INVESTOR_FEE_POS_OWNER, POLICY_SEED, PROGRESS_SEED, VAULT_SEED,
    },
    state::Policy,
};

use crate::utils::{
    damm_v2::{get_pool_with_config_pda, get_position_nft_account_pda, get_position_pda},
    find_program_address, find_program_event_authority,
    svm::{get_payer, TestContext},
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

    // TODO: remove log
    println!(
        "accounts::\n- {}\n- {}\n- {}\n- {}\n- {}\n- {}\n- {}\n- {}\n- {}\n- {}\n- {}\n- {}\n- {}\n- {}\n- {}\n- {}\n- {}\n- {}\n- {}\n- {}\n- {}\n- {}\n",
        policy,
        find_program_address(&[PROGRESS_SEED, vault.as_ref()], None).0,
        pool,
        position_nft_account,
        position,
        pool_authority,
        owner,
        get_associated_token_address(&owner, &quote_mint),
        get_associated_token_address(&owner, &base_mint),
        get_associated_token_address(&owner, &quote_mint),
        get_associated_token_address(&pool_authority, &base_mint),
        get_associated_token_address(&pool_authority, &quote_mint),
        base_mint,
        quote_mint,
        base_mint_acc.owner,
        quote_mint_acc.owner,
        get_associated_token_address(&policy_program_acc.creator, &quote_mint),
        payer,
        event_authority,
        damm_v2::ID,
        spl_associated_token_account::ID,
        system_program::ID,
    );

    AccountCrank {
        policy,
        progress: find_program_address(&[PROGRESS_SEED, vault.as_ref()], None).0,
        pool,
        position_nft_account,
        position,
        pool_authority,
        owner,
        treasury: get_associated_token_address(&owner, &quote_mint),
        base_account: get_associated_token_address(&owner, &base_mint),
        quote_account: get_associated_token_address(&owner, &quote_mint),
        base_vault: get_associated_token_address(&pool_authority, &base_mint),
        quote_vault: get_associated_token_address(&pool_authority, &quote_mint),
        base_mint,
        quote_mint,
        base_program: base_mint_acc.owner,
        quote_program: quote_mint_acc.owner,
        creator_accoount: get_associated_token_address(&policy_program_acc.creator, &quote_mint),
        payer,
        event_authority,
        amm_program: damm_v2::ID,
        associated_token_program: spl_associated_token_account::ID,
        system_program: system_program::ID,
    }
}

pub fn crank_ix(accounts: impl ToAccountMetas, args: tollgate::instruction::Crank) -> Instruction {
    Instruction::new_with_bytes(tollgate::ID, &args.data(), accounts.to_account_metas(None))
}

#[test]
fn test_01_crank() {
    let mut ctx = TestContext::default();
    let payer = get_payer();
    let key = String::from("tollgate");
    let token = ctx.tokens.get(&key).expect("");
    let base_mint = token.base_mint.pubkey();
    let quote_mint = token.quote_mint;
    let pos_mint = token.pos_mints.get("initialize").unwrap();
    let (position_nft_account, _) = get_position_nft_account_pda(pos_mint.pubkey());
    let (pool, _) = get_pool_with_config_pda(token.pool_config, base_mint, quote_mint);
    let (position, _) = get_position_pda(pos_mint.pubkey());
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
                get_crank_ix_accs(
                    &ctx,
                    token.vault.pubkey(),
                    pool,
                    position_nft_account,
                    position,
                    pool_authority,
                    base_mint,
                    quote_mint,
                    payer.pubkey(),
                    find_program_event_authority(&damm_v2::ID).0,
                ),
                tollgate::instruction::Crank {
                    params: tollgate::instructions::CrankParams {
                        cursor: 0,
                        page_size: 2,
                    },
                },
            ),
        ],
        Some(&payer.pubkey()),
        &[payer],
    )
    .expect("");
}
