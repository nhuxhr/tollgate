use anchor_lang::prelude::Pubkey;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenOrder {
    A,
    B,
}

pub fn get_token_order(pool: &damm_v2::accounts::Pool, mint: &Pubkey) -> Option<TokenOrder> {
    if *mint == pool.token_a_mint {
        Some(TokenOrder::A)
    } else if *mint == pool.token_b_mint {
        Some(TokenOrder::B)
    } else {
        None
    }
}
