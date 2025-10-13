use std::{collections::HashMap, time::SystemTime};

use anchor_client::solana_sdk::{
    native_token::LAMPORTS_PER_SOL, pubkey::Pubkey, signature::Keypair, signer::Signer,
    system_instruction,
};
use anchor_spl::{
    associated_token::{
        get_associated_token_address,
        spl_associated_token_account::instruction::create_associated_token_account_idempotent,
    },
    token::spl_token::{self, native_mint},
};
use solana_pubkey::pubkey;
use tollgate::constants::damm_v2_constants;

use crate::{
    constants::{MAX_SQRT_PRICE, MIN_SQRT_PRICE, SOL_MINT},
    utils::{
        damm_v2::{get_initialize_pool_ix_accs, initialize_pool_ix, prepare_pool_creation_params},
        rand_investors_num,
        streamflow::{create_stream_ix, get_create_stream_ix_accs},
        svm::{Investor, TestContext, Token},
    },
};

pub fn ensure_token(
    ctx: &mut TestContext,
    key: String,
    amount: u64,
    pool_config: Pubkey,
    quote_mint: Pubkey,
) -> (Keypair, Keypair, Pubkey) {
    if let Some(token) = ctx.tokens.get(&key) {
        // Get the associated token address for the creator
        let ata = get_associated_token_address(&token.creator.pubkey(), &token.base_mint.pubkey());

        let creator_clone = token.creator.insecure_clone();
        let mint_clone = token.base_mint.insecure_clone();
        (creator_clone, mint_clone, ata)
    } else {
        let creator = Keypair::new();
        let base_mint = Keypair::new();

        ctx.airdrop(&creator.pubkey(), 100).expect("");
        let base_mint = ctx.create_spl_token(Some(&creator), Some(base_mint), amount);

        // Get the associated token address for the creator
        let ata = get_associated_token_address(&creator.pubkey(), &base_mint.pubkey());

        let creator_clone = creator.insecure_clone();
        let mint_clone = base_mint.insecure_clone();
        ctx.tokens.insert(
            key,
            Token {
                creator,
                base_mint,
                quote_mint,
                pool_config,
                pos_mints: HashMap::from([
                    ("initial".to_string(), Keypair::new()),
                    ("initialize".to_string(), Keypair::new()),
                ]),
                vault: Keypair::new(),
                investors: vec![],
            },
        );

        (creator_clone, mint_clone, ata)
    }
}

#[test]
fn test_01_create_tollgate_token() {
    let mut ctx = TestContext::default();
    let key = String::from("tollgate");
    let amount = 1000 * LAMPORTS_PER_SOL;
    let quote_mint = SOL_MINT;
    let pool_config = pubkey!("EQbqYxecZuJsVt6g5QbKTWpNWa3QyWQE5NWz5AZBAiNv");
    let (creator, base_mint, creator_ata) =
        ensure_token(&mut ctx, key.clone(), amount, pool_config, quote_mint);
    println!("[Tollgate]::Mint: {}", base_mint.pubkey());
    println!("[Tollgate]::Creator: {}", creator.pubkey());
    println!("[Tollgate]::Creator ATA: {}", creator_ata);

    let token = ctx.tokens.get(&key).expect("");
    let position_nft_mint = token.pos_mints.get("initial").unwrap().insecure_clone();
    let pool_params = prepare_pool_creation_params(
        10 * LAMPORTS_PER_SOL,
        10 * LAMPORTS_PER_SOL,
        MIN_SQRT_PRICE,
        MAX_SQRT_PRICE,
    )
    .expect("");
    ctx.send_transaction(
        &[
            create_associated_token_account_idempotent(
                &creator.pubkey(),
                &creator.pubkey(),
                &native_mint::id(),
                &spl_token::ID,
            ),
            system_instruction::transfer(
                &creator.pubkey(),
                &get_associated_token_address(&creator.pubkey(), &native_mint::ID),
                10 * LAMPORTS_PER_SOL,
            ),
            spl_token::instruction::sync_native(
                &spl_token::ID,
                &get_associated_token_address(&creator.pubkey(), &native_mint::ID),
            )
            .expect(""),
            initialize_pool_ix(
                get_initialize_pool_ix_accs(
                    &ctx,
                    creator.pubkey(),
                    position_nft_mint.pubkey(),
                    creator.pubkey(),
                    pool_config,
                    damm_v2_constants::pool_authority::ID,
                    base_mint.pubkey(),
                    quote_mint,
                ),
                damm_v2::client::args::InitializePool {
                    params: damm_v2::types::InitializePoolParameters {
                        liquidity: pool_params.liquidity_delta,
                        sqrt_price: pool_params.init_sqrt_price,
                        activation_point: None,
                    },
                },
            ),
        ],
        Some(&creator.pubkey()),
        &[&creator, &position_nft_mint],
    )
    .expect("");

    let investors_rand = rand_investors_num(8..12);
    let mut investors = vec![];
    for _ in 1..=investors_rand {
        let mut signers = vec![];
        let investor = Investor {
            key: Keypair::new(),
            stream: Keypair::new(),
        };
        signers.push(creator.insecure_clone());
        signers.push(investor.stream.insecure_clone());

        let current_timestamp = SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let start_time = current_timestamp + rand::random::<u64>() % (7 * 24 * 60 * 60);

        let net_amount_deposited = rand::random::<u64>() % (10_000_000 - 5_000_000 + 1) + 5_000_000;

        let periods = rand::random::<u64>() % (8 - 4 + 1) + 4;
        let amount_per_period = net_amount_deposited / periods;

        let period = 3600; // 1 hour in seconds
        let cliff = 0;
        let cliff_amount = 0;
        let cancelable_by_sender = true;
        let cancelable_by_recipient = false;
        let automatic_withdrawal = true;
        let transferable_by_sender = false;
        let transferable_by_recipient = false;
        let can_topup = true;
        let stream_name = [1; 64];
        let withdraw_frequency = 3600;
        let pausable = Some(true);
        let can_update_rate = Some(true);

        let create_stream_ix = create_stream_ix(
            get_create_stream_ix_accs(
                creator.pubkey(),
                investor.key.pubkey(),
                investor.stream.pubkey(),
                base_mint.pubkey(),
            ),
            streamflow_sdk::instruction::Create {
                start_time,
                net_amount_deposited,
                period,
                amount_per_period,
                cliff,
                cliff_amount,
                cancelable_by_sender,
                cancelable_by_recipient,
                automatic_withdrawal,
                transferable_by_sender,
                transferable_by_recipient,
                can_topup,
                stream_name,
                withdraw_frequency,
                pausable,
                can_update_rate,
            },
        );

        signers.push(creator.insecure_clone());
        let signers_iter: Vec<&Keypair> = signers.iter().collect();
        ctx.send_transaction(&[create_stream_ix], Some(&creator.pubkey()), &signers_iter)
            .expect("Failed to create streams");

        investors.push(investor);
    }

    ctx.tokens.get_mut(&key).expect("").investors = investors;
}

