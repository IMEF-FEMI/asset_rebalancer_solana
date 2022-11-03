import * as fs from "fs";
import {
  TOKEN_PROGRAM_ID,
  createWrappedNativeAccount,
  createAssociatedTokenAccountInstruction,
  createMintToInstruction,
  getAssociatedTokenAddress,
  RawAccount,
  AccountLayout,
  getOrCreateAssociatedTokenAccount,
  MintLayout,
  RawMint
} from "@solana/spl-token";
import * as anchor from "@project-serum/anchor";

import { Keypair, PublicKey } from "@solana/web3.js";
import { PORTFOLIO_INFO_STR, VAULT_SIGNER_STR } from "./constants";


const SPL_ASSOCIATED_TOKEN_ACCOUNT_PROGRAM_ID: PublicKey = new PublicKey(
  'ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL',
);


interface PDAParameters {
  key: anchor.web3.PublicKey,
  bump: number,
  signer: anchor.web3.PublicKey,
  signerBump: number,
}
export const getSecretKey = (name: string) =>
  Uint8Array.from(
    JSON.parse(fs.readFileSync(`test_utils/keys/${name}.json`) as unknown as string)
  );

/**
 * gets KeyPair from file
 * @param name name of secretKey file
 * @returns KeyPair
 */
export const getKeypair = (name: string) =>
  Keypair.fromSecretKey(getSecretKey(name));

export const wrapSol = async (
  provider: anchor.AnchorProvider,
  user: Keypair,
  amount: number,
): Promise<PublicKey> => {

  return await createWrappedNativeAccount(
    provider.connection,
    user,
    user.publicKey,
    amount,
    // user
  )
}

export const createAssociatedTokenAccountAndMintTo = async (
  provider: anchor.AnchorProvider,
  amount: number,
  mint: anchor.web3.PublicKey,
  user: anchor.web3.Keypair,
): Promise<[anchor.web3.Keypair, anchor.web3.PublicKey | undefined]> => {
  let userAssociatedTokenAccount = await getAssociatedTokenAddress(
    mint,
    user.publicKey,
  )

  const txFundTokenAccount = new anchor.web3.Transaction();
  txFundTokenAccount.add(createAssociatedTokenAccountInstruction(
    user.publicKey,
    userAssociatedTokenAccount,
    user.publicKey,
    mint,
  ))
  txFundTokenAccount.add(createMintToInstruction(
    mint,
    userAssociatedTokenAccount,
    provider.wallet.publicKey,
    amount,
  ));

  await provider.sendAndConfirm(txFundTokenAccount, [user,]);
  return [user, userAssociatedTokenAccount];
}


export const createAssociatedTokenAccount = async (
  provider: anchor.AnchorProvider,
  mint: anchor.web3.PublicKey,
  user: anchor.web3.Keypair,
): Promise<anchor.web3.PublicKey | undefined> => {
  let ata = await getOrCreateAssociatedTokenAccount(
    provider.connection, //connection
    user, //payer
    mint, //mint
    user.publicKey, //owner
  )
  return ata.address
}

export const createAssociatedTokenAccountWithDefaultPayer = async (
  provider: anchor.AnchorProvider,
  mint: anchor.web3.PublicKey,
  user: anchor.web3.PublicKey,
): Promise<anchor.web3.PublicKey | undefined> => {
  let ata = await getOrCreateAssociatedTokenAccount(
    provider.connection, //connection
    (provider.wallet as anchor.Wallet).payer, //default payer
    mint, //mint
    user, //owner
  )
  return ata.address
}
export const mintTokens = async (
  provider: anchor.AnchorProvider,
  amount: number,
  mint: anchor.web3.PublicKey,
  user: anchor.web3.Keypair,
  userAssociatedTokenAccount: anchor.web3.PublicKey
) => {
  const txFundTokenAccount = new anchor.web3.Transaction();
  txFundTokenAccount.add(createMintToInstruction(
    mint,
    userAssociatedTokenAccount,
    user.publicKey,
    amount,
  ));
  await provider.sendAndConfirm(txFundTokenAccount, [user]);
}

