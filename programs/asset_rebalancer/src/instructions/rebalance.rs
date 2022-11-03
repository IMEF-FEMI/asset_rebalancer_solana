use anchor_lang::prelude::*;

use crate::{
    errors::ErrorCode,
    utils::{empty, swap_transitive, ExchangeRate, OrderbookClient, SwapDirection},
};
use anchor_spl::token::{self, Token, TokenAccount};
use pyth_sdk_solana::{load_price_feed_from_account_info, Price, PriceFeed};

use crate::{
    constants::{PORTFOLIO_INFO_STR, VAULT_SIGNER_STR},
    state::PortfolioInfo,
    utils::calculate_assets_percentage_worth_in_vault,
};

pub fn refresh_prices(ctx: Context<RefreshPriceContext>) -> Result<()> {
    //get prices
    let token_a_price_feed: PriceFeed =
        load_price_feed_from_account_info(&ctx.accounts.token_a_pyth_price).unwrap();
    let token_b_price_feed: PriceFeed =
        load_price_feed_from_account_info(&ctx.accounts.token_b_pyth_price).unwrap();
    let token_a_price: Price = token_a_price_feed.get_current_price().unwrap();
    let token_b_price: Price = token_b_price_feed.get_current_price().unwrap();

    ctx.accounts.portfolio_info.token_a_price = token_a_price.price.into();
    ctx.accounts.portfolio_info.token_b_price = token_b_price.price.into();
    Ok(())
}
pub fn rebalance_assets<'info>(ctx: Context<'_, '_, '_, 'info, Rebalance<'info>>) -> Result<()> {
    let portfolio_info = ctx.accounts.portfolio_info.clone();
    let clock = Clock::get().unwrap();
    // Ensure price is recent(within the last minute)
    require!(
        portfolio_info.last_update_unix + 60i64 > clock.unix_timestamp,
        ErrorCode::InvalidPrice
    );
    let token_a_vault = ctx.accounts.token_a_market.coin_wallet.clone();
    let token_b_vault = ctx.accounts.token_b_market.coin_wallet.clone();


    let vault_a_balance = token::accessor::amount(&token_a_vault)
        .unwrap()
        .checked_div(
            10u64
                .checked_pow(portfolio_info.token_a_decimals.into())
                .unwrap(),
        )
        .unwrap();

    let vault_b_balance = token::accessor::amount(&token_b_vault)
        .unwrap()
        .checked_div(
            10u64
                .checked_pow(portfolio_info.token_b_decimals.into())
                .unwrap(),
        )
        .unwrap();

    let current_token_a_percentage = calculate_assets_percentage_worth_in_vault(
        vault_a_balance, //actual amount (eliminating decimals -- so calculations can be balanced )
        portfolio_info.token_a_price as u64,
        vault_b_balance,
        ctx.accounts.portfolio_info.token_b_price as u64,
    );
    let current_token_b_percentage = calculate_assets_percentage_worth_in_vault(
        vault_b_balance,
        ctx.accounts.portfolio_info.token_b_price as u64,
        vault_a_balance,
        portfolio_info.token_a_price as u64,
    );

    let portfolio_info = ctx.accounts.portfolio_info.clone();

    if current_token_a_percentage == portfolio_info.token_a_percentage {
        msg!(
            "portfolio is balanced token a percent:{}, token b percent: {}",
            current_token_a_percentage,
            current_token_b_percentage
        );
    } else if current_token_a_percentage > portfolio_info.token_a_percentage {
        // sell a, buy b
        msg!("A is the outperforming asset");
        msg!(
            "expected a percentage: {}",
            portfolio_info.token_a_percentage
        );
        msg!("current a percentage: {}", current_token_a_percentage);
        msg!("current b percentage: {}", current_token_b_percentage);
        msg!(
            "sell {} of A to buy B",
            (current_token_a_percentage - portfolio_info.token_a_percentage)
        );
        let percentage_to_sell = current_token_a_percentage - portfolio_info.token_a_percentage;

        let amount_to_swap = token::accessor::amount(&token_a_vault)
            .unwrap()
            .checked_mul(percentage_to_sell as u64)
            .unwrap()
            .checked_div(1000u64)
            .unwrap();

        msg!("amount of A to swap: {}", amount_to_swap);
        msg!(
            "current amount of A: {}",
            token::accessor::amount(&token_a_vault).unwrap()
        );

        swap_transitive(
            ctx,
            SwapDirection::AB,
            amount_to_swap,
            ExchangeRate {
                rate: 1,
                from_decimals: portfolio_info.token_a_decimals,
                quote_decimals: portfolio_info.pc_decimals,
                strict: false,
            },
        )?;
    } else {
        // sell b, buy a
        msg!("B is the outperforming asset");
        msg!(
            "expected a percentage: {}",
            portfolio_info.token_b_percentage
        );
        msg!("current a percentage: {}", current_token_a_percentage);
        msg!("current b percentage: {}", current_token_b_percentage);
        msg!(
            "sell {} of B to buy A",
            (current_token_b_percentage - portfolio_info.token_b_percentage)
        );
        let percentage_to_sell = current_token_b_percentage - portfolio_info.token_b_percentage;

        let amount_to_swap = token::accessor::amount(&token_b_vault)
            .unwrap()
            .checked_mul(percentage_to_sell as u64)
            .unwrap()
            .checked_div(1000u64)
            .unwrap();

        msg!("amount of B to swap: {}", amount_to_swap);
        msg!(
            "current amount of B: {}",
            token::accessor::amount(&token_b_vault).unwrap()
        );

        swap_transitive(
            ctx,
            SwapDirection::BA,
            amount_to_swap,
            ExchangeRate {
                rate: 1,
                from_decimals: portfolio_info.token_b_decimals,
                quote_decimals: portfolio_info.pc_decimals,
                strict: false,
            },
        )?;
    }
    let vault_a_balance = token::accessor::amount(&token_a_vault)
        .unwrap()
        .checked_div(
            10u64
                .checked_pow(portfolio_info.token_a_decimals.into())
                .unwrap(),
        )
        .unwrap();

    let vault_b_balance = token::accessor::amount(&token_b_vault)
        .unwrap()
        .checked_div(
            10u64
                .checked_pow(portfolio_info.token_b_decimals.into())
                .unwrap(),
        )
        .unwrap();

    let new_token_a_worth = vault_a_balance
        .checked_mul(portfolio_info.token_a_price as u64)
        .unwrap();

    let new_token_b_worth = vault_b_balance
        .checked_mul(portfolio_info.token_b_price as u64)
        .unwrap();

    emit!(AssetsBalanced {
        new_token_a_worth,
        token_a_percentage: portfolio_info.token_a_percentage,
        new_token_b_worth,
        token_b_percentage: portfolio_info.token_b_percentage,
    });
    Ok(())
}
#[derive(Accounts)]
pub struct RefreshPriceContext<'info> {
    #[account(
            mut,
            seeds = [PORTFOLIO_INFO_STR.as_bytes(), user.key().as_ref()],
            bump,
        )]
    portfolio_info: Box<Account<'info, PortfolioInfo>>,
    /// CHECK:`
    token_a_pyth_price: AccountInfo<'info>,
    /// CHECK:`
    token_b_pyth_price: AccountInfo<'info>,
    #[account(mut)]
    user: Signer<'info>,
}

