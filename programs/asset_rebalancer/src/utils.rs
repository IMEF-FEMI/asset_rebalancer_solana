use std::num::NonZeroU64;

use crate::constants::VAULT_SIGNER_STR;
use crate::errors::ErrorCode;
use crate::instructions::rebalance::{MarketAccounts, Rebalance};

use crate::state::PortfolioInfo;
use anchor_lang::prelude::*;
use anchor_spl::dex::serum_dex::instruction::SelfTradeBehavior;
use anchor_spl::dex::serum_dex::matching::{OrderType, Side as SerumSide};
use anchor_spl::dex::serum_dex::state::MarketState;
use anchor_spl::{dex, token};

pub mod empty {
    use super::*;
    declare_id!("HJt8Tjdsc9ms9i4WCZEzhzr4oyf3ANcdzXrNdLPFqm3M");
}

#[access_control(is_valid_swap_transitive(&ctx))]
pub fn swap_transitive<'info>(
    // ctx: Context<>,
    ctx: Context<Rebalance<'info>>,
    swap_direction: SwapDirection,
    amount: u64,
    min_exchange_rate: ExchangeRate,
) -> Result<()> {
    let from_coin_wallet = if swap_direction.clone() == SwapDirection::AB {
        ctx.accounts.token_a_market.coin_wallet.clone()
    } else {
        ctx.accounts.token_b_market.coin_wallet.clone()
    };
    let to_coin_wallet = if swap_direction.clone() == SwapDirection::AB {
        ctx.accounts.token_b_market.coin_wallet.clone()
    } else {
        ctx.accounts.token_a_market.coin_wallet.clone()
    };

    // Leg 1: Sell Token A for USD(x) (or whatever quote currency is used).
    let (from_amount, sell_proceeds) = {
        // Token balances before the trade.
        let base_before = token::accessor::amount(&from_coin_wallet)?;
        let quote_before = token::accessor::amount(&ctx.accounts.pc_wallet.to_account_info())?;
        // Execute the trade.
        let orderbook = ctx.accounts.orderbook_from(swap_direction.clone());
        orderbook.sell(amount, None)?;
        orderbook.settle(None)?;

        // Token balances after the trade.
        let base_after = token::accessor::amount(&from_coin_wallet)?;
        let quote_after = token::accessor::amount(&ctx.accounts.pc_wallet.to_account_info())?;

        // Report the delta.

        (
            base_before.checked_sub(base_after).unwrap(),
            quote_after.checked_sub(quote_before).unwrap(),
        )
    };
    msg!("--------------------------------");
    msg!("-------------------------------");
    msg!("------------------------------");
    msg!("-----------------------------");
    msg!("----------------------------");
    msg!(
        "amount to swap: {} eth swapped: {}, usdc received: {}",
        amount,
        from_amount,
        sell_proceeds
    );
    // Leg 2: Buy Token B with USD(x) (or whatever quote currency is used).
    let (to_amount, buy_proceeds) = {
        // Token balances before the trade.
        let base_before = token::accessor::amount(&to_coin_wallet)?;
        let quote_before = token::accessor::amount(&ctx.accounts.pc_wallet.to_account_info())?;


        // Execute the trade.
        let orderbook = ctx.accounts.orderbook_to(swap_direction);
        // let amount_to_buy = sell_proceeds
        //     .checked_div(to_price as u64)
        //     .unwrap()
        //     .checked_mul(10u64.checked_pow(to_decimals.into()).unwrap())
        //     .unwrap();

        orderbook.buy(sell_proceeds, None)?;
        orderbook.settle(None)?;

        // Token balances after the trade.
        let base_after = token::accessor::amount(&to_coin_wallet)?;
        let quote_after = token::accessor::amount(&ctx.accounts.pc_wallet.to_account_info())?;

        // Report the delta.
        (
            base_after.checked_sub(base_before).unwrap(),
            quote_before.checked_sub(quote_after).unwrap(),
        )
    };
    msg!("--------------------------------");
    msg!("-------------------------------");
    msg!("------------------------------");
    msg!("-----------------------------");
    msg!("----------------------------");
    msg!(
        " sol obtained: {}, usdc remaining: {}",
        to_amount,
        buy_proceeds
    );

    // The amount of surplus quote currency *not* fully consumed by the
    // second half of the swap.
    let spill_amount = sell_proceeds.checked_sub(buy_proceeds).unwrap();

    // Safety checks.
    apply_risk_checks(DidSwap {
        given_amount: amount,
        min_exchange_rate,
        from_amount,
        to_amount,
        quote_amount: sell_proceeds,
        spill_amount,
        from_mint: token::accessor::mint(&from_coin_wallet)?,
        to_mint: token::accessor::mint(&to_coin_wallet)?,
        quote_mint: token::accessor::mint(&ctx.accounts.pc_wallet.to_account_info())?,
        authority: *ctx.accounts.vault_signer.key,
    })?;

    Ok(())
}

