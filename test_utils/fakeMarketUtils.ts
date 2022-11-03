import * as anchor from "@project-serum/anchor";
import { DexInstructions, Market, TokenInstructions } from "@project-serum/serum";
import { createAssociatedTokenAccount, createTransferCheckedInstruction, createWrappedNativeAccount, TOKEN_PROGRAM_ID } from "@solana/spl-token";
import { Account, Connection, PublicKey, SystemProgram, Transaction } from "@solana/web3.js";
import { createAssociatedTokenAccountWithDefaultPayer, getVaultOwnerAndNonce, mintInfo, mintTokensWithDefaultAuthority } from "./helpers";
import { PYTH_ETH_PRICE_ACCOUNT, PYTH_PROGRAM_ID, serumDexV3, USDC_MINT, WETH_MINT, wETH_USDC_MARKET, WSOL_MINT, wSOL_USDC_MARKET } from "./constants";


// Types
type ListMarket = {
    provider: anchor.AnchorProvider,
    wallet: anchor.web3.Keypair,
    baseMint: anchor.web3.PublicKey,
    quoteMint: anchor.web3.PublicKey,
    baseLotSize: number,
    quoteLotSize: number,
    dexProgramId: anchor.web3.PublicKey,
    feeRateBps: number,
}

type SetupMarket = {
    provider: anchor.AnchorProvider,
    baseMint: anchor.web3.PublicKey,
    quoteMint: anchor.web3.PublicKey,
    bids: number[][],
    asks: number[][],
    marketMaker: {
        account: anchor.web3.Keypair,
        baseToken: anchor.web3.PublicKey,
        quoteToken: anchor.web3.PublicKey,
    }
}

type AddFakeOrders = {
    provider: anchor.AnchorProvider,
    marketPublicKey: anchor.web3.PublicKey,
    price: number,
    marketMaker: {
        account: anchor.web3.Keypair,
        baseToken: anchor.web3.PublicKey,
        quoteToken: anchor.web3.PublicKey,
    }
}
type FundAccount = {
    provider: anchor.AnchorProvider,
    mints: Array<{
        god: anchor.web3.PublicKey,
        mint: anchor.web3.PublicKey,
        amount: number,
        decimals: number
    }>
}
type SetUpTwoMarkets = {
    provider: anchor.AnchorProvider,
}
type MarketMaker = {
    tokens: {
        [key: string]: anchor.web3.PublicKey
    },
    account: anchor.web3.Keypair,
}
export type OrderBook = {
    marketA: Market,
    marketB: Market,
    marketAMarketMaker: {
        account: anchor.web3.Keypair,
        baseToken: anchor.web3.PublicKey,
        quoteToken: anchor.web3.PublicKey,
    },
    marketBMarketMaker: {
        account: anchor.web3.Keypair,
        baseToken: anchor.web3.PublicKey,
        quoteToken: anchor.web3.PublicKey,
    },
    mintA: anchor.web3.PublicKey,
    mintB: anchor.web3.PublicKey,
    usdc: anchor.web3.PublicKey,
    godA: anchor.web3.PublicKey,
    godB: anchor.web3.PublicKey,
    godUsdc: anchor.web3.PublicKey,
}
// fns
async function generateAsksAndBids(marketAddr: PublicKey): Promise<{ asks: number[][]; bids: number[][]; }> {
    let connection = new Connection("https://api.mainnet-beta.solana.com", "confirmed");

    let market = await Market.load(connection, marketAddr, {}, serumDexV3);
    let marketBids = await market.loadBids(connection);
    let bids = [];

    let marketAsks = await market.loadAsks(connection);
    let asks = [];

    //bids
    for (let [price, size] of marketBids.getL2(10)) {
        if (size > 1000) continue;
        bids.push([price, size])
    }

    // asks
    for (let [price, size] of marketAsks.getL2(10)) {
        if (size > 1000) continue;
        asks.push([price, size])
    }


    return {
        asks,
        bids
    }
}

