import { PublicKey } from "@solana/web3.js";
import * as anchor from "@project-serum/anchor";

export const PORTFOLIO_INFO_STR = "portfolio_info";
export const VAULT_SIGNER_STR = "vault_signer";
export const TOKEN_A_VAULT_STR = "token_a_vault";
export const TOKEN_B_VAULT_STR = "token_b_vault";
export const PC_VAULT_STR = "pc_vault";
export const OPEN_ORDERS_A_STR = "open_orders_a";
export const OPEN_ORDERS_B_STR = "open_orders_b";




export const PYTH_PROGRAM_ID = new anchor.web3.PublicKey("HEvSKofvBgfaexv23kMabbYqxasxU3mQ4ibBMEmJWHny");

export const PYTH_SOL_PRICE_ACCOUNT = new anchor.web3.PublicKey("H6ARHf6YXhGYeQfUzQNGk6rDNnLBQKrenN712K4AQJEG");
export const PYTH_ETH_PRICE_ACCOUNT = new anchor.web3.PublicKey("JBu1AL4obBcCMqKBBxhpWCNUt136ijcuMZLFvTP7iWdB");



export const WETH_MINT:anchor.web3.PublicKey = new anchor.web3.PublicKey("7vfCXTUXx5WJV5JADk17DUJ4ksgau7utNKj4b963voxs");
export const USDC_MINT:anchor.web3.PublicKey = new anchor.web3.PublicKey("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v")
export const WSOL_MINT:anchor.web3.PublicKey = new anchor.web3.PublicKey("So11111111111111111111111111111111111111112");

export const wSOL_USDC_MARKET = new anchor.web3.PublicKey('9wFFyRfZBsuAha4YcuxcXLKwMxJR43S7fPfQLusDBzvT');
export const wETH_USDC_MARKET = new anchor.web3.PublicKey('8Gmi2HhZmwQPVdCwzS7CM66MGstMXPcTVHA7jF19cLZz');
export const serumDexV3 = new anchor.web3.PublicKey("9xQeWvG816bUx9EPjHmaT23yvVM2ZWbrrpZb9PusVFin"); //serum Dex
