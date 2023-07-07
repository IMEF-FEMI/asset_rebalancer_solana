# Asset Rebalancer

## About

The Asset Rebalancer is an on-chain Solana program designed to rebalance two tokens based on a specified proportion. It utilizes an Automated Market Maker (AMM) to sell the outperforming asset and buy the underperforming asset, maintaining the desired allocation. The program can be used for portfolio rebalancing, a strategy that brings a portfolio back into line with the target asset allocation, making gains through volatility harvesting.

## How It Works

The Asset Rebalancer program operates on-chain and performs the following steps to rebalance the assets:

1. **Deposit**
   - Users can deposit two tokens into the program, along with the desired proportion in which they should be maintained.
   - The deposit function takes the following parameters:
     - Token A percentage: The percentage of the first token in the allocation.
     - Token B percentage: The percentage of the second token in the allocation.
     - Vault signer bump: A unique identifier to secure the vault associated with the deposited tokens.

2. **Refresh Prices**
   - The program periodically refreshes the prices of the assets using an on-chain price oracle.
   - This ensures accurate valuation of the tokens for rebalancing calculations.

3. **Rebalance Assets**
   - The rebalance_assets function is called to execute the rebalancing process.
   - It compares the actual allocation of the tokens with the desired proportion and performs the necessary token swaps on the AMM.
   - This step involves selling the outperforming asset and buying the underperforming asset to realign the allocation.

4. **Withdraw**
   - Users can withdraw their deposited tokens, including any rebalancing gains or losses.
   - The withdrawal function ensures that the tokens are returned to the user's account.

## Portfolio Rebalancing

Portfolio rebalancing is a financial strategy used to bring a portfolio back into line with the target asset allocation. It involves selling outperforming assets and investing in underperforming assets to maintain the original allocation or make gains through volatility harvesting. The Asset Rebalancer program enables portfolio rebalancing by leveraging the asset volatility.

## How to Test

### Prerequisites

Before proceeding, ensure that you have the latest versions of Rust, Solana and the Anchor framework installed. Follow the instructions in the links below:

1. **Set Up Rust**
   - Rust is a programming language used for the protocol. Follow the instructions below to install Rust:
     - Open your web browser and visit the Rust installation page: [Rust Installation Guide](https://www.rust-lang.org/tools/install).
     - Follow the guide provided on the page to install Rust on your system.

2. **Set Up Solana**
   - Solana is the blockchain platform required for running the protocol. Follow the instructions below to install Solana:
     - Open your web browser and visit the Solana installation page: [Solana Installation Guide](https://docs.solana.com/cli/install-solana-cli-tools).
     - Follow the guide provided on the page to install Solana on your system.

3. **Set Up Anchor**
   - Anchor is a framework used for Solana smart contract development. Follow the instructions below to install Anchor:
     - Open your web browser and visit the Solana installation page: [Solana Installation Guide](https://www.anchor-lang.com/docs/installation).
     - Follow the guide provided on the page to install Anchor on your system.

     - Anchor is now installed and ready to be used for the Freelance Escrow Payment Protocol.
### Installation

Follow the step-by-step instructions below to install and test the Asset Rebalancer program:

1. **Clone the Repository**
   - Open your terminal or command prompt.
   - Execute the following command to clone the repository:

     ```
     $ git clone https://github.com/IMEF-FEMI/asset_rebalancer
     ```

   - Navigate to the cloned directory:

     ```
     $ cd asset_rebalancer
     ```

2. **Build and Test the Program Locally**
   - Once you have Solana and Rust installed, proceed with the following commands:
     - Install the required dependencies:

       ```
       $ anchor build
       ```

     - Run the tests to ensure the program functions correctly:

       ```
       $ anchor test
       ```

   - Make sure all the tests pass without errors.

Congratulations! You have successfully installed and tested the Asset Rebalancer program. If you encounter any issues during the installation process, refer to the documentation or seek assistance from the program's support channels.

### Feedback

We greatly appreciate any feedback you have. Please feel free to provide your suggestions and improvements for the program. Feedback and criticisms are always welcome!

Feel free to reach out to me at [@femi_0x](https://twitter.com/femi_0x) on Twitter or simply make a pull request (PR) with your contributions. Thank you for your support!