async function generateAsksAndBidsLargeMarketVolume(marketAddr: PublicKey): Promise<{ asks: number[][], bids: number[][] }> {
    let connection = new Connection("https://api.mainnet-beta.solana.com", "confirmed");

    let market = await Market.load(connection, marketAddr, {}, serumDexV3);
    let marketBids = await market.loadBids(connection);
    let bids = [];

    let marketAsks = await market.loadAsks(connection);
    let asks = [];
    //asks

    for (let [price, _] of marketBids.getL2(10)) {
        const randomValue = Math.random()

        bids.push([price, randomValue * 20000])
    }
    for (let [price, _] of marketAsks.getL2(10)) {
        const randomValue = Math.random()

        asks.push([price, randomValue * 20000])
    }
 

    return {
        asks,
        bids
    }
}
export async function setUpTwoMarkets({
    provider,
}: SetUpTwoMarkets): Promise<OrderBook> {

    console.log("Setting up markets...");


    const USDC_MINT_INFO = await mintInfo(provider, USDC_MINT)
    const WETH_MINT_INFO = await mintInfo(provider, WETH_MINT)
    const WSOL_MINT_INFO = await mintInfo(provider, WSOL_MINT)

    // create USDC token account
    const USDC_GOD = await createAssociatedTokenAccountWithDefaultPayer(
        provider,
        USDC_MINT,
        provider.wallet.publicKey,
    );
    const amountToMint = 1_00_000_000;
    // mint 1_000_000_000 USDC(more usdc)
    await mintTokensWithDefaultAuthority(
        provider,
        10_000_000_000 * 10 ** USDC_MINT_INFO.decimals,
        USDC_MINT,
        USDC_GOD
    )
    // create WETH token account
    const WETH_GOD = await createAssociatedTokenAccountWithDefaultPayer(
        provider,
        WETH_MINT,
        provider.wallet.publicKey,
    );

    // mint amountToMint
    await mintTokensWithDefaultAuthority(
        provider,
        amountToMint * 10 ** WETH_MINT_INFO.decimals,
        WETH_MINT,
        WETH_GOD
    )


    //SOL
    const WSOL_GOD = await createWrappedNativeAccount(
        provider.connection,
        (provider.wallet as anchor.Wallet).payer,
        provider.wallet.publicKey,
        anchor.web3.LAMPORTS_PER_SOL * amountToMint,
    );

    // Create a funded account to act as market maker.
    const amount = 100_000_00;
    //create marketMaker acct, two token accts for the market maker and fund them
    // returns the marketMaker and the token accounts
    const marketMaker = await fundAccount({
        provider: provider as any,
        mints: [
            { god: USDC_GOD, mint: USDC_MINT, amount: 1_000_000_000 * 10 ** USDC_MINT_INFO.decimals, decimals: USDC_MINT_INFO.decimals },
            { god: WETH_GOD, mint: WETH_MINT, amount: amount * 10 ** WETH_MINT_INFO.decimals, decimals: WETH_MINT_INFO.decimals },
            { god: WSOL_GOD, mint: WSOL_MINT, amount: amount * 10 ** WSOL_MINT_INFO.decimals, decimals: WSOL_MINT_INFO.decimals },
        ],
    });
    // Setup WSOL/USDC and WETH/USDC markets with resting orders.
    // console.log(generateAsksAndBids(tokenAPrice));
    // console.log(generateAsksAndBids(tokenBPrice));

    const tokenAOrders = await generateAsksAndBidsLargeMarketVolume(wSOL_USDC_MARKET);
    const wSolUsdcAsks = tokenAOrders.asks;
    const wSolUsdcBids = tokenAOrders.bids;
    // console.log(tokenAOrders);


    const tokenBOrders = await generateAsksAndBidsLargeMarketVolume(wETH_USDC_MARKET);
    const wEthUsdcAsks = tokenBOrders.asks;
    const wEthUsdcBids = tokenBOrders.bids;
    // console.log(tokenAOrders);

    const MARKET_SOL_USDC = await setupMarket({
        provider: provider,
        baseMint: WSOL_MINT,
        quoteMint: USDC_MINT,
        bids: wSolUsdcBids,
        asks: wSolUsdcAsks,
        marketMaker: {
            account: marketMaker.account,
            baseToken: marketMaker.tokens[WSOL_MINT.toString()],
            quoteToken: marketMaker.tokens[USDC_MINT.toString()],
        },
    }
    );

    const MARKET_WETH_USDC = await setupMarket({
        provider: provider,
        baseMint: WETH_MINT,
        quoteMint: USDC_MINT,
        bids: wEthUsdcBids,
        asks: wEthUsdcAsks,
        marketMaker: {
            account: marketMaker.account,
            baseToken: marketMaker.tokens[WETH_MINT.toString()],
            quoteToken: marketMaker.tokens[USDC_MINT.toString()],
        },
    }
    );

    return {
        marketA: MARKET_SOL_USDC,
        marketB: MARKET_WETH_USDC,
        marketAMarketMaker: {
            account: marketMaker.account,
            baseToken: marketMaker.tokens[WSOL_MINT.toString()],
            quoteToken: marketMaker.tokens[USDC_MINT.toString()],
        },
        marketBMarketMaker: {
            account: marketMaker.account,
            baseToken: marketMaker.tokens[WSOL_MINT.toString()],
            quoteToken: marketMaker.tokens[USDC_MINT.toString()],
        },
        mintA: WSOL_MINT,
        mintB: WETH_MINT,
        usdc: USDC_MINT,
        godA: WSOL_GOD,
        godB: WETH_GOD,
        godUsdc: USDC_GOD,
    };
}