export const mintTokensWithDefaultAuthority = async (
  provider: anchor.AnchorProvider,
  amount: number,
  mint: anchor.web3.PublicKey,
  userAssociatedTokenAccount: anchor.web3.PublicKey
) => {
  const txFundTokenAccount = new anchor.web3.Transaction();
  txFundTokenAccount.add(createMintToInstruction(
    mint,
    userAssociatedTokenAccount,
    provider.wallet.publicKey,
    amount,
  ));
  await provider.sendAndConfirm(txFundTokenAccount,);
}
export async function findAssociatedTokenAddress(
  ownerAddress: PublicKey,
  tokenMintAddress: PublicKey,
  programId: PublicKey = SPL_ASSOCIATED_TOKEN_ACCOUNT_PROGRAM_ID
): Promise<PublicKey> {
  return (await PublicKey.findProgramAddress(
    [
      ownerAddress.toBuffer(),
      TOKEN_PROGRAM_ID.toBuffer(),
      tokenMintAddress.toBuffer(),
    ],
    programId
  ))[0];
}

export const getPortfolioInfoAndSigner = async (program: anchor.Program, user: anchor.web3.PublicKey): Promise<PDAParameters> => {
  const [portfolioInfo, portfolioInfoBump] = await anchor.web3.PublicKey.findProgramAddress(
    [Buffer.from(PORTFOLIO_INFO_STR), user.toBuffer()], program.programId,
  );
  const [vaultSigner, vaultSignerBump] = await anchor.web3.PublicKey.findProgramAddress(
    [Buffer.from(VAULT_SIGNER_STR), portfolioInfo.toBuffer()], program.programId,
  );
  return {
    key: portfolioInfo,
    bump: portfolioInfoBump,
    signer: vaultSigner,
    signerBump: vaultSignerBump
  }
}

export const getPDA = async (program: anchor.Program, seed: string): Promise<[PublicKey, number]> => {
  const [key, bump] = await anchor.web3.PublicKey.findProgramAddress(
    [Buffer.from(seed)], program.programId,
  );

  return [key, bump]
}

export const tokenAccountInfo = async (provider: anchor.Provider, accountPublicKey: anchor.web3.PublicKey,): Promise<RawAccount> => {
  const tokenInfoBuffer = await provider.connection.getAccountInfo(accountPublicKey);
  const data = Buffer.from(tokenInfoBuffer.data);
  const accountInfo: RawAccount = AccountLayout.decode(data);

  return accountInfo;
}


export const mintInfo = async (provider: anchor.Provider,mintPublicKey: anchor.web3.PublicKey, ): Promise<RawMint> => {
  const tokenInfo = await provider.connection.getAccountInfo(mintPublicKey);
  const data = Buffer.from(tokenInfo.data);
  const accountInfo = MintLayout.decode(data);
  return {
    ...accountInfo,
    mintAuthority: accountInfo.mintAuthority == null ? null : anchor.web3.PublicKey.decode(accountInfo.mintAuthority.toBuffer()),
    freezeAuthority: accountInfo.freezeAuthority == null ? null : anchor.web3.PublicKey.decode(accountInfo.freezeAuthority.toBuffer()),
  }
}

export async function getVaultOwnerAndNonce(marketPublicKey, dexProgramId ): Promise<[PublicKey, anchor.BN]> {
  const nonce = new anchor.BN(0);
  while (nonce.toNumber() < 255) {
    try {
      const vaultOwner = await PublicKey.createProgramAddress(
        [marketPublicKey.toBuffer(), nonce.toArrayLike(Buffer, "le", 8)],
        dexProgramId
      );
      return [vaultOwner, nonce];
    } catch (e) {
      nonce.iaddn(1);
    }
  }
  throw new Error("Unable to find nonce");
}

export function sleep(ms) {
  return new Promise((resolve) => {
    setTimeout(resolve, ms);
  });
}