use anchor_client::{
    anchor_lang::{Discriminator, InstructionData, ToAccountMetas},
    solana_sdk::{
        account, instruction::Instruction, pubkey::Pubkey, signer::Signer, system_program,
    },
};
use anchor_spl::{associated_token::get_associated_token_address_with_program_id, token_2022};
use anyhow::{ensure, Result};
use ruint::aliases::U256;
use tollgate::constants::damm_v2_constants;

use crate::utils::{find_program_address, find_program_event_authority, svm::TestContext};

pub fn max_key(left: &Pubkey, right: &Pubkey) -> [u8; 32] {
    std::cmp::max(left, right).to_bytes()
}

pub fn min_key(left: &Pubkey, right: &Pubkey) -> [u8; 32] {
    std::cmp::min(left, right).to_bytes()
}

/// Calculates the address of the Position NFT account PDA.
pub fn get_position_nft_account_pda(position_nft_mint: Pubkey) -> (Pubkey, u8) {
    find_program_address(
        &[
            damm_v2_constants::seeds::POSITION_NFT_ACCOUNT_PREFIX,
            position_nft_mint.as_ref(),
        ],
        Some(&damm_v2::ID),
    )
}

/// Calculates the address of the Pool PDA with config.
pub fn get_pool_with_config_pda(
    config: Pubkey,
    base_mint: Pubkey,
    quote_mint: Pubkey,
) -> (Pubkey, u8) {
    find_program_address(
        &[
            damm_v2_constants::seeds::POOL_PREFIX,
            config.as_ref(),
            &max_key(&base_mint, &quote_mint),
            &min_key(&base_mint, &quote_mint),
        ],
        Some(&damm_v2::ID),
    )
}

/// Calculates the address of the Pool PDA.
pub fn get_pool_pda(base_mint: Pubkey, quote_mint: Pubkey) -> (Pubkey, u8) {
    find_program_address(
        &[
            damm_v2_constants::seeds::CUSTOMIZABLE_POOL_PREFIX,
            &max_key(&base_mint, &quote_mint),
            &min_key(&base_mint, &quote_mint),
        ],
        Some(&damm_v2::ID),
    )
}

/// Calculates the address of the Position PDA.
pub fn get_position_pda(position_nft_mint: Pubkey) -> (Pubkey, u8) {
    find_program_address(
        &[
            damm_v2_constants::seeds::POSITION_PREFIX,
            position_nft_mint.as_ref(),
        ],
        Some(&damm_v2::ID),
    )
}

/// Calculates the address of the Token Vault PDA.
pub fn get_token_vault_pda(token_mint: Pubkey, pool: Pubkey) -> (Pubkey, u8) {
    find_program_address(
        &[
            damm_v2_constants::seeds::TOKEN_VAULT_PREFIX,
            token_mint.as_ref(),
            pool.as_ref(),
        ],
        Some(&damm_v2::ID),
    )
}

