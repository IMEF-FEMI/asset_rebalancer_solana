use anchor_lang::prelude::*;
pub mod constants;
pub mod errors;
pub mod instructions;
pub mod state;
pub mod utils;

use instructions::*;

// use state::*;

declare_id!("BAnFYuoxjdNH3rsLrebcFeAyAwvUYmdHsrmsX4CvBF7U");

#[program]
pub mod asset_rebalancer {
    use super::*;

    pub fn deposit(
        ctx: Context<Deposit>,
        token_a_percentage: u16,
        token_b_percentage: u16,
        vault_signer_bump: u8,
    ) -> Result<()> {
        instructions::deposit_withdraw::deposit(
            ctx,
            token_a_percentage,
            token_b_percentage,
            vault_signer_bump,
        )
    }

    pub fn withdraw(ctx: Context<Withdraw>) -> Result<()> {
        instructions::deposit_withdraw::withdraw(ctx)
    }
    pub fn refresh_prices(ctx: Context<RefreshPriceContext>) -> Result<()> {
        instructions::rebalance::refresh_prices(ctx)
    }


    pub fn rebalance_assets<'info>(
        ctx: Context<'_, '_, '_, 'info, Rebalance<'info>>,
    ) -> Result<()> {
        instructions::rebalance::rebalance_assets(ctx)
    }

    pub fn init_accounts<'info>(
        ctx: Context<'_, '_, '_, 'info, InitAccount<'info>>,
        bump: InitOrdersBumpSeeds
    ) -> Result<()> {
        instructions::swap::init_accounts(ctx, bump)
    }
    pub fn close_account<'info>(ctx: Context<CloseAccount>, vault_signer_bump: u8) -> Result<()> {
        instructions::swap::close_account(ctx, vault_signer_bump)
    }
}
