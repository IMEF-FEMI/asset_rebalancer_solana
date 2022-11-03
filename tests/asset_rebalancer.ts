import * as anchor from "@project-serum/anchor";
import { Program } from "@project-serum/anchor";
import assert from "assert";
import { Keypair, } from "@solana/web3.js";
import { AssetRebalancer } from "../target/types/asset_rebalancer";
import { createAssociatedTokenAccountAndMintTo, getKeypair, getPortfolioInfoAndSigner, getVaultOwnerAndNonce, mintInfo, mintTokens, mintTokensWithDefaultAuthority, sleep, wrapSol } from "../test_utils/helpers";
import { getAssociatedTokenAddress, NATIVE_MINT, RawMint } from "@solana/spl-token";
import {
  OPEN_ORDERS_A_STR,
  OPEN_ORDERS_B_STR,

  PYTH_ETH_PRICE_ACCOUNT,
  PYTH_SOL_PRICE_ACCOUNT,
  serumDexV3,
  USDC_MINT,
  WETH_MINT,

  WSOL_MINT,
} from "../test_utils/constants";
import { Market, OpenOrders, } from '@project-serum/serum';
import { OrderBook, setUpTwoMarkets } from "../test_utils/fakeMarketUtils";

let user: Keypair;
let wSolAccount: anchor.web3.PublicKey;
let wethAccount: anchor.web3.PublicKey;
let usdcAccount: anchor.web3.PublicKey;
let solPercentage = 300;
let wEthPercentage = 700;


let wSolVault: anchor.web3.PublicKey;
let wEthVault: anchor.web3.PublicKey;
let usdcVault: anchor.web3.PublicKey;

let portfolioInfo: anchor.web3.PublicKey;
let vaultSigner: anchor.web3.PublicKey;
let vaultSignerBump: number;

let solMintInfo: RawMint;
let wethMintInfo: RawMint;
let usdcMintInfo: RawMint;


let solUsdcMarket: Market;
let solUsdcMarketDecoded;
let ethUsdcMarket: Market;
let ethUsdcMarketDecoded;

let solUsdcMarketVaultSigner: anchor.web3.PublicKey;
let ethUsdcMarketVaultSigner: anchor.web3.PublicKey;

let openOrderAAccount: anchor.web3.Keypair;
let openOrderBAccount: anchor.web3.Keypair;
let vaultOpenOrderAAccount;
let vaultOpenOrderBAccount;

let orderBook: OrderBook;

let USDC_SCALER: number;
let WETH_SCALER: number;
let SOL_SCALER: number;