#[test]
fn test_02_create_coh_token() {
    let mut ctx = TestContext::default();
    let key = String::from("coh");
    let amount = 10 * LAMPORTS_PER_SOL;
    let quote_mint = SOL_MINT;
    let pool_config = pubkey!("FzvMYBQ29z2J21QPsABpJYYxQBEKGsxA6w6J2HYceFj8");
    let (creator, base_mint, creator_ata) =
        ensure_token(&mut ctx, key.clone(), amount, pool_config, quote_mint);
    println!("[Cat On Horse]::Mint: {}", base_mint.pubkey());
    println!("[Cat On Horse]::Creator: {}", creator.pubkey());
    println!("[Cat On Horse]::Creator ATA: {}", creator_ata);

    let token = ctx.tokens.get(&key).expect("");
    let position_nft_mint = token.pos_mints.get("initial").unwrap().insecure_clone();
    let pool_params = prepare_pool_creation_params(
        4 * LAMPORTS_PER_SOL,
        10 * LAMPORTS_PER_SOL,
        MIN_SQRT_PRICE,
        MAX_SQRT_PRICE,
    )
    .expect("");
    ctx.send_transaction(
        &[
            create_associated_token_account_idempotent(
                &creator.pubkey(),
                &creator.pubkey(),
                &native_mint::id(),
                &spl_token::ID,
            ),
            system_instruction::transfer(
                &creator.pubkey(),
                &get_associated_token_address(&creator.pubkey(), &native_mint::ID),
                10 * LAMPORTS_PER_SOL,
            ),
            spl_token::instruction::sync_native(
                &spl_token::ID,
                &get_associated_token_address(&creator.pubkey(), &native_mint::ID),
            )
            .expect(""),
            initialize_pool_ix(
                get_initialize_pool_ix_accs(
                    &ctx,
                    creator.pubkey(),
                    position_nft_mint.pubkey(),
                    creator.pubkey(),
                    pool_config,
                    damm_v2_constants::pool_authority::ID,
                    base_mint.pubkey(),
                    quote_mint,
                ),
                damm_v2::client::args::InitializePool {
                    params: damm_v2::types::InitializePoolParameters {
                        liquidity: pool_params.liquidity_delta,
                        sqrt_price: pool_params.init_sqrt_price,
                        activation_point: None,
                    },
                },
            ),
        ],
        Some(&creator.pubkey()),
        &[&creator, &position_nft_mint],
    )
    .expect("");
}
