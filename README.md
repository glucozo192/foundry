# PancakeSwap V2 Router Demo with Anvil Fork

A complete Rust program demonstrating PancakeSwap V2 router interaction using a local Anvil fork of BSC (Binance Smart Chain).

## Features

- ✅ Anvil fork setup for BSC with configurable block number
- ✅ Provider and signer configuration with proper middleware
- ✅ Smart contract integration with PancakeSwap V2 Router
- ✅ Token swap simulation with proper error handling
- ✅ Comprehensive logging and execution metrics
- ✅ Production-ready code structure

## Prerequisites

1. **Rust**: Install from [rustup.rs](https://rustup.rs/)
2. **BSC RPC Access**: You can use public BSC RPC or get API key from NodeReal, Ankr, etc.

## Configuration

Before running, update the constants in `src/main.rs`:

```rust
// Replace these with actual values:
const RPC_URL: &str = "https://bsc-dataseed.binance.org/"; // BSC public RPC
const BLOCK_NUMBER: u64 = 35000000; // Recent BSC block
const TOKEN_IN_ADDRESS: &str = "0x55d398326f99059fF775485246999027B3197955"; // USDT
const TOKEN_OUT_ADDRESS: &str = "0xbb4CdB9CBd36B01bD1cBaEBF2De08d9173bc095c"; // WBNB
const AMOUNT_IN: &str = "1000000000000000000"; // 1 USDT (18 decimals)
```

### Example Token Addresses (BSC)

```rust
// Popular BSC tokens for testing:
const WBNB: &str = "0xbb4CdB9CBd36B01bD1cBaEBF2De08d9173bc095c"; // Wrapped BNB (18 decimals)
const USDT: &str = "0x55d398326f99059fF775485246999027B3197955"; // Tether USD (18 decimals)
const BUSD: &str = "0xe9e7CEA3DedcA5984780Bafc599bD69ADd087D56"; // Binance USD (18 decimals)
const USDC: &str = "0x8AC76a51cc950d9822D68b83fE1Ad97B32Cd580d"; // USD Coin (18 decimals)
const CAKE: &str = "0x0E09FaBB73Bd3Ade0a17ECC321fD13a19e81cE82"; // PancakeSwap Token (18 decimals)

// Example amounts (all BSC tokens use 18 decimals):
const AMOUNT_1_TOKEN: &str = "1000000000000000000";       // 1 token
const AMOUNT_10_TOKEN: &str = "10000000000000000000";      // 10 tokens
const AMOUNT_100_TOKEN: &str = "100000000000000000000";    // 100 tokens
```

## Usage

1. **Clone and setup**:
   ```bash
   git clone <your-repo>
   cd uniswap-v2-router-demo
   ```

2. **Update configuration** in `src/main.rs` with your RPC URL and desired tokens

3. **Run the demo**:
   ```bash
   cargo run
   ```

## What the Program Does

1. **Anvil Setup**: Spawns a local Anvil instance forking Ethereum mainnet
2. **Connection**: Establishes provider connection and configures signer
3. **Verification**: Checks connection and displays account information
4. **Contract Setup**: Initializes Uniswap V2 Router and ERC20 contracts
5. **Swap Simulation**: 
   - Checks token balances
   - Calls `getAmountsOut` to preview expected outputs
   - Attempts token approval
   - Executes `swapExactTokensForTokens`
   - Reports transaction details and execution time

## Expected Output

```
🚀 Starting Uniswap V2 Router Demo
✅ Anvil fork started successfully
✅ Provider and signer configured
Current block number: 18500000
Account address: 0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266
Account ETH balance: 1000 ETH
✅ Contracts initialized
🔄 Starting swap simulation...
Expected output amounts: [1000000000000000000, 1850000000000000000]
Expected tokens out: 1850000000000000000
✅ Swap transaction submitted: 0x...
Transaction confirmed in block: Some(18500001)
Gas used: Some(150000)
⏱️  Total execution time: 2.5s
✅ Swap simulation completed successfully
🏁 Demo completed
```

## Error Handling

The program handles various error scenarios:

- **Network connectivity issues**: Proper RPC connection validation
- **Contract call failures**: Graceful error handling with informative messages
- **Insufficient balances**: Balance checks before attempting swaps
- **Invalid addresses**: Address parsing validation
- **Anvil startup failures**: Comprehensive Anvil configuration

## Code Structure

- `main()`: Entry point and orchestration
- `setup_anvil_fork()`: Anvil instance configuration and startup
- `setup_client()`: Provider and signer middleware setup
- `verify_connection()`: Connection validation and account info
- `setup_contracts()`: Contract instantiation with proper ABIs
- `execute_swap_simulation()`: Complete swap workflow
- Helper functions for balance checks, approvals, and swaps

## Dependencies

- `ethers`: Ethereum interaction library
- `ethers-anvil`: Local blockchain forking
- `tokio`: Async runtime
- `eyre`: Error handling
- `tracing`: Structured logging
- `serde`: JSON serialization
- `chrono`: Time handling

## Notes

- The program uses placeholder token addresses by default
- For real testing, replace with actual token addresses
- Anvil provides 1000 ETH per account for testing
- The fork captures the exact state at the specified block
- All transactions are executed on the local fork, not mainnet

## Troubleshooting

1. **RPC URL Issues**: Ensure your RPC URL is valid and has sufficient rate limits
2. **Block Number**: Use a recent block number (within last few thousand blocks)
3. **Token Addresses**: Verify token addresses are valid ERC20 contracts
4. **Network Issues**: Check internet connectivity for initial fork setup

## License

MIT License - feel free to use and modify as needed.