pub fn set_damm_v2_position_fees(
    ctx: &mut TestContext,
    key: &str,
    pos_key: &str,
    base_fee: Option<u64>,
    quote_fee: Option<u64>,
) {
    let token = ctx.tokens.get(key).expect("");
    let base_mint = token.base_mint.pubkey();
    let quote_mint = token.quote_mint;
    let (pool, _) = get_pool_with_config_pda(token.pool_config, base_mint, quote_mint);
    let pos_mint = token.pos_mints.get(pos_key).unwrap();
    let (position, _) = get_position_pda(pos_mint.pubkey());

    let pool_acc = ctx.svm.get_account(&pool).unwrap();
    let pos_acc = ctx.svm.get_account(&position).unwrap();

    let mut pos_data_slice = [0; 400];
    let pool_pacc: damm_v2::accounts::Pool =
        bytemuck::try_pod_read_unaligned(pool_acc.data.as_slice().split_at(8).1).expect("");
    let pos_pacc: damm_v2::accounts::Position =
        bytemuck::try_pod_read_unaligned(pos_acc.data.as_slice().split_at(8).1).expect("");
    let pos_state: &mut damm_v2::accounts::Position =
        bytemuck::try_from_bytes_mut(&mut pos_data_slice).expect("");

    let quote_token_order = tollgate::utils::token::get_token_order(&pool_pacc, &quote_mint);
    let (final_base_fee, final_quote_fee) = match quote_token_order.unwrap() {
        tollgate::utils::token::TokenOrder::A => (
            base_fee.unwrap_or(pos_pacc.fee_b_pending),
            quote_fee.unwrap_or(pos_pacc.fee_a_pending),
        ),
        tollgate::utils::token::TokenOrder::B => (
            base_fee.unwrap_or(pos_pacc.fee_a_pending),
            quote_fee.unwrap_or(pos_pacc.fee_b_pending),
        ),
    };

    let (fee_a_pending, fee_b_pending) = match quote_token_order.unwrap() {
        tollgate::utils::token::TokenOrder::A => (final_quote_fee, final_base_fee),
        tollgate::utils::token::TokenOrder::B => (final_base_fee, final_quote_fee),
    };

    pos_state.pool = pos_pacc.pool;
    pos_state.nft_mint = pos_pacc.nft_mint;
    pos_state.fee_a_per_token_checkpoint = pos_pacc.fee_a_per_token_checkpoint;
    pos_state.fee_b_per_token_checkpoint = pos_pacc.fee_b_per_token_checkpoint;
    pos_state.fee_a_pending = fee_a_pending;
    pos_state.fee_b_pending = fee_b_pending;
    pos_state.unlocked_liquidity = pos_pacc.unlocked_liquidity;
    pos_state.vested_liquidity = pos_pacc.vested_liquidity;
    pos_state.permanent_locked_liquidity = pos_pacc.permanent_locked_liquidity;
    pos_state.metrics = pos_pacc.metrics;
    pos_state.reward_infos = pos_pacc.reward_infos;
    pos_state.padding = pos_pacc.padding;

    let mut updated_pos_data = Vec::with_capacity(408);
    updated_pos_data.extend_from_slice(damm_v2::accounts::Position::DISCRIMINATOR);
    updated_pos_data.extend_from_slice(&pos_data_slice);

    ctx.svm
        .set_account(
            position,
            account::Account {
                lamports: pos_acc.lamports,
                data: updated_pos_data,
                owner: pos_acc.owner,
                executable: pos_acc.executable,
                rent_epoch: pos_acc.rent_epoch,
            },
        )
        .unwrap();
}

#[allow(clippy::too_many_arguments)]
pub fn get_initialize_pool_ix_accs(
    ctx: &TestContext,
    creator: Pubkey,
    position_nft_mint: Pubkey,
    // position_nft_account: Pubkey,
    payer: Pubkey,
    config: Pubkey,
    pool_authority: Pubkey,
    // pool: Pubkey,
    // position: Pubkey,
    base_mint: Pubkey,
    quote_mint: Pubkey,
    // base_vault: Pubkey,
    // quote_vault: Pubkey,
    // payer_base_account: Pubkey,
    // payer_quote_account: Pubkey,
    // base_program: Pubkey,
    // quote_program: Pubkey,
) -> damm_v2::client::accounts::InitializePool {
    let (position_nft_account, _) = get_position_nft_account_pda(position_nft_mint);
    let (pool, _) = get_pool_with_config_pda(config, base_mint, quote_mint);
    let (position, _) = get_position_pda(position_nft_mint);
    let (base_vault, _) = get_token_vault_pda(base_mint, pool);
    let (quote_vault, _) = get_token_vault_pda(quote_mint, pool);

    let base_mint_acc = ctx.svm.get_account(&base_mint).expect("");
    let quote_mint_acc = ctx.svm.get_account(&quote_mint).expect("");

    let payer_base_account =
        get_associated_token_address_with_program_id(&creator, &base_mint, &base_mint_acc.owner);
    let payer_quote_account =
        get_associated_token_address_with_program_id(&creator, &quote_mint, &quote_mint_acc.owner);

    damm_v2::client::accounts::InitializePool {
        creator,
        position_nft_mint,
        position_nft_account,
        payer,
        config,
        pool_authority,
        pool,
        position,
        token_a_mint: base_mint,
        token_b_mint: quote_mint,
        token_a_vault: base_vault,
        token_b_vault: quote_vault,
        payer_token_a: payer_base_account,
        payer_token_b: payer_quote_account,
        token_a_program: base_mint_acc.owner,
        token_b_program: quote_mint_acc.owner,
        token_2022_program: token_2022::ID,
        system_program: system_program::ID,
        event_authority: find_program_event_authority(&damm_v2::ID).0,
        program: damm_v2::ID,
    }
}