async function fundAccount({ provider, mints }: FundAccount): Promise<MarketMaker> {
    const MARKET_MAKER = anchor.web3.Keypair.generate();

    const marketMaker = {
        tokens: {},
        account: MARKET_MAKER,
    };

    // Transfer lamports to market maker.
    await provider.sendAndConfirm(
        (() => {
            const tx = new Transaction();
            tx.add(
                SystemProgram.transfer({
                    fromPubkey: provider.wallet.publicKey,
                    toPubkey: MARKET_MAKER.publicKey,
                    lamports: 100000000000,
                })
            );
            return tx;
        })()
    );

    // Transfer SPL tokens to the market maker.
    for (let k = 0; k < mints.length; k += 1) {
        const { mint, god, amount, decimals } = mints[k];
        let MINT_A = mint;
        let GOD_A = god;
        // Setup token accounts owned by the market maker.
        const marketMakerATA = await createAssociatedTokenAccount(
            provider.connection,
            MARKET_MAKER,
            MINT_A,
            MARKET_MAKER.publicKey,
        )


        await provider.sendAndConfirm(
            (() => {
                const tx = new Transaction();
                tx.add(
                    createTransferCheckedInstruction(
                        GOD_A,
                        MINT_A,
                        marketMakerATA,
                        provider.wallet.publicKey,
                        amount,
                        decimals,
                        [],
                        // TOKEN_PROGRAM_ID,
                    )
                );
                return tx;
            })()
        );

        marketMaker.tokens[mint.toString()] = marketMakerATA;
    }

    return marketMaker;
}
async function listMarket({
    provider,
    wallet,
    baseMint,
    quoteMint,
    baseLotSize,
    quoteLotSize,
    dexProgramId,
    feeRateBps,
}: ListMarket): Promise<anchor.web3.PublicKey> {
    const connection = provider.connection;
    const market = anchor.web3.Keypair.generate();
    const requestQueue = anchor.web3.Keypair.generate();
    const eventQueue = anchor.web3.Keypair.generate();
    const bids = anchor.web3.Keypair.generate();
    const asks = anchor.web3.Keypair.generate();
    const baseVault = anchor.web3.Keypair.generate();
    const quoteVault = anchor.web3.Keypair.generate();
    const quoteDustThreshold = new anchor.BN(100);

    const [vaultOwner, vaultSignerNonce] = await getVaultOwnerAndNonce(
        market.publicKey,
        dexProgramId
    );

    const tx1 = new Transaction();
    tx1.add(
        SystemProgram.createAccount({
            fromPubkey: wallet.publicKey,
            newAccountPubkey: baseVault.publicKey,
            lamports: await connection.getMinimumBalanceForRentExemption(165),
            space: 165,
            programId: TOKEN_PROGRAM_ID,
        }),
        SystemProgram.createAccount({
            fromPubkey: wallet.publicKey,
            newAccountPubkey: quoteVault.publicKey,
            lamports: await connection.getMinimumBalanceForRentExemption(165),
            space: 165,
            programId: TOKEN_PROGRAM_ID,
        }),
        TokenInstructions.initializeAccount({
            account: baseVault.publicKey,
            mint: baseMint,
            owner: vaultOwner,
        }),
        TokenInstructions.initializeAccount({
            account: quoteVault.publicKey,
            mint: quoteMint,
            owner: vaultOwner,
        })
    );
    await provider.sendAndConfirm(tx1, [baseVault, quoteVault])

    const tx2 = new Transaction();
    tx2.add(
        SystemProgram.createAccount({
            fromPubkey: wallet.publicKey,
            newAccountPubkey: market.publicKey,
            lamports: await connection.getMinimumBalanceForRentExemption(
                Market.getLayout(dexProgramId).span
            ),
            space: Market.getLayout(dexProgramId).span,
            programId: dexProgramId,
        }),
        SystemProgram.createAccount({
            fromPubkey: wallet.publicKey,
            newAccountPubkey: requestQueue.publicKey,
            lamports: await connection.getMinimumBalanceForRentExemption(5120 + 12),
            space: 5120 + 12,
            programId: dexProgramId,
        }),
        SystemProgram.createAccount({
            fromPubkey: wallet.publicKey,
            newAccountPubkey: eventQueue.publicKey,
            lamports: await connection.getMinimumBalanceForRentExemption(262144 + 12),
            space: 262144 + 12,
            programId: dexProgramId,
        }),
        SystemProgram.createAccount({
            fromPubkey: wallet.publicKey,
            newAccountPubkey: bids.publicKey,
            lamports: await connection.getMinimumBalanceForRentExemption(65536 + 12),
            space: 65536 + 12,
            programId: dexProgramId,
        }),
        SystemProgram.createAccount({
            fromPubkey: wallet.publicKey,
            newAccountPubkey: asks.publicKey,
            lamports: await connection.getMinimumBalanceForRentExemption(65536 + 12),
            space: 65536 + 12,
            programId: dexProgramId,
        }),
        DexInstructions.initializeMarket({
            market: market.publicKey,
            requestQueue: requestQueue.publicKey,
            eventQueue: eventQueue.publicKey,
            bids: bids.publicKey,
            asks: asks.publicKey,
            baseVault: baseVault.publicKey,
            quoteVault: quoteVault.publicKey,
            baseMint,
            quoteMint,
            baseLotSize: new anchor.BN(baseLotSize),
            quoteLotSize: new anchor.BN(quoteLotSize),
            feeRateBps,
            vaultSignerNonce,
            quoteDustThreshold,
            programId: dexProgramId,
        })
    );

    await provider.sendAndConfirm(tx2, [market, requestQueue, eventQueue, bids, asks,])
    return market.publicKey;

}

