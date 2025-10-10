use anchor_lang::prelude::*;

pub mod accounts_ix;
pub mod constants;
pub mod error;
pub mod events;
pub mod instructions;
pub mod state;
pub mod utils;

pub use accounts_ix::*;

declare_id!("tgateWnTjQqyETHFwgHVuLYGokci8jeAENes2oXhfHZ");

#[program]
pub mod tollgate {
    use super::*;

    pub fn initialize(
        ctx: Context<AccountInitialize>,
        params: instructions::InitializeParams,
    ) -> Result<()> {
        instructions::initialize(ctx, params)
    }

    pub fn crank<'info>(
        ctx: Context<'_, '_, '_, 'info, AccountCrank<'info>>,
        params: instructions::CrankParams,
    ) -> Result<()> {
        instructions::crank(ctx, params)
    }
}
