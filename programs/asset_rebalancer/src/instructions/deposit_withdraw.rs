use crate::{
    constants::{PORTFOLIO_INFO_STR, VAULT_SIGNER_STR},
    state::portfolio::PortfolioInfo,
};
use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{CloseAccount, Mint, Token, TokenAccount, Transfer},
};
use pyth_sdk_solana::{load_price_feed_from_account_info, Price, PriceFeed};

pub fn deposit(
    ctx: Context<Deposit>,
    token_a_percentage: u16,
    token_b_percentage: u16,
    vault_signer_bump: u8,
) -> Result<()> {
    //get prices
    assert_eq!(
        ctx.accounts.token_b_vault.owner.key(),
        ctx.accounts.token_a_vault.owner.key(),
    );
    assert_eq!(
        ctx.accounts.pc_vault.owner.key(),
        ctx.accounts.token_a_vault.owner.key(),
    );

    let token_a_price_feed: PriceFeed =
        load_price_feed_from_account_info(&ctx.accounts.token_a_pyth_price).unwrap();
    let token_b_price_feed: PriceFeed =
        load_price_feed_from_account_info(&ctx.accounts.token_b_pyth_price).unwrap();
    let token_a_price: Price = token_a_price_feed.get_current_price().unwrap();
    let token_b_price: Price = token_b_price_feed.get_current_price().unwrap();

    // make transfers
    anchor_spl::token::transfer(
        ctx.accounts.transfer_tokens_a_to_vault(),
        ctx.accounts.user_token_a_account.amount,
    )
    .expect("transfer failed");
    anchor_spl::token::transfer(
        ctx.accounts.transfer_tokens_b_to_vault(),
        ctx.accounts.user_token_b_account.amount,
    )
    .expect("transfer failed");
    // check that both a and b percentage adds up to 1000
    // check chainlink price to see if assets are balanced in the expected proportion
    PortfolioInfo::init(
        &mut ctx.accounts.portfolio_info,
        ctx.accounts.user.key(),
        ctx.accounts.token_a_mint.key(),
        ctx.accounts.token_a_mint.decimals,
        ctx.accounts.token_b_mint.key(),
        ctx.accounts.token_b_mint.decimals,
        ctx.accounts.token_a_vault.key(),
        ctx.accounts.token_b_vault.key(),
        ctx.accounts.token_a_pyth_price.key(),
        ctx.accounts.token_b_pyth_price.key(),
        token_a_price.price.into(),
        token_b_price.price.into(),
        ctx.accounts.pc_vault.key(),
        ctx.accounts.pc_mint.key(),
        ctx.accounts.pc_mint.decimals,
        token_a_percentage,
        token_b_percentage,
        vault_signer_bump,
    )?;
    Ok(())
}

pub fn withdraw(ctx: Context<Withdraw>) -> Result<()> {
    let portfolio_info_key = ctx.accounts.portfolio_info.key().clone();
    //Get PDA signer seed of vault owner
    let pda_seeds = &[
        VAULT_SIGNER_STR.as_bytes(),
        portfolio_info_key.as_ref(),
        &[ctx.accounts.portfolio_info.vault_signer_bump],
    ];

    // make transfers
    anchor_spl::token::transfer(
        ctx.accounts
            .transfer_tokens_a_from_vault()
            .with_signer(&[pda_seeds.as_ref()]),
        ctx.accounts.token_a_vault.amount,
    )
    .expect("transfer failed");
    anchor_spl::token::transfer(
        ctx.accounts
            .transfer_tokens_b_from_vault()
            .with_signer(&[pda_seeds.as_ref()]),
        ctx.accounts.token_b_vault.amount,
    )
    .expect("transfer failed");
    anchor_spl::token::transfer(
        ctx.accounts
            .transfer_pc_tokens_from_vault()
            .with_signer(&[pda_seeds.as_ref()]),
        ctx.accounts.pc_vault.amount,
    )
    .expect("transfer failed");

    //close accounts
    anchor_spl::token::close_account(
        ctx.accounts
            .close_token_a_vault_account_context()
            .with_signer(&[pda_seeds.as_ref()]),
    )?;
    anchor_spl::token::close_account(
        ctx.accounts
            .close_token_b_vault_account_context()
            .with_signer(&[pda_seeds.as_ref()]),
    )?;
    anchor_spl::token::close_account(
        ctx.accounts
            .close_pc_vault_account_context()
            .with_signer(&[pda_seeds.as_ref()]),
    )?;
    Ok(())
}

#[derive(Accounts)]
pub struct Withdraw<'info> {
    token_a_mint: Account<'info, Mint>,
    #[account(
        mut,
        token::mint=token_a_mint,
        token::authority=user,
    )]
    user_token_a_account: Box<Account<'info, TokenAccount>>,
    #[account(
        mut,
        token::mint=token_a_mint,
        token::authority=vault_signer,
    )]
    token_a_vault: Box<Account<'info, TokenAccount>>,

    token_b_mint: Account<'info, Mint>,
    #[account(
        mut,
        token::mint=token_b_mint,
        token::authority=user,
    )]
    user_token_b_account: Box<Account<'info, TokenAccount>>,
    #[account(
        mut,
        token::mint=token_b_mint,
        token::authority=vault_signer,
    )]
    token_b_vault: Box<Account<'info, TokenAccount>>,
    #[account(
        mut,
        token::mint=pc_mint,
        token::authority=vault_signer,
    )]
    pc_vault: Box<Account<'info, TokenAccount>>,
    #[account(
        mut,
        token::mint=pc_mint,
        token::authority=user,
    )]
    user_pc_account: Box<Account<'info, TokenAccount>>,
    pc_mint: Account<'info, Mint>,
    /// CHECK: This is the vault signer Acct
    #[account(
        seeds = [VAULT_SIGNER_STR.as_bytes(), portfolio_info.key().as_ref()],
        bump = portfolio_info.vault_signer_bump,
    )]
    vault_signer: AccountInfo<'info>,
    #[account(
        mut,
        seeds = [PORTFOLIO_INFO_STR.as_bytes(), user.key().as_ref()],
        bump,
    )]
    portfolio_info: Box<Account<'info, PortfolioInfo>>,
    #[account(
        mut,
        constraint = user.key() == portfolio_info.owner
    )]
    user: Signer<'info>,
    token_program: Program<'info, Token>,
    rent: Sysvar<'info, Rent>,
    system_program: Program<'info, System>,
}