pub fn initialize_pool_ix(
    accounts: impl ToAccountMetas,
    args: damm_v2::client::args::InitializePool,
) -> Instruction {
    Instruction::new_with_bytes(damm_v2::ID, &args.data(), accounts.to_account_metas(None))
}

#[allow(clippy::too_many_arguments)]
pub fn get_initialize_customizable_pool_ix_accs(
    ctx: &TestContext,
    creator: Pubkey,
    position_nft_mint: Pubkey,
    // position_nft_account: Pubkey,
    payer: Pubkey,
    pool_authority: Pubkey,
    // pool: Pubkey,
    // position: Pubkey,
    base_mint: Pubkey,
    quote_mint: Pubkey,
    // base_vault: Pubkey,
    // quote_vault: Pubkey,
    // payer_base_account: Pubkey,
    // payer_quote_account: Pubkey,
    // base_program: Pubkey,
    // quote_program: Pubkey,
) -> damm_v2::client::accounts::InitializeCustomizablePool {
    let (position_nft_account, _) = get_position_nft_account_pda(position_nft_mint);
    let (pool, _) = get_pool_pda(base_mint, quote_mint);
    let (position, _) = get_position_pda(position_nft_mint);
    let (base_vault, _) = get_token_vault_pda(base_mint, pool);
    let (quote_vault, _) = get_token_vault_pda(quote_mint, pool);

    let base_mint_acc = ctx.svm.get_account(&base_mint).expect("");
    let quote_mint_acc = ctx.svm.get_account(&quote_mint).expect("");

    let payer_base_account =
        get_associated_token_address_with_program_id(&creator, &base_mint, &base_mint_acc.owner);
    let payer_quote_account =
        get_associated_token_address_with_program_id(&creator, &quote_mint, &quote_mint_acc.owner);

    damm_v2::client::accounts::InitializeCustomizablePool {
        creator,
        position_nft_mint,
        position_nft_account,
        payer,
        pool_authority,
        pool,
        position,
        token_a_mint: base_mint,
        token_b_mint: quote_mint,
        token_a_vault: base_vault,
        token_b_vault: quote_vault,
        payer_token_a: payer_base_account,
        payer_token_b: payer_quote_account,
        token_a_program: base_mint_acc.owner,
        token_b_program: quote_mint_acc.owner,
        token_2022_program: token_2022::ID,
        system_program: system_program::ID,
        event_authority: find_program_event_authority(&damm_v2::ID).0,
        program: damm_v2::ID,
    }
}

pub fn initialize_customizable_pool_ix(
    accounts: impl ToAccountMetas,
    args: damm_v2::client::args::InitializeCustomizablePool,
) -> Instruction {
    Instruction::new_with_bytes(damm_v2::ID, &args.data(), accounts.to_account_metas(None))
}