describe("asset_rebalancer", () => {
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env();

  anchor.setProvider(provider);

  const program = anchor.workspace.AssetRebalancer as Program<AssetRebalancer>;


  before(async () => {

    orderBook = await setUpTwoMarkets({
      provider,
    })



    solUsdcMarket = await Market.load(provider.connection, orderBook.marketA.address, {}, serumDexV3);
    ethUsdcMarket = await Market.load(provider.connection, orderBook.marketB.address, {}, serumDexV3);
    // solUsdcMarket = await Market.load(provider.connection, wSOL_USDC_MARKET, {}, serumDexV3);
    // ethUsdcMarket = await Market.load(provider.connection, wETH_USDC_MARKET, {}, serumDexV3);

    ethUsdcMarketDecoded = ethUsdcMarket._decoded;
    solUsdcMarketDecoded = solUsdcMarket._decoded;

    [solUsdcMarketVaultSigner,] = await getVaultOwnerAndNonce(
      solUsdcMarketDecoded.ownAddress,
      serumDexV3
    );

    [ethUsdcMarketVaultSigner,] = await getVaultOwnerAndNonce(
      ethUsdcMarketDecoded.ownAddress,
      serumDexV3
    );


    user = getKeypair("user");
    let tx = new anchor.web3.Transaction().add(
      anchor.web3.SystemProgram.transfer({
        fromPubkey: provider.wallet.publicKey,
        toPubkey: user.publicKey,
        lamports: anchor.web3.LAMPORTS_PER_SOL * 1500,
      }),

    );

    await program.provider.sendAndConfirm(tx)



    const pda = await getPortfolioInfoAndSigner(program as anchor.Program, user.publicKey)
    portfolioInfo = pda.key;

    vaultSigner = pda.signer;
    vaultSignerBump = pda.signerBump;

    wSolVault = await getAssociatedTokenAddress(WSOL_MINT, vaultSigner, true);
    wEthVault = await getAssociatedTokenAddress(WETH_MINT, vaultSigner, true);
    usdcVault = await getAssociatedTokenAddress(USDC_MINT, vaultSigner, true);

    wethMintInfo = await mintInfo(provider, WETH_MINT);
    solMintInfo = await mintInfo(provider, WSOL_MINT);
    usdcMintInfo = await mintInfo(provider, USDC_MINT);

    WETH_SCALER = 10 ** wethMintInfo.decimals;
    USDC_SCALER = 10 ** usdcMintInfo.decimals;
    SOL_SCALER = 10 ** solMintInfo.decimals;


    openOrderAAccount = anchor.web3.Keypair.generate();
    openOrderBAccount = anchor.web3.Keypair.generate();


    vaultOpenOrderAAccount = await anchor.web3.PublicKey.findProgramAddress(
      [Buffer.from(OPEN_ORDERS_A_STR), vaultSigner.toBuffer()], program.programId,
    )
    vaultOpenOrderBAccount = await anchor.web3.PublicKey.findProgramAddress(
      [Buffer.from(OPEN_ORDERS_B_STR), vaultSigner.toBuffer()], program.programId,
    )

  })
  it("mints assets (WETH, SOL)", async () => {

    //wrap sol
    wSolAccount = await wrapSol(
      provider,
      user,
      anchor.web3.LAMPORTS_PER_SOL * 1000
    );



    [, wethAccount] = await createAssociatedTokenAccountAndMintTo(
      provider,
      10 ** wethMintInfo.decimals * 1000,
      WETH_MINT,
      user
    );

    [, usdcAccount] = await createAssociatedTokenAccountAndMintTo(
      provider,
      10 ** usdcMintInfo.decimals * 1000,
      USDC_MINT,
      user
    );

    const wSolBalance = await provider.connection.getTokenAccountBalance(wSolAccount,);
    const wethBalance = await provider.connection.getTokenAccountBalance(wethAccount,);
    const usdcBalance = await provider.connection.getTokenAccountBalance(usdcAccount,);

    assert.equal(wSolBalance.value.amount, anchor.web3.LAMPORTS_PER_SOL * 1000);
    assert.equal(wethBalance.value.amount, 10 ** wethMintInfo.decimals * 1000);
    assert.equal(usdcBalance.value.amount, 10 ** usdcMintInfo.decimals * 1000);

  })

  it("deposit tokens", async () => {



    await program.methods
      .deposit(solPercentage, wEthPercentage, vaultSignerBump)
      .accounts({
        tokenAMint: NATIVE_MINT,
        userTokenAAccount: wSolAccount,
        tokenAVault: wSolVault,
        tokenBMint: WETH_MINT,
        userTokenBAccount: wethAccount,
        tokenBVault: wEthVault,
        pcVault: usdcVault,
        pcMint: USDC_MINT,
        vaultSigner: vaultSigner,
        tokenAPythPrice: PYTH_SOL_PRICE_ACCOUNT,
        tokenBPythPrice: PYTH_ETH_PRICE_ACCOUNT,
        portfolioInfo,
        user: user.publicKey,
      })

      .signers([user,])
      .rpc()
      .catch(e => console.log(e))

    const newWsolBalance = await provider.connection.getTokenAccountBalance(wSolAccount);
    const newWEthBalance = await provider.connection.getTokenAccountBalance(wethAccount);
    assert.equal(newWsolBalance.value.amount, 0)
    assert.equal(newWEthBalance.value.amount, 0)

    const newVaultWsolBalance = await provider.connection.getTokenAccountBalance(wSolVault);
    const newVaultWEthBalance = await provider.connection.getTokenAccountBalance(wEthVault);
    assert.equal(newVaultWsolBalance.value.amount, anchor.web3.LAMPORTS_PER_SOL * 1000)
    assert.equal(newVaultWEthBalance.value.amount, 10 ** wethMintInfo.decimals * 1000)
  });



  it("Initializes a fake market and open orders account", async () => {
    const portfolioInfoState = await program.account.portfolioInfo.fetch(portfolioInfo)


    let bumps: {
      vaultAuthority: number;
      openOrdersA: number;
      openOrdersB: number;
    } = {
      vaultAuthority: vaultSignerBump,
      openOrdersA: vaultOpenOrderAAccount[1],
      openOrdersB: vaultOpenOrderBAccount[1],
    };

    await program.methods
      .initAccounts(bumps)
      .accounts({
        openOrdersA: vaultOpenOrderAAccount[0],
        openOrdersB: vaultOpenOrderBAccount[0],
        authority: vaultSigner,
        marketA: orderBook.marketA.address,
        marketB: orderBook.marketB.address,
        dexProgram: serumDexV3,
        portfolioInfo,
        user: user.publicKey,
      })
      .signers([user,])
      .rpc()
      .catch(e => console.log(e));


    // await program.methods
    //   .initAccounts(bumps)
    //   .accounts({
    //     openOrdersA: vaultOpenOrderAAccount[0],
    //     openOrdersB: vaultOpenOrderBAccount[0],
    //     authority: vaultSigner,
    //     marketA: wSOL_USDC_MARKET,
    //     marketB: wETH_USDC_MARKET,
    //     dexProgram: serumDexV3,
    //     portfolioInfo,
    //     user: user.publicKey,
    //   })
    //   .signers([user,])
    //   .rpc()
    //   .catch(e => console.log(e));
  })


  it("rebalance assets", async () => {

    // const newWsolBalanceBefore = await provider.connection.getTokenAccountBalance(wSolVault);
    // console.log("SOL balance before: " + Number(newWsolBalanceBefore.value.amount) / SOL_SCALER);

    let listener = null;
    let [event, slot] = await new Promise((resolve, _reject) => {
      listener = program.addEventListener("AssetsBalanced", (event, slot) => {
        resolve([event, slot]);
      });
      program.methods
        .rebalanceAssets()
        .accounts({
          tokenAMarket: {
            market: solUsdcMarketDecoded.ownAddress,
            requestQueue: solUsdcMarketDecoded.requestQueue,
            eventQueue: solUsdcMarketDecoded.eventQueue,
            bids: solUsdcMarketDecoded.bids,
            asks: solUsdcMarketDecoded.asks,
            coinVault: solUsdcMarketDecoded.baseVault,
            pcVault: solUsdcMarketDecoded.quoteVault,
            vaultSigner: solUsdcMarketVaultSigner,
            // User params.
            openOrders: vaultOpenOrderAAccount[0],
            // Swapping from SOL -> USDC.
            orderPayerTokenAccount: wSolVault,
            coinWallet: wSolVault,
          },
          tokenBMarket: {
            market: ethUsdcMarketDecoded.ownAddress,
            requestQueue: ethUsdcMarketDecoded.requestQueue,
            eventQueue: ethUsdcMarketDecoded.eventQueue,
            bids: ethUsdcMarketDecoded.bids,
            asks: ethUsdcMarketDecoded.asks,
            coinVault: ethUsdcMarketDecoded.baseVault,
            pcVault: ethUsdcMarketDecoded.quoteVault,
            vaultSigner: ethUsdcMarketVaultSigner,
            // User params.
            openOrders: vaultOpenOrderBAccount[0],

            // Swapping from USDC -> ETH.
            orderPayerTokenAccount: wEthVault,
            coinWallet: wEthVault,
          },
          pcWallet: usdcVault,
          vaultSigner: vaultSigner,
          portfolioInfo,
          dexProgram: serumDexV3,
        })
        .rpc()
        .catch(e => console.log(e));

    });

    await program.removeEventListener(listener);

    const newTokenAWorth = event.newTokenAWorth.toNumber();
    const newTokenBWorth = event.newTokenBWorth.toNumber();
    const totalVaultWorth = newTokenAWorth + newTokenBWorth;


    const expectedTokenAPercentage = event.tokenAPercentage / 1000
    const expectedTokenBPercentage = event.tokenBPercentage / 1000

    const newTokenAPercentage = newTokenAWorth / totalVaultWorth;
    const newTokenBPercentage = newTokenBWorth / totalVaultWorth;


    assert.ok(newTokenAPercentage >= expectedTokenAPercentage - 0.5 && newTokenAPercentage <= expectedTokenAPercentage + 0.5);
    assert.ok(newTokenBPercentage >= expectedTokenBPercentage - 0.5 && newTokenBPercentage <= expectedTokenBPercentage + 0.5);

    // const newWsolBalance = await provider.connection.getTokenAccountBalance(wSolVault);
    // console.log("SOL balance after: " + Number(newWsolBalance.value.amount) / SOL_SCALER);


  })






  it('withdraws tokens', async () => {
    const vaultSolBalance = await provider.connection.getTokenAccountBalance(wSolVault);
    const vaultUsdcBalance = await provider.connection.getTokenAccountBalance(usdcVault);
    const vaultWethBalance = await provider.connection.getTokenAccountBalance(wEthVault);

    const formerUserUsdcBalance = await provider.connection.getTokenAccountBalance(usdcAccount);
    const formerUserSolBalance = await provider.connection.getTokenAccountBalance(wSolAccount);
    const formerUserWethBalance = await provider.connection.getTokenAccountBalance(wethAccount);

    await program.methods
      .withdraw()
      .accounts({
        tokenAMint: WSOL_MINT,
        tokenAVault: wSolVault,
        tokenBMint: WETH_MINT,
        pcMint: USDC_MINT,
        userTokenAAccount: wSolAccount,
        userTokenBAccount: wethAccount,
        userPcAccount: usdcAccount,
        tokenBVault: wEthVault,
        pcVault: usdcVault,
        vaultSigner: vaultSigner,
        portfolioInfo,
        user: user.publicKey,
      })
      .signers([user])
      .rpc()
      .catch(e => console.log(e));

    const newUserUsdcBalance = await provider.connection.getTokenAccountBalance(usdcAccount);
    const newUserSolBalance = await provider.connection.getTokenAccountBalance(wSolAccount);
    const newUserWethBalance = await provider.connection.getTokenAccountBalance(wethAccount);

    assert.equal(Number(newUserUsdcBalance.value.amount), Number(formerUserUsdcBalance.value.amount) + Number(vaultUsdcBalance.value.amount));
    assert.equal(Number(newUserSolBalance.value.amount), Number(formerUserSolBalance.value.amount) + Number(vaultSolBalance.value.amount));
    assert.equal(Number(newUserWethBalance.value.amount), Number(formerUserWethBalance.value.amount) + Number(vaultWethBalance.value.amount));

  })

});


// solana logs -u http://127.0.0.1:8899 BAnFYuoxjdNH3rsLrebcFeAyAwvUYmdHsrmsX4CvBF7U