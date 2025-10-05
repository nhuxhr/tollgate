use anchor_lang::prelude::*;

declare_id!("tgateWnTjQqyETHFwgHVuLYGokci8jeAENes2oXhfHZ");

#[program]
pub mod tollgate {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        msg!("Greetings from: {:?}", ctx.program_id);
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize {}