#[allow(clippy::too_many_arguments)]
pub fn get_initialize_pool_with_dynamic_config_ix_accs(
    ctx: &TestContext,
    creator: Pubkey,
    position_nft_mint: Pubkey,
    // position_nft_account: Pubkey,
    payer: Pubkey,
    pool_creator_authority: Pubkey,
    config: Pubkey,
    pool_authority: Pubkey,
    // pool: Pubkey,
    // position: Pubkey,
    base_mint: Pubkey,
    quote_mint: Pubkey,
    // base_vault: Pubkey,
    // quote_vault: Pubkey,
    // payer_base_account: Pubkey,
    // payer_quote_account: Pubkey,
    // base_program: Pubkey,
    // quote_program: Pubkey,
) -> damm_v2::client::accounts::InitializePoolWithDynamicConfig {
    let (position_nft_account, _) = get_position_nft_account_pda(position_nft_mint);
    let (pool, _) = get_pool_with_config_pda(config, base_mint, quote_mint);
    let (position, _) = get_position_pda(position_nft_mint);
    let (base_vault, _) = get_token_vault_pda(base_mint, pool);
    let (quote_vault, _) = get_token_vault_pda(quote_mint, pool);

    let base_mint_acc = ctx.svm.get_account(&base_mint).expect("");
    let quote_mint_acc = ctx.svm.get_account(&quote_mint).expect("");

    let payer_base_account =
        get_associated_token_address_with_program_id(&creator, &base_mint, &base_mint_acc.owner);
    let payer_quote_account =
        get_associated_token_address_with_program_id(&creator, &quote_mint, &quote_mint_acc.owner);

    damm_v2::client::accounts::InitializePoolWithDynamicConfig {
        creator,
        position_nft_mint,
        position_nft_account,
        payer,
        pool_creator_authority,
        config,
        pool_authority,
        pool,
        position,
        token_a_mint: base_mint,
        token_b_mint: quote_mint,
        token_a_vault: base_vault,
        token_b_vault: quote_vault,
        payer_token_a: payer_base_account,
        payer_token_b: payer_quote_account,
        token_a_program: base_mint_acc.owner,
        token_b_program: quote_mint_acc.owner,
        token_2022_program: token_2022::ID,
        system_program: system_program::ID,
        event_authority: find_program_event_authority(&damm_v2::ID).0,
        program: damm_v2::ID,
    }
}

pub fn initialize_pool_with_dynamic_config_ix(
    accounts: impl ToAccountMetas,
    args: damm_v2::client::args::InitializeCustomizablePool,
) -> Instruction {
    Instruction::new_with_bytes(damm_v2::ID, &args.data(), accounts.to_account_metas(None))
}

/// Helpers

#[derive(Debug)]
pub struct PreparedPoolCreation {
    pub init_sqrt_price: u128,
    pub liquidity_delta: u128,
}

pub fn sqrt_u256(radicand: U256) -> Option<U256> {
    if radicand == U256::ZERO {
        return Some(U256::ZERO);
    }
    // Compute bit, the largest power of 4 <= n
    let max_shift = U256::ZERO.leading_zeros() - 1;
    let shift = (max_shift - radicand.leading_zeros()) & !1;
    let mut bit = U256::ONE.checked_shl(shift)?;

    let mut n = radicand;
    let mut result = U256::ZERO;
    while bit != U256::ZERO {
        let result_with_bit = result.checked_add(bit)?;
        if n >= result_with_bit {
            n = n.checked_sub(result_with_bit)?;
            result = result.checked_shr(1)?.checked_add(bit)?;
        } else {
            result = result.checked_shr(1)?;
        }
        (bit, _) = bit.overflowing_shr(2);
    }
    Some(result)
}