// Asserts the swap event executed at an exchange rate acceptable to the client.
fn apply_risk_checks(event: DidSwap) -> Result<()> {
    // Emit the event for client consumption.
    emit!(event);

    if event.to_amount == 0 {
        return Err(ErrorCode::ZeroSwap.into());
    }

    // Use the exchange rate to calculate the client's expectation.
    //
    // The exchange rate given must always have decimals equal to the
    // `to_mint` decimals, guaranteeing the `min_expected_amount`
    // always has decimals equal to
    //
    // `decimals(from_mint) + decimals(to_mint) + decimals(quote_mint)`.
    //
    // We avoid truncating by adding `decimals(quote_mint)`.
    let min_expected_amount = u128::from(
        // decimals(from).
        event.from_amount,
    )
    .checked_mul(
        // decimals(from) + decimals(to).
        event.min_exchange_rate.rate.into(),
    )
    .unwrap()
    .checked_mul(
        // decimals(from) + decimals(to) + decimals(quote).
        10u128
            .checked_pow(event.min_exchange_rate.quote_decimals.into())
            .unwrap(),
    )
    .unwrap();

    // If there is spill (i.e. quote tokens *not* fully consumed for
    // the buy side of a transitive swap), then credit those tokens marked
    // at the executed exchange rate to create an "effective" to_amount.
    let effective_to_amount = {
        // Translates the leftover spill amount into "to" units via
        //
        // `(to_amount_received/quote_amount_given) * spill_amount`
        //
        let spill_surplus = match event.spill_amount == 0 || event.min_exchange_rate.strict {
            true => 0,
            false => u128::from(
                // decimals(to).
                event.to_amount,
            )
            .checked_mul(
                // decimals(to) + decimals(quote).
                event.spill_amount.into(),
            )
            .unwrap()
            .checked_mul(
                // decimals(to) + decimals(quote) + decimals(from).
                10u128
                    .checked_pow(event.min_exchange_rate.from_decimals.into())
                    .unwrap(),
            )
            .unwrap()
            .checked_mul(
                // decimals(to) + decimals(quote)*2 + decimals(from).
                10u128
                    .checked_pow(event.min_exchange_rate.quote_decimals.into())
                    .unwrap(),
            )
            .unwrap()
            .checked_div(
                // decimals(to) + decimals(quote) + decimals(from).
                event
                    .quote_amount
                    .checked_sub(event.spill_amount)
                    .unwrap()
                    .into(),
            )
            .unwrap(),
        };

        // Translate the `to_amount` into a common number of decimals.
        let to_amount = u128::from(
            // decimals(to).
            event.to_amount,
        )
        .checked_mul(
            // decimals(to) + decimals(from).
            10u128
                .checked_pow(event.min_exchange_rate.from_decimals.into())
                .unwrap(),
        )
        .unwrap()
        .checked_mul(
            // decimals(to) + decimals(from) + decimals(quote).
            10u128
                .checked_pow(event.min_exchange_rate.quote_decimals.into())
                .unwrap(),
        )
        .unwrap();

        to_amount.checked_add(spill_surplus).unwrap()
    };

    // Abort if the resulting amount is less than the client's expectation.
    if effective_to_amount < min_expected_amount {
        msg!(
            "effective_to_amount, min_expected_amount: {:?}, {:?}",
            effective_to_amount,
            min_expected_amount,
        );
        return Err(ErrorCode::SlippageExceeded.into());
    }

    Ok(())
}

