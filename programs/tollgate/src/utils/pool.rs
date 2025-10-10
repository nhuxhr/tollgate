use std::cell::Ref;

pub fn is_valid_pool(
    pool: &Option<Ref<'_, damm_v2::accounts::Pool>>,
    // base_mint: Pubkey,
    // quote_mint: Pubkey,
) -> bool {
    // Check pool is initialized
    if pool.is_none() {
        return false;
    }
    // Unwrap pool
    let pool = pool.as_ref().unwrap();

    // // Check base token order is valid
    // if utils::token::get_token_order(pool, &base_mint).is_none() {
    //     return false;
    // }

    // // Check quote token order is valid
    // if utils::token::get_token_order(pool, &quote_mint).is_none() {
    //     return false;
    // }

    // Ensure min < max price
    if pool.sqrt_min_price == 0 || pool.sqrt_max_price <= pool.sqrt_min_price {
        return false;
    }
    // Check pool is enabled
    if pool.pool_status != 0 {
        return false;
    }
    true
}

pub fn is_valid_pool_cfg(pool_cfg: &Option<Ref<'_, damm_v2::accounts::Config>>) -> bool {
    // Check pool config is initialized
    if pool_cfg.is_none() {
        return false;
    }
    true
}
