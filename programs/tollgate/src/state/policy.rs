use anchor_lang::prelude::*;

use crate::{constants::MAX_BPS, error::TollgateError, instructions::InitializeParams};

#[account]
#[derive(Debug, InitSpace)]
pub struct Policy {
    pub vault: Pubkey,               // Associated vault
    pub creator: Pubkey,             // Creator pubkey that receives remainder
    pub quote_mint: Pubkey,          // Quote mint of the associated pool
    pub investor_count: u32,         // Investor count
    pub init_investor_ata: bool,     // Initialize investor ATA if needed
    pub investor_fee_share_bps: u16, // e.g., 7000 for 70%
    pub min_payout_lamports: u64,    // Dust threshold
    pub daily_cap: Option<u64>,      // Optional total daily distributable
    pub y0: u64,                     // Total investor allocation at TGE
    pub is_initialized: bool,        // Whether initialized
    pub owner_bump: u8,              // Position owner bump
    pub bump: u8,                    // PDA bump
}

impl Policy {
    pub const SPACE: usize = Self::DISCRIMINATOR.len() + Self::INIT_SPACE;

    /// Initializes the Policy account.
    pub fn initialize(
        &mut self,
        vault: Pubkey,
        creator: Pubkey,
        quote_mint: Pubkey,
        params: InitializeParams,
        owner_bump: u8,
        bump: u8,
    ) -> Result<()> {
        // assert policy is not already initialized
        require!(
            !self.is_initialized,
            TollgateError::PolicyAlreadyInitialized
        );

        // assert inveestor fee share bps is less than or equal to 100%
        require_gte!(
            MAX_BPS,
            params.investor_fee_share_bps,
            TollgateError::InvalidInvestorFeeShareBps
        );

        // assert min payout lamports is greater than 0
        require_gt!(
            params.min_payout_lamports,
            0,
            TollgateError::InvalidMinPayoutLamports
        );

        // assert daily cap is either None or greater than 0
        if let Some(daily_cap) = params.daily_cap {
            require_gt!(daily_cap, 0, TollgateError::InvalidDailyCap);
        }

        // assert y0 is greater than 0
        require_gt!(params.y0, 0, TollgateError::InvalidY0Allocation);

        self.vault = vault;
        self.creator = creator;
        self.quote_mint = quote_mint;
        self.investor_count = params.investor_count;
        self.init_investor_ata = params.init_investor_ata;
        self.investor_fee_share_bps = params.investor_fee_share_bps;
        self.min_payout_lamports = params.min_payout_lamports;
        self.daily_cap = params.daily_cap;
        self.y0 = params.y0;
        self.is_initialized = true;
        self.owner_bump = owner_bump;
        self.bump = bump;

        Ok(())
    }
}