// Client for sending orders to the Serum DEX.
#[derive(Clone)]
pub struct OrderbookClient<'info> {
    pub market: MarketAccounts<'info>,
    /// CHECK:
    pub authority: AccountInfo<'info>,
    /// CHECK:
    pub pc_wallet: AccountInfo<'info>,
    /// CHECK:
    pub dex_program: AccountInfo<'info>,
    /// CHECK:
    pub token_program: AccountInfo<'info>,
    /// CHECK:
    pub rent: AccountInfo<'info>,
    pub portfolio_info: Box<Account<'info, PortfolioInfo>>,
}

impl<'info> OrderbookClient<'info> {
    // Executes the sell order portion of the swap, purchasing as much of the
    // quote currency as possible for the given `base_amount`.
    //
    // `base_amount` is the "native" amount of the base currency, i.e., token
    // amount including decimals.
    pub fn sell(
        &self,
        base_amount: u64,
        srm_msrm_discount: Option<AccountInfo<'info>>,
    ) -> Result<()> {
        let limit_price = 1;
        let max_coin_qty = {
            // The loaded market must be dropped before CPI.
            let market = MarketState::load(&self.market.market, &dex::ID).unwrap();
            coin_lots(&market, base_amount)
        };
        let max_native_pc_qty = u64::MAX;
        self.order_cpi(
            limit_price,
            max_coin_qty,
            max_native_pc_qty,
            Side::Ask,
            srm_msrm_discount,
        )
    }
    // Executes the buy order portion of the swap, purchasing as much of the
    // base currency as possible, for the given `quote_amount`.
    //
    // `quote_amount` is the "native" amount of the quote currency, i.e., token
    // amount including decimals.
    pub fn buy(
        &self,
        quote_amount: u64,
        srm_msrm_discount: Option<AccountInfo<'info>>,
    ) -> Result<()> {
        let limit_price = u64::MAX;
        let max_coin_qty = u64::MAX;
        let max_native_pc_qty = quote_amount;
        self.order_cpi(
            limit_price,
            max_coin_qty,
            max_native_pc_qty,
            Side::Bid,
            srm_msrm_discount,
        )
    }

