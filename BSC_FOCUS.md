# BSC PancakeSwap V2 Router Demo - Cleaned & Focused

## 🎯 What Was Cleaned

### ✅ Removed Complexity
- **Multiple Networks**: Removed Ethereum, Polygon, Arbitrum configurations
- **Advanced Features**: Removed complex slippage calculations, multiple scenarios
- **Unnecessary Dependencies**: Kept only essential crates
- **Complex Examples**: Simplified to basic BSC token swaps

### ✅ BSC-Only Focus
- **Network**: BSC Mainnet (Chain ID: 56)
- **Router**: PancakeSwap V2 Router (`0x10ED43C718714eb63d5aA57B78B54704E256024E`)
- **RPC**: BSC public RPC (`https://bsc-dataseed.binance.org/`)
- **Tokens**: Popular BSC tokens (WBNB, USDT, BUSD, USDC, CAKE)

## 📁 Current Project Structure

```
pancakeswap-v2-router-demo/
├── Cargo.toml                 # Simple dependencies
├── src/main.rs               # BSC-focused main code
├── tests/integration_test.rs # BSC-specific tests
├── examples/
│   ├── config.rs            # BSC swap configurations
│   └── bsc_config.rs        # BSC token definitions
├── scripts/test-compile.sh   # Build verification
├── README.md                # BSC-focused documentation
├── PROJECT_SUMMARY.md       # Updated summary
└── BSC_FOCUS.md             # This file
```

## 🔧 Key Configuration

### Main Constants (src/main.rs)
```rust
const RPC_URL: &str = "https://bsc-dataseed.binance.org/";
const BLOCK_NUMBER: u64 = 35000000;
const ROUTER_ADDRESS: &str = "0x10ED43C718714eb63d5aA57B78B54704E256024E";
const TOKEN_IN_ADDRESS: &str = "0x55d398326f99059fF775485246999027B3197955"; // USDT
const TOKEN_OUT_ADDRESS: &str = "0xbb4CdB9CBd36B01bD1cBaEBF2De08d9173bc095c"; // WBNB
const AMOUNT_IN: &str = "1000000000000000000"; // 1 USDT
```

### BSC Chain Settings
- **Chain ID**: 56 (BSC Mainnet)
- **All tokens**: 18 decimals (simplified)
- **Router**: PancakeSwap V2 only

## 🚀 Quick Start

1. **Install Dependencies**:
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   curl -L https://foundry.paradigm.xyz | bash && foundryup
   ```

2. **Test Build**:
   ```bash
   cargo check
   ```

3. **Run Demo**:
   ```bash
   cargo run
   ```

## 🎯 Supported Swaps

### Popular BSC Token Pairs
- **USDT → WBNB**: Stablecoin to native token
- **BUSD → USDC**: Stablecoin to stablecoin  
- **WBNB → CAKE**: Native token to governance token
- **Any BSC token pair** via PancakeSwap V2

### Example Amounts
```rust
// All BSC tokens use 18 decimals
const AMOUNT_1_TOKEN: &str = "1000000000000000000";    // 1 token
const AMOUNT_10_TOKEN: &str = "10000000000000000000";   // 10 tokens
const AMOUNT_100_TOKEN: &str = "100000000000000000000"; // 100 tokens
```

## 🧪 Testing

```bash
# Run all tests
cargo test

# Test specific functionality
cargo test test_anvil_fork_setup
cargo test test_get_amounts_out_call
```

## 📝 What's Next

### Ready to Use
- ✅ BSC fork setup with Anvil
- ✅ PancakeSwap V2 router integration
- ✅ Token swap simulation
- ✅ Comprehensive error handling
- ✅ Production-ready logging

### Easy to Extend
- Add more BSC tokens
- Implement price monitoring
- Add liquidity pool interactions
- Create trading strategies
- Build web interface

## 🎉 Benefits of This Cleanup

1. **Simpler**: Focus on one network (BSC)
2. **Faster**: Less code to compile and understand
3. **Clearer**: BSC-specific examples and documentation
4. **Practical**: Real BSC tokens and addresses
5. **Educational**: Learn BSC/PancakeSwap without complexity

The project is now focused, clean, and ready for BSC development! 🚀