#[derive(Accounts)]
pub struct Rebalance<'info> {
    pub token_a_market: MarketAccounts<'info>,
    pub token_b_market: MarketAccounts<'info>,

    #[account(mut)]
    pub pc_wallet: Box<Account<'info, TokenAccount>>,
    /// CHECK: This is the vault signer Acct
    #[account(
            // mut,
            seeds = [VAULT_SIGNER_STR.as_bytes(), portfolio_info.key().as_ref()],
            bump,
        )]
    pub vault_signer: AccountInfo<'info>,
    #[account(mut)]
    pub portfolio_info: Box<Account<'info, PortfolioInfo>>,
    /// CHECK:
    pub dex_program: AccountInfo<'info>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
}

impl<'info> Rebalance<'info> {
    pub fn orderbook_from(&mut self, swap_direction: SwapDirection) -> OrderbookClient<'info> {
        if swap_direction == SwapDirection::AB {
            self.token_a_market.order_payer_token_account = self.token_a_market.coin_wallet.clone();
            OrderbookClient {
                market: self.token_a_market.clone(),
                authority: self.vault_signer.clone(),
                pc_wallet: self.pc_wallet.to_account_info().clone(),
                dex_program: self.dex_program.clone(),
                token_program: self.token_program.to_account_info().clone(),
                rent: self.rent.to_account_info().clone(),
                portfolio_info: self.portfolio_info.clone(),
            }
        } else {
            self.token_b_market.order_payer_token_account = self.token_b_market.coin_wallet.clone();

            OrderbookClient {
                market: self.token_b_market.clone(),
                authority: self.vault_signer.clone(),
                pc_wallet: self.pc_wallet.to_account_info().clone(),
                dex_program: self.dex_program.clone(),
                token_program: self.token_program.to_account_info().clone(),
                rent: self.rent.to_account_info().clone(),
                portfolio_info: self.portfolio_info.clone(),
            }
        }
    }
    pub fn orderbook_to(&mut self, swap_direction: SwapDirection) -> OrderbookClient<'info> {
        if swap_direction == SwapDirection::AB {
            self.token_b_market.order_payer_token_account =
                self.pc_wallet.to_account_info().clone();
            OrderbookClient {
                market: self.token_b_market.clone(),
                authority: self.vault_signer.clone(),
                pc_wallet: self.pc_wallet.to_account_info().clone(),
                dex_program: self.dex_program.clone(),
                token_program: self.token_program.to_account_info().clone(),
                rent: self.rent.to_account_info().clone(),
                portfolio_info: self.portfolio_info.clone(),
            }
        } else {
            self.token_a_market.order_payer_token_account =
                self.pc_wallet.to_account_info().clone();
            OrderbookClient {
                market: self.token_a_market.clone(),
                authority: self.vault_signer.clone(),
                pc_wallet: self.pc_wallet.to_account_info().clone(),
                dex_program: self.dex_program.clone(),
                token_program: self.token_program.to_account_info().clone(),
                rent: self.rent.to_account_info().clone(),
                portfolio_info: self.portfolio_info.clone(),
            }
        }
    }
}