    // Executes a new order on the serum dex via CPI.
    //
    // * `limit_price` - the limit order price in lot units.
    // * `max_coin_qty`- the max number of the base currency lot units.
    // * `max_native_pc_qty` - the max number of quote currency in native token
    //                         units (includes decimals).
    // * `side` - bid or ask, i.e. the type of order.
    // * `referral` - referral account, earning a fee.
    pub fn order_cpi(
        &self,
        limit_price: u64,
        max_coin_qty: u64,
        max_native_pc_qty: u64,
        side: Side,
        srm_msrm_discount: Option<AccountInfo<'info>>,
    ) -> Result<()> {
        // Client order id is only used for cancels. Not used here so hardcode.
        let client_order_id = 0;
        // Limit is the dex's custom compute budge parameter, setting an upper
        // bound on the number of matching cycles the program can perform
        // before giving up and posting the remaining unmatched order.
        let limit = 65535;

        let mut ctx = CpiContext::new(self.dex_program.clone(), self.clone().into());
        if let Some(srm_msrm_discount) = srm_msrm_discount {
            ctx = ctx.with_remaining_accounts(vec![srm_msrm_discount]);
        }
        let portfolio_info_key = self.portfolio_info.key().clone();
        let pda_seeds = &[
            VAULT_SIGNER_STR.as_bytes(),
            portfolio_info_key.as_ref(),
            &[self.portfolio_info.vault_signer_bump],
        ];

        dex::new_order_v3(
            // ctx,
            ctx.with_signer(&[pda_seeds.as_ref()]),
            side.into(),
            NonZeroU64::new(limit_price).unwrap(),
            NonZeroU64::new(max_coin_qty).unwrap(),
            NonZeroU64::new(max_native_pc_qty).unwrap(),
            SelfTradeBehavior::DecrementTake,
            OrderType::ImmediateOrCancel,
            client_order_id,
            limit,
        )
    }
    pub fn settle(&self, referral: Option<AccountInfo<'info>>) -> Result<()> {
        let settle_accs = dex::SettleFunds {
            market: self.market.market.to_account_info().clone(),
            open_orders: self.market.open_orders.to_account_info().clone(),
            open_orders_authority: self.authority.to_account_info().clone(),
            coin_vault: self.market.coin_vault.to_account_info().clone(),
            pc_vault: self.market.pc_vault.to_account_info().clone(),
            coin_wallet: self.market.coin_wallet.to_account_info().clone(),
            pc_wallet: self.pc_wallet.to_account_info().clone(),
            vault_signer: self.market.vault_signer.to_account_info().clone(),
            token_program: self.token_program.to_account_info().clone(),
        };

        let mut ctx = CpiContext::new(self.dex_program.clone(), settle_accs);
        if let Some(referral) = referral {
            ctx = ctx.with_remaining_accounts(vec![referral]);
        }
        let portfolio_info_key = self.portfolio_info.key().clone();
        let pda_seeds = &[
            VAULT_SIGNER_STR.as_bytes(),
            portfolio_info_key.as_ref(),
            &[self.portfolio_info.vault_signer_bump],
        ];

        dex::settle_funds(ctx.with_signer(&[pda_seeds.as_ref()]))
    }
}

impl<'info> From<OrderbookClient<'info>> for dex::NewOrderV3<'info> {
    fn from(c: OrderbookClient<'info>) -> dex::NewOrderV3<'info> {
        dex::NewOrderV3 {
            market: c.market.market.clone(),
            open_orders: c.market.open_orders.clone(),
            request_queue: c.market.request_queue.clone(),
            event_queue: c.market.event_queue.clone(),
            market_bids: c.market.bids.clone(),
            market_asks: c.market.asks.clone(),
            order_payer_token_account: c.market.order_payer_token_account.clone(),
            open_orders_authority: c.authority.clone(),
            coin_vault: c.market.coin_vault.clone(),
            pc_vault: c.market.pc_vault.clone(),
            token_program: c.token_program.to_account_info().clone(),
            rent: c.rent.to_account_info().clone(),
        }
    }
}
#[derive(AnchorSerialize, AnchorDeserialize)]
pub enum Side {
    Bid,
    Ask,
}
#[derive(PartialEq, Clone)]
pub enum SwapDirection {
    AB,
    BA,
}
impl From<Side> for SerumSide {
    fn from(side: Side) -> SerumSide {
        match side {
            Side::Bid => SerumSide::Bid,
            Side::Ask => SerumSide::Ask,
        }
    }
}
// Returns the amount of lots for the base currency of a trade with `size`.
pub fn coin_lots(market: &MarketState, size: u64) -> u64 {
    size.checked_div(market.coin_lot_size).unwrap()
}
#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct VaultSignerSeeds {
    pub portfolio_info_key: Pubkey,
    pub vault_signer_bump: u8,
}
// An exchange rate for swapping *from* one token *to* another.
#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct ExchangeRate {
    // The amount of *to* tokens one should receive for a single *from token.
    // This number must be in native *to* units with the same amount of decimals
    // as the *to* mint.
    pub rate: u64,
    // Number of decimals of the *from* token's mint.
    pub from_decimals: u8,
    // Number of decimals of the *to* token's mint.
    // For a direct swap, this should be zero.
    pub quote_decimals: u8,
    // True if *all* of the *from* currency sold should be used when calculating
    // the executed exchange rate.
    //
    // To perform a transitive swap, one sells on one market and buys on
    // another, where both markets are quoted in the same currency. Now suppose
    // one swaps A for B across A/USDC and B/USDC. Further suppose the first
    // leg swaps the entire *from* amount A for USDC, and then only half of
    // the USDC is used to swap for B on the second leg. How should we calculate
    // the exchange rate?
    //
    // If strict is true, then the exchange rate will be calculated as a direct
    // function of the A tokens lost and B tokens gained, ignoring the surplus
    // in USDC received. If strict is false, an effective exchange rate will be
    // used. I.e. the surplus in USDC will be marked at the exchange rate from
    // the second leg of the swap and that amount will be added to the
    // *to* mint received before calculating the swap's exchange rate.
    //
    // Transitive swaps only. For direct swaps, this field is ignored.
    pub strict: bool,
}