impl<'info> Withdraw<'info> {
    pub fn transfer_tokens_a_from_vault(&self) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        let transfer_acct = Transfer {
            from: self.token_a_vault.to_account_info().clone(),
            to: self.user_token_a_account.to_account_info().clone(),
            authority: self.vault_signer.to_account_info().clone(),
        };
        CpiContext::new(self.token_program.to_account_info(), transfer_acct)
    }

    pub fn transfer_tokens_b_from_vault(&self) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        let transfer_acct = Transfer {
            from: self.token_b_vault.to_account_info().clone(),
            to: self.user_token_b_account.to_account_info().clone(),
            authority: self.vault_signer.to_account_info().clone(),
        };
        CpiContext::new(self.token_program.to_account_info(), transfer_acct)
    }
    pub fn transfer_pc_tokens_from_vault(&self) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        let transfer_acct = Transfer {
            from: self.pc_vault.to_account_info().clone(),
            to: self.user_pc_account.to_account_info().clone(),
            authority: self.vault_signer.to_account_info().clone(),
        };
        CpiContext::new(self.token_program.to_account_info(), transfer_acct)
    }
    pub fn close_token_a_vault_account_context(
        &self,
    ) -> CpiContext<'_, '_, '_, 'info, CloseAccount<'info>> {
        let close_accounts = CloseAccount {
            account: self.token_a_vault.to_account_info().clone(),
            destination: self.user.to_account_info().clone(),
            authority: self.vault_signer.to_account_info().clone(),
        };
        CpiContext::new(self.token_program.to_account_info().clone(), close_accounts)
    }
    pub fn close_token_b_vault_account_context(
        &self,
    ) -> CpiContext<'_, '_, '_, 'info, CloseAccount<'info>> {
        let close_accounts = CloseAccount {
            account: self.token_b_vault.to_account_info().clone(),
            destination: self.user.to_account_info().clone(),
            authority: self.vault_signer.to_account_info().clone(),
        };
        CpiContext::new(self.token_program.to_account_info().clone(), close_accounts)
    }
    pub fn close_pc_vault_account_context(
        &self,
    ) -> CpiContext<'_, '_, '_, 'info, CloseAccount<'info>> {
        let close_accounts = CloseAccount {
            account: self.pc_vault.to_account_info().clone(),
            destination: self.user.to_account_info().clone(),
            authority: self.vault_signer.to_account_info().clone(),
        };
        CpiContext::new(self.token_program.to_account_info().clone(), close_accounts)
    }
}
#[derive(Accounts)]
#[instruction(token_a_percentage: u16, token_b_percentage: u16, vault_signer_bump: u8)]
pub struct Deposit<'info> {
    token_a_mint: Account<'info, Mint>,
    #[account(
        mut,
        token::mint=token_a_mint,
        token::authority=user,
    )]
    user_token_a_account: Box<Account<'info, TokenAccount>>,
    #[account(
        init_if_needed,
        payer = user,
        associated_token::mint=token_a_mint,
        associated_token::authority=vault_signer,
    )]
    token_a_vault: Box<Account<'info, TokenAccount>>,

    token_b_mint: Account<'info, Mint>,
    #[account(
        mut,
        token::mint=token_b_mint,
        token::authority=user,
    )]
    user_token_b_account: Box<Account<'info, TokenAccount>>,
    #[account(
        init_if_needed,
        payer = user,
        associated_token::mint=token_b_mint,
        associated_token::authority=vault_signer,
    )]
    token_b_vault: Box<Account<'info, TokenAccount>>,
    #[account(
        init_if_needed,
        payer = user,
        associated_token::mint=pc_mint,
        associated_token::authority=vault_signer,
    )]
    pc_vault: Box<Account<'info, TokenAccount>>,
    pc_mint: Account<'info, Mint>,
    /// CHECK: This is the vault signer Acct
    #[account(
        seeds = [VAULT_SIGNER_STR.as_bytes(), portfolio_info.key().as_ref()],
        bump = vault_signer_bump,
    )]
    vault_signer: AccountInfo<'info>,
    #[account(
        init,
        space = 8 + PortfolioInfo::MAX_SIZE ,
        payer = user,
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
    token_program: Program<'info, Token>,
    associated_token_program: Program<'info, AssociatedToken>,
    rent: Sysvar<'info, Rent>,
    system_program: Program<'info, System>,
}

impl<'info> Deposit<'info> {
    pub fn transfer_tokens_a_to_vault(&self) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        let transfer_acct = Transfer {
            to: self.token_a_vault.to_account_info().clone(),
            from: self.user_token_a_account.to_account_info().clone(),
            authority: self.user.to_account_info().clone(),
        };
        CpiContext::new(self.token_program.to_account_info(), transfer_acct)
    }

    pub fn transfer_tokens_b_to_vault(&self) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        let transfer_acct = Transfer {
            to: self.token_b_vault.to_account_info().clone(),
            from: self.user_token_b_account.to_account_info().clone(),
            authority: self.user.to_account_info().clone(),
        };
        CpiContext::new(self.token_program.to_account_info(), transfer_acct)
    }
}
