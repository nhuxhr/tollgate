use anchor_lang::prelude::*;

#[event]
pub struct HonoraryPositionInitialized {
    pub vault: Pubkey,
    pub policy: Pubkey,
    pub progress: Pubkey,
    pub pool: Pubkey,
    pub pool_cfg: Pubkey,
    pub position: Pubkey,
    pub owner: Pubkey,
    pub base_mint: Pubkey,
    pub quote_mint: Pubkey,
    pub investor_fee_share_bps: u16,
    pub min_payout_lamports: u64,
    pub daily_cap: Option<u64>,
    pub y0: u64,
}

#[event]
pub struct QuoteFeesClaimed {
    pub vault: Pubkey,
    pub policy: Pubkey,
    pub progress: Pubkey,
    pub pool: Pubkey,
    pub position: Pubkey,
    pub owner: Pubkey,
    pub base_fee_claimed: u64,
    pub quote_fee_claimed: u64,
}

#[event]
pub struct InvestorPayoutPage {
    pub vault: Pubkey,
    pub policy: Pubkey,
    pub progress: Pubkey,
    pub pool: Pubkey,
    pub position: Pubkey,
    pub owner: Pubkey,
    pub cursor: u32,
    pub investors: u32,
    pub page_start: u32,
    pub page_end: u32,
    pub payout: u64,
}

#[event]
pub struct CreatorPayoutDayClosed {
    pub vault: Pubkey,
    pub policy: Pubkey,
    pub progress: Pubkey,
    pub pool: Pubkey,
    pub position: Pubkey,
    pub owner: Pubkey,
    pub timestamp: i64,
    pub total_distributed: u64,
    pub creator_payout: u64,
    pub carry: u64,
}