export async function setupMarket({
    provider,
    baseMint,
    quoteMint,
    bids,
    asks,
    marketMaker,
}: SetupMarket): Promise<Market> {
    const marketAPublicKey = await listMarket({
        provider: provider,
        wallet: provider.wallet as any,
        baseMint: baseMint,
        quoteMint: quoteMint,
        baseLotSize: 100,
        quoteLotSize: 1,
        dexProgramId: serumDexV3,
        feeRateBps: 0,
    });

    const MARKET_A_USDC = await Market.load(
        provider.connection,
        marketAPublicKey,
        { commitment: "recent" },
        serumDexV3
    );

    for (let k = 0; k < asks.length; k += 1) {
        let ask = asks[k];

        const {
            transaction,
            signers,
        } = await MARKET_A_USDC.makePlaceOrderTransaction(provider.connection, {
            owner: marketMaker.account.publicKey,
            payer: marketMaker.baseToken,
            side: "sell",
            price: ask[0],
            size: ask[1],
            orderType: "postOnly",
            clientId: undefined,
            openOrdersAddressKey: undefined,
            openOrdersAccount: undefined,
            feeDiscountPubkey: null,
            selfTradeBehavior: "abortTransaction",
        });
        await provider.sendAndConfirm(transaction, signers.concat(marketMaker.account as unknown as Account));
    }


    for (let k = 0; k < bids.length; k += 1) {
        let bid = bids[k];
        const {
            transaction,
            signers,
        } = await MARKET_A_USDC.makePlaceOrderTransaction(provider.connection, {
            owner: marketMaker.account.publicKey,
            payer: marketMaker.quoteToken,
            side: "buy",
            price: bid[0],
            size: bid[1],
            orderType: "postOnly",
            clientId: undefined,
            openOrdersAddressKey: undefined,
            openOrdersAccount: undefined,
            feeDiscountPubkey: null,
            selfTradeBehavior: "abortTransaction",
        });
        await provider.sendAndConfirm(transaction, signers.concat(marketMaker.account as unknown as Account));

        return MARKET_A_USDC
    }
}

