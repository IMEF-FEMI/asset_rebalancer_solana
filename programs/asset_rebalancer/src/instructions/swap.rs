use crate::constants::{OPEN_ORDERS_A_STR, OPEN_ORDERS_B_STR};

use anchor_lang::prelude::*;

use anchor_spl::dex::serum_dex::state::OpenOrders;
use anchor_spl::dex::{self, InitOpenOrders};

use crate::{constants::VAULT_SIGNER_STR, state::PortfolioInfo};

// Associated token account for Pubkey::default.
mod empty {
    use super::*;
    declare_id!("HJt8Tjdsc9ms9i4WCZEzhzr4oyf3ANcdzXrNdLPFqm3M");
}

/// Convenience API to initialize an open orders account on the Serum DEX.
pub fn init_accounts<'info>(
    ctx: Context<'_, '_, '_, 'info, InitAccount<'info>>,
    bump: InitOrdersBumpSeeds,
) -> Result<()> {
    let portfolio_info_key = ctx.accounts.portfolio_info.key();

    //Get PDA signer seed of vault owner
    let pda_seeds = &[
        VAULT_SIGNER_STR.as_bytes(),
        portfolio_info_key.as_ref(),
        &[bump.vault_authority],
    ];

    let market_a_ctx = ctx.accounts.init_open_orders_a_context();
    let market_b_ctx = ctx.accounts.init_open_orders_b_context();
    dex::init_open_orders(market_a_ctx.with_signer(&[pda_seeds.as_ref()]))?;
    dex::init_open_orders(market_b_ctx.with_signer(&[pda_seeds.as_ref()]))?;

    Ok(())
}

/// Convenience API to close an open orders account on the Serum DEX.
pub fn close_account<'info>(ctx: Context<CloseAccount>, vault_signer_bump: u8) -> Result<()> {
    let portfolio_info_key = ctx.accounts.portfolio_info.key();
    //Get PDA signer seed of vault owner
    let pda_seeds = &[
        VAULT_SIGNER_STR.as_bytes(),
        portfolio_info_key.as_ref(),
        &[vault_signer_bump],
    ];
    let market_a_ctx = CpiContext::new(
        ctx.accounts.dex_program.clone(),
        ctx.accounts.close_open_orders_a_context(),
    );
    dex::close_open_orders(market_a_ctx.with_signer(&[pda_seeds.as_ref()]))?;

    let market_b_ctx = CpiContext::new(
        ctx.accounts.dex_program.clone(),
        ctx.accounts.close_open_orders_b_context(),
    );
    dex::close_open_orders(market_b_ctx.with_signer(&[pda_seeds.as_ref()]))?;
    Ok(())
}
#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct InitOrdersBumpSeeds {
    pub vault_authority: u8,
    pub open_orders_a: u8,
    pub open_orders_b: u8,
}

#[derive(Accounts)]
#[instruction(bump: InitOrdersBumpSeeds)]
pub struct InitAccount<'info> {
    /// CHECK
    #[account(
        init,
        seeds = [
            OPEN_ORDERS_A_STR.as_ref(),
            authority.key().as_ref()
        ],
        bump,
        payer = user,
        owner = dex::ID,
        space = std::mem::size_of::<OpenOrders>() + 12,
        // rent_exempt = skip,
    )]
    open_orders_a: AccountInfo<'info>,
    /// CHECK
    #[account(
        init,
        seeds = [
            OPEN_ORDERS_B_STR.as_ref(),
            authority.key().as_ref()
        ],
        bump,
        payer = user,
        owner = dex::ID,
        space = std::mem::size_of::<OpenOrders>() + 12,
        // rent_exempt = skip,
    )]
    open_orders_b: AccountInfo<'info>,
    /// CHECK
    authority: AccountInfo<'info>,
    /// CHECK
    market_a: AccountInfo<'info>,
    /// CHECK
    market_b: AccountInfo<'info>,
    /// CHECK
    dex_program: AccountInfo<'info>,
    #[account(mut)]
    user: Signer<'info>,
    #[account(mut)]
    portfolio_info: Box<Account<'info, PortfolioInfo>>,
    pub rent: Sysvar<'info, Rent>,
    system_program: Program<'info, System>,
}

impl<'info> InitAccount<'info> {
    fn init_open_orders_a_context(&self) -> CpiContext<'_, '_, '_, 'info, InitOpenOrders<'info>> {
        CpiContext::new(
            self.dex_program.clone(),
            InitOpenOrders {
                open_orders: self.open_orders_a.clone(),
                authority: self.authority.clone(),
                market: self.market_a.clone(),
                rent: self.rent.to_account_info(),
                // dex_program: self.dex_program,
            },
        )
    }
    fn init_open_orders_b_context(&self) -> CpiContext<'_, '_, '_, 'info, InitOpenOrders<'info>> {
        CpiContext::new(
            self.dex_program.clone(),
            InitOpenOrders {
                open_orders: self.open_orders_b.clone(),
                authority: self.authority.clone(),
                market: self.market_b.clone(),
                rent: self.rent.to_account_info(),
                // dex_program: self.dex_program,
            },
        )
    }
}

#[derive(Accounts)]
pub struct CloseAccount<'info> {
    /// CHECK
    #[account(
        mut,
        seeds = [
            OPEN_ORDERS_A_STR.as_ref(),
            authority.key().as_ref()
        ],
        bump,
    )]
    open_orders_a: AccountInfo<'info>,
    /// CHECK
    #[account(
        mut,
        seeds = [
            OPEN_ORDERS_B_STR.as_ref(),
            authority.key().as_ref()
        ],
        bump,
    )]
    open_orders_b: AccountInfo<'info>,
    /// CHECK
    authority: AccountInfo<'info>,
    /// CHECK
    market_a: AccountInfo<'info>,
    /// CHECK
    market_b: AccountInfo<'info>,
    /// CHECK
    dex_program: AccountInfo<'info>,
    #[account(mut)]
    portfolio_info: Box<Account<'info, PortfolioInfo>>,
    #[account(mut)]
    user: Signer<'info>,
    pub rent: Sysvar<'info, Rent>,
    system_program: Program<'info, System>,
}

impl<'info> CloseAccount<'info> {
    fn close_open_orders_a_context(&self) -> dex::CloseOpenOrders<'info> {
        dex::CloseOpenOrders {
            open_orders: self.open_orders_a.clone(),
            authority: self.authority.clone(),
            destination: self.user.to_account_info().clone(),
            market: self.market_a.clone(),
        }
    }

    fn close_open_orders_b_context(&self) -> dex::CloseOpenOrders<'info> {
        dex::CloseOpenOrders {
            open_orders: self.open_orders_b.clone(),
            authority: self.authority.clone(),
            destination: self.user.to_account_info().clone(),
            market: self.market_b.clone(),
        }
    }
}