#[event]
pub struct DidSwap {
    // User given (max) amount  of the "from" token to swap.
    pub given_amount: u64,
    // The minimum exchange rate for swapping `from_amount` to `to_amount` in
    // native units with decimals equal to the `to_amount`'s mint--specified
    // by the client.
    pub min_exchange_rate: ExchangeRate,
    // Amount of the `from` token sold.
    pub from_amount: u64,
    // Amount of the `to` token purchased.
    pub to_amount: u64,
    // The amount of the quote currency used for a *transitive* swap. This is
    // the amount *received* for selling on the first leg of the swap.
    pub quote_amount: u64,
    // Amount of the quote currency accumulated from a *transitive* swap, i.e.,
    // the difference between the amount gained from the first leg of the swap
    // (to sell) and the amount used in the second leg of the swap (to buy).
    pub spill_amount: u64,
    // Mint sold.
    pub from_mint: Pubkey,
    // Mint purchased.
    pub to_mint: Pubkey,
    // Mint of the token used as the quote currency in the two markets used
    // for swapping.
    pub quote_mint: Pubkey,
    // User that signed the transaction.
    pub authority: Pubkey,
}

fn is_valid_swap_transitive<'info>(ctx: &Context<Rebalance>) -> Result<()> {
    _is_valid_swap(
        &ctx.accounts.token_a_market.coin_wallet,
        &ctx.accounts.token_b_market.coin_wallet,
    )
}

// Validates the tokens being swapped are of different mints.
fn _is_valid_swap<'info>(from: &AccountInfo<'info>, to: &AccountInfo<'info>) -> Result<()> {
    let from_token_mint = token::accessor::mint(from)?;
    let to_token_mint = token::accessor::mint(to)?;
    if from_token_mint == to_token_mint {
        return Err(ErrorCode::SwapTokensCannotMatch.into());
    }
    Ok(())
}

pub fn calculate_assets_percentage_worth_in_vault(
    token_a_amount: u64,
    token_a_price: u64,
    token_b_amount: u64,
    token_b_price: u64,
) -> u16 {
    let token_a_worth = u128::from(token_a_amount)
        .checked_mul(token_a_price.into())
        .unwrap();
    let token_b_worth = u128::from(token_b_amount)
        .checked_mul(token_b_price.into())
        .unwrap();
    let total_vault_worth = token_a_worth.checked_add(token_b_worth).unwrap();

    // calculate token a percentage
    let token_a_percentage = token_a_worth
        .checked_mul(1000)
        .unwrap()
        .checked_div(total_vault_worth)
        .unwrap();
    u16::from(token_a_percentage as u16)
}

#[test]
pub fn test_percentage_calc() {
    let value = calculate_assets_percentage_worth_in_vault(5, 1250, 3, 200);
    print!("{}", value);
}