// Market accounts are the accounts used to place orders against the dex minus
// common accounts, i.e., program ids, sysvars, and the `pc_wallet`.
#[derive(Accounts, Clone)]
pub struct MarketAccounts<'info> {
    /// CHECK:
    #[account(mut)]
    pub market: AccountInfo<'info>,
    /// CHECK:
    #[account(mut)]
    pub open_orders: AccountInfo<'info>,
    /// CHECK:
    #[account(mut)]
    pub request_queue: AccountInfo<'info>,
    /// CHECK:
    #[account(mut)]
    pub event_queue: AccountInfo<'info>,
    /// CHECK:
    #[account(mut)]
    pub bids: AccountInfo<'info>,
    /// CHECK:
    #[account(mut)]
    pub asks: AccountInfo<'info>,
    // The `spl_token::Account` that funds will be taken from, i.e., transferred
    // from the user into the market's vault.
    //
    // For bids, this is the base currency. For asks, the quote.
    /// CHECK:
    #[account(mut, constraint = order_payer_token_account.key != &empty::ID)]
    pub order_payer_token_account: AccountInfo<'info>,
    // Also known as the "base" currency. For a given A/B market,
    // this is the vault for the A mint.
    /// CHECK:
    #[account(mut)]
    pub coin_vault: AccountInfo<'info>,
    // Also known as the "quote" currency. For a given A/B market,
    // this is the vault for the B mint.
    /// CHECK:
    #[account(mut)]
    pub pc_vault: AccountInfo<'info>,
    // PDA owner of the DEX's token accounts for base + quote currencies
    /// CHECK:.
    pub vault_signer: AccountInfo<'info>,
    // User wallets.
    /// CHECK:
    #[account(mut, constraint = coin_wallet.key != &empty::ID)]
    pub coin_wallet: AccountInfo<'info>,
}

#[event]
pub struct AssetsBalanced {
    new_token_a_worth: u64,
    token_a_percentage: u16,
    new_token_b_worth: u64,
    token_b_percentage: u16,
}
