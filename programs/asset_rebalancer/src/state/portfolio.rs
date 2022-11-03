use anchor_lang::prelude::*;

#[account]
#[derive(Copy)]
pub struct PortfolioInfo {
    pub owner: Pubkey,
    pub token_a_mint: Pubkey,
    pub token_a_decimals: u8,
    pub token_b_mint: Pubkey,
    pub token_b_decimals: u8,
    pub pc_mint: Pubkey,
    pub pc_decimals: u8,
    pub token_a_vault: Pubkey,
    pub token_b_vault: Pubkey,
    pub pc_vault: Pubkey,
    pub token_a_price_feed: Pubkey,
    pub token_b_price_feed: Pubkey,
    pub token_a_price: i128,
    pub token_b_price: i128,
    pub token_a_percentage: u16,
    pub token_b_percentage: u16,
    pub vault_signer_bump: u8,
    pub last_update_unix: i64,
}

impl PortfolioInfo {
    pub const MAX_SIZE: usize = 32 //token_a_mint
    + 32 //owner
    + 1 //token_a_decimals
    + 32 //token_b_mint
    + 1 //token_b_decimals
    + 32 //pc_mint
    + 1 //pc_decimals
    + 32 //token_a_vault
    + 32 //token_b_vault
    + 32 //pc_vault
    + 32 //token_a_price_feed
    + 32 //token_b_price_feed
    + 16 //token_a_price
    + 16 //token_b_price
    + 2 //token_a_percentage
    + 2 //token_b_percentage
    + 1 //vault_signer_bump
    + 8; //last_update_unix

    pub fn init(
        &mut self,
        owner: Pubkey,
        token_a_mint: Pubkey,
        token_a_decimals: u8,
        token_b_mint: Pubkey,
        token_b_decimals: u8,
        token_a_vault: Pubkey,
        token_b_vault: Pubkey,
        token_a_price_feed: Pubkey,
        token_b_price_feed: Pubkey,
        token_a_price: i128,
        token_b_price: i128,
        pc_vault: Pubkey,
        pc_mint: Pubkey,
        pc_decimals: u8,
        token_a_percentage: u16,
        token_b_percentage: u16,
        vault_signer_bump: u8,
    ) -> Result<()> {
        self.owner = owner;
        self.token_a_mint = token_a_mint;
        self.token_a_decimals = token_a_decimals;
        self.token_b_mint = token_b_mint;
        self.token_b_decimals = token_b_decimals;
        self.token_a_vault = token_a_vault;
        self.token_b_vault = token_b_vault;
        self.token_a_price_feed = token_a_price_feed;
        self.token_b_price_feed = token_b_price_feed;
        self.token_a_price = token_a_price;
        self.token_b_price = token_b_price;
        self.pc_vault = pc_vault;
        self.pc_mint = pc_mint;
        self.pc_decimals = pc_decimals;
        self.token_a_percentage = token_a_percentage;
        self.token_b_percentage = token_b_percentage;
        self.vault_signer_bump = vault_signer_bump;
        self.last_update_unix = Clock::get().unwrap().unix_timestamp;
        Ok(())
    }
}