// a = L * (1/s - 1/pb)
// b = L * (s - pa)
// b/a = (s - pa) / (1/s - 1/pb)
// With: x = 1 / pb and y = b/a
// => s ^ 2 + s * (-pa + x * y) - y = 0
// s = [(pa - xy) + √((xy - pa)² + 4y)]/2, // pa: min_sqrt_price, pb: max_sqrt_price
// s = [(pa - b << 128 / a / pb) + sqrt((b << 128 / a / pb - pa)² + 4 * b << 128 / a)] / 2
pub fn calculate_init_sqrt_price(
    token_a_amount: u64,
    token_b_amount: u64,
    min_sqrt_price: u128,
    max_sqrt_price: u128,
) -> Result<u128> {
    ensure!(
        token_a_amount != 0 && token_b_amount != 0,
        "Amount cannot be zero"
    );

    let a = U256::from(token_a_amount);
    let b = U256::from(token_b_amount) << 128;
    let pa = U256::from(min_sqrt_price);
    let pb = U256::from(max_sqrt_price);

    let four = U256::from(4u8);
    let two = U256::from(2u8);

    let y = b / a;
    let xy = y / pb;

    let s = if y > pa * pb {
        let delta = xy - pa;
        let discriminant = delta * delta + four * y;
        let sqrt_value =
            sqrt_u256(discriminant).ok_or_else(|| anyhow::anyhow!("Sqrt calculation failed"))?;
        (sqrt_value - delta) / two
    } else {
        let delta = pa - xy;
        let discriminant = delta * delta + four * y;
        let sqrt_value =
            sqrt_u256(discriminant).ok_or_else(|| anyhow::anyhow!("Sqrt calculation failed"))?;
        (sqrt_value + delta) / two
    };

    u128::try_from(s).map_err(|_| anyhow::anyhow!("Type cast failed"))
}

pub fn get_liquidity_delta_from_amount_a(
    amount_a: u64,
    lower_sqrt_price: u128,
    upper_sqrt_price: u128,
) -> Result<u128> {
    ensure!(
        upper_sqrt_price > lower_sqrt_price,
        "Upper price must be greater than lower price"
    );

    let a = U256::from(amount_a);
    let lower = U256::from(lower_sqrt_price);
    let upper = U256::from(upper_sqrt_price);
    let denom = upper - lower;

    let product = a * lower * upper;
    let liquidity = product / denom;

    u128::try_from(liquidity).map_err(|_| anyhow::anyhow!("Type cast failed"))
}

pub fn get_liquidity_delta_from_amount_b(
    amount_b: u64,
    lower_sqrt_price: u128,
    upper_sqrt_price: u128,
) -> Result<u128> {
    ensure!(
        upper_sqrt_price > lower_sqrt_price,
        "Upper price must be greater than lower price"
    );

    let b = U256::from(amount_b);
    let denom = U256::from(upper_sqrt_price - lower_sqrt_price);

    let product = b << 128;
    let liquidity = product / denom;

    u128::try_from(liquidity).map_err(|_| anyhow::anyhow!("Type cast failed"))
}

pub fn prepare_pool_creation_params(
    token_a_amount: u64,
    token_b_amount: u64,
    min_sqrt_price: u128,
    max_sqrt_price: u128,
) -> Result<PreparedPoolCreation> {
    ensure!(
        !(token_a_amount == 0 && token_b_amount == 0),
        "Invalid input amount"
    );

    let actual_amount_a_in = token_a_amount;
    let actual_amount_b_in = token_b_amount;

    let init_sqrt_price = calculate_init_sqrt_price(
        actual_amount_a_in,
        actual_amount_b_in,
        min_sqrt_price,
        max_sqrt_price,
    )?;

    let liquidity_delta_from_amount_a =
        get_liquidity_delta_from_amount_a(actual_amount_a_in, init_sqrt_price, max_sqrt_price)?;

    let liquidity_delta_from_amount_b =
        get_liquidity_delta_from_amount_b(actual_amount_b_in, min_sqrt_price, init_sqrt_price)?;

    let liquidity_delta =
        std::cmp::min(liquidity_delta_from_amount_a, liquidity_delta_from_amount_b);

    Ok(PreparedPoolCreation {
        init_sqrt_price,
        liquidity_delta,
    })
}
