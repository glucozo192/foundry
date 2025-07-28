use std::sync::Arc;
use ethers::{
    types::{Address, U256},
    contract::Contract,
    abi::Abi,
    providers::Middleware,
};
use eyre::Result;
use tracing::{info, warn};
use std::str::FromStr;

use crate::config::simple_config::SwapConfig;
use crate::anvil_setup::{SignerClient, set_token_balance_anvil, approve_token, get_token_balance};

// Uniswap V3 SwapRouter ABI - Key functions for swapping
const UNISWAP_V3_ROUTER_ABI: &str = r#"[
    {
        "inputs": [
            {
                "components": [
                    {"internalType": "address", "name": "tokenIn", "type": "address"},
                    {"internalType": "address", "name": "tokenOut", "type": "address"},
                    {"internalType": "uint24", "name": "fee", "type": "uint24"},
                    {"internalType": "address", "name": "recipient", "type": "address"},
                    {"internalType": "uint256", "name": "deadline", "type": "uint256"},
                    {"internalType": "uint256", "name": "amountIn", "type": "uint256"},
                    {"internalType": "uint256", "name": "amountOutMinimum", "type": "uint256"},
                    {"internalType": "uint160", "name": "sqrtPriceLimitX96", "type": "uint160"}
                ],
                "internalType": "struct ISwapRouter.ExactInputSingleParams",
                "name": "params",
                "type": "tuple"
            }
        ],
        "name": "exactInputSingle",
        "outputs": [{"internalType": "uint256", "name": "amountOut", "type": "uint256"}],
        "stateMutability": "payable",
        "type": "function"
    },
    {
        "inputs": [
            {
                "components": [
                    {"internalType": "address", "name": "tokenIn", "type": "address"},
                    {"internalType": "address", "name": "tokenOut", "type": "address"},
                    {"internalType": "uint24", "name": "fee", "type": "uint24"},
                    {"internalType": "address", "name": "recipient", "type": "address"},
                    {"internalType": "uint256", "name": "deadline", "type": "uint256"},
                    {"internalType": "uint256", "name": "amountOut", "type": "uint256"},
                    {"internalType": "uint256", "name": "amountInMaximum", "type": "uint256"},
                    {"internalType": "uint160", "name": "sqrtPriceLimitX96", "type": "uint160"}
                ],
                "internalType": "struct ISwapRouter.ExactOutputSingleParams",
                "name": "params",
                "type": "tuple"
            }
        ],
        "name": "exactOutputSingle",
        "outputs": [{"internalType": "uint256", "name": "amountIn", "type": "uint256"}],
        "stateMutability": "payable",
        "type": "function"
    },
    {
        "inputs": [{"internalType": "uint256", "name": "deadline", "type": "uint256"}],
        "name": "refundETH",
        "outputs": [],
        "stateMutability": "payable",
        "type": "function"
    },
    {
        "inputs": [
            {"internalType": "address", "name": "token", "type": "address"},
            {"internalType": "uint256", "name": "value", "type": "uint256"},
            {"internalType": "uint256", "name": "deadline", "type": "uint256"},
            {"internalType": "uint8", "name": "v", "type": "uint8"},
            {"internalType": "bytes32", "name": "r", "type": "bytes32"},
            {"internalType": "bytes32", "name": "s", "type": "bytes32"}
        ],
        "name": "selfPermit",
        "outputs": [],
        "stateMutability": "payable",
        "type": "function"
    }
]"#;

// Uniswap V3 Pool ABI - For checking pool state
const UNISWAP_V3_POOL_ABI: &str = r#"[
    {
        "inputs": [],
        "name": "slot0",
        "outputs": [
            {"internalType": "uint160", "name": "sqrtPriceX96", "type": "uint160"},
            {"internalType": "int24", "name": "tick", "type": "int24"},
            {"internalType": "uint16", "name": "observationIndex", "type": "uint16"},
            {"internalType": "uint16", "name": "observationCardinality", "type": "uint16"},
            {"internalType": "uint16", "name": "observationCardinalityNext", "type": "uint16"},
            {"internalType": "uint8", "name": "feeProtocol", "type": "uint8"},
            {"internalType": "bool", "name": "unlocked", "type": "bool"}
        ],
        "stateMutability": "view",
        "type": "function"
    },
    {
        "inputs": [],
        "name": "liquidity",
        "outputs": [{"internalType": "uint128", "name": "", "type": "uint128"}],
        "stateMutability": "view",
        "type": "function"
    },
    {
        "inputs": [],
        "name": "token0",
        "outputs": [{"internalType": "address", "name": "", "type": "address"}],
        "stateMutability": "view",
        "type": "function"
    },
    {
        "inputs": [],
        "name": "token1",
        "outputs": [{"internalType": "address", "name": "", "type": "address"}],
        "stateMutability": "view",
        "type": "function"
    },
    {
        "inputs": [],
        "name": "fee",
        "outputs": [{"internalType": "uint24", "name": "", "type": "uint24"}],
        "stateMutability": "view",
        "type": "function"
    }
]"#;

/// Execute a Uniswap V3 swap
pub async fn execute_swap(config: &SwapConfig, client: &Arc<SignerClient>) -> Result<()> {
    info!("🔄 Executing Uniswap V3 swap simulation...");

    // Setup router contract
    let router_contract = setup_router_contract(client, config).await?;

    // Check pool state first
    check_pool_state(client, config).await?;

    // Prepare tokens for swap (fund account and approve router)
    prepare_tokens_for_swap(client, config).await?;

    // Parse amounts
    let amount_in = U256::from_dec_str(&config.amount_in)?;
    let expected_amount_out = U256::from_dec_str(&config.expected_amount_out)?;

    info!("📊 V3 Swap Details:");
    info!("  Amount In: {} wei ({:.6} tokens)", amount_in, amount_in.as_u128() as f64 / 1e18);
    info!("  Expected Out: {} wei ({:.6} tokens)", expected_amount_out, expected_amount_out.as_u128() as f64 / 1e18);
    info!("  Fee Tier: {} basis points", config.fee);

    // Determine swap type and execute
    let token1_addr = Address::from_str(&config.token1)?;
    let token2_addr = Address::from_str(&config.token2)?;
    let wbnb_address = Address::from_str("0xbb4CdB9CBd36B01bD1cBaEBF2De08d9173bc095c")?;

    if token1_addr == wbnb_address {
        // ETH to Token swap
        execute_eth_to_token_swap(&router_contract, config, amount_in, expected_amount_out).await?;
    } else if token2_addr == wbnb_address {
        // Token to ETH swap
        execute_token_to_eth_swap(&router_contract, config, amount_in, expected_amount_out).await?;
    } else {
        // Token to Token swap
        execute_token_to_token_swap(&router_contract, config, amount_in, expected_amount_out).await?;
    }

    Ok(())
}

async fn setup_router_contract(client: &Arc<SignerClient>, config: &SwapConfig) -> Result<Contract<SignerClient>> {
    let router_abi: Abi = serde_json::from_str(UNISWAP_V3_ROUTER_ABI)?;
    let router_address = Address::from_str(config.get_router_address())?;
    let contract = Contract::new(router_address, router_abi, client.clone());

    info!("📍 Using {} Router: {}", config.pool_type.display_name(), config.get_router_address());

    Ok(contract)
}

async fn execute_eth_to_token_swap(
    router_contract: &Contract<SignerClient>,
    config: &SwapConfig,
    amount_in: U256,
    expected_amount_out: U256,
) -> Result<()> {
    info!("🔄 Executing V3 ETH to Token swap...");

    let token_in = Address::from_str(&config.token1)?;
    let token_out = Address::from_str(&config.token2)?;
    let fee = config.fee as u32; // Fee tier (500, 3000, 10000)
    let recipient = router_contract.client().address();
    let deadline = U256::from(chrono::Utc::now().timestamp() + 300); // 5 minutes from now
    let amount_out_minimum = U256::zero(); // Accept any amount of tokens out
    let sqrt_price_limit_x96 = U256::zero(); // No price limit

    info!("🔄 Calling exactInputSingle...");
    info!("  Token In: {}", token_in);
    info!("  Token Out: {}", token_out);
    info!("  Fee: {}", fee);
    info!("  Amount In: {} wei", amount_in);
    info!("  Amount Out Min: {} wei", amount_out_minimum);
    info!("  Recipient: {}", recipient);
    info!("  Deadline: {}", deadline);

    // Create ExactInputSingleParams struct
    let params = (
        token_in,
        token_out,
        fee,
        recipient,
        deadline,
        amount_in,
        amount_out_minimum,
        sqrt_price_limit_x96,
    );

    let call = router_contract
        .method::<_, U256>("exactInputSingle", (params,))?
        .value(amount_in);

    let result = call.call().await?;
    
    info!("✅ exactInputSingle successful!");
    info!("  Amount Out: {} wei ({:.6} tokens)", result, result.as_u128() as f64 / 1e18);
    
    compare_results(config, &result.to_string());

    Ok(())
}

async fn execute_token_to_eth_swap(
    _router_contract: &Contract<SignerClient>,
    _config: &SwapConfig,
    _amount_in: U256,
    _expected_amount_out: U256,
) -> Result<()> {
    info!("🔄 Executing V3 Token to ETH swap...");
    warn!("⚠️  Token to ETH swap not implemented in this demo");
    Ok(())
}

async fn execute_token_to_token_swap(
    router_contract: &Contract<SignerClient>,
    config: &SwapConfig,
    amount_in: U256,
    _expected_amount_out: U256,
) -> Result<()> {
    info!("🔄 Executing V3 Token to Token swap...");

    let token_in = Address::from_str(&config.token1)?;
    let token_out = Address::from_str(&config.token2)?;
    let fee = config.fee as u32; // Fee tier
    let recipient = router_contract.client().address();
    let deadline = U256::from(chrono::Utc::now().timestamp() + 300);
    let amount_out_minimum = U256::zero();
    let sqrt_price_limit_x96 = U256::zero();

    info!("🔄 Calling exactInputSingle for Token to Token...");
    info!("  Token In: {}", token_in);
    info!("  Token Out: {}", token_out);
    info!("  Fee: {}", fee);
    info!("  Amount In: {} wei", amount_in);

    let params = (
        token_in,
        token_out,
        fee,
        recipient,
        deadline,
        amount_in,
        amount_out_minimum,
        sqrt_price_limit_x96,
    );

    let call = router_contract
        .method::<_, U256>("exactInputSingle", (params,))?;

    let result = call.call().await?;
    
    info!("✅ exactInputSingle successful!");
    info!("  Amount Out: {} wei ({:.6} tokens)", result, result.as_u128() as f64 / 1e18);
    
    compare_results(config, &result.to_string());

    Ok(())
}

async fn prepare_tokens_for_swap(client: &Arc<SignerClient>, config: &SwapConfig) -> Result<()> {
    info!("🔧 Preparing tokens for V3 swap...");

    let token_in = Address::from_str(&config.token1)?;
    let amount_in = U256::from_dec_str(&config.amount_in)?;
    let router_address = Address::from_str(config.get_router_address())?;
    let account = client.address();

    // Check if this is an ETH swap (WBNB)
    let wbnb_address = Address::from_str("0xbb4CdB9CBd36B01bD1cBaEBF2De08d9173bc095c")?;

    if token_in == wbnb_address {
        // For ETH swaps, ensure we have enough ETH balance
        let eth_balance = client.get_balance(account, None).await?;
        if eth_balance < amount_in {
            info!("⚠️  Insufficient ETH balance. Setting ETH balance...");
            let required_eth: U256 = amount_in * 2; // Get 2x what we need for safety
            client.provider().request::<_, ()>(
                "anvil_setBalance",
                (account, format!("0x{:x}", required_eth))
            ).await?;
            info!("✅ Set ETH balance: {} ETH", required_eth.as_u128() as f64 / 1e18);
        }
    } else {
        // For token swaps, ensure we have enough token balance
        let current_balance = get_token_balance(client, token_in, account).await?;
        if current_balance < amount_in {
            info!("⚠️  Insufficient token balance. Setting token balance...");
            let required_amount = amount_in * 2; // Get 2x what we need for safety

            match set_token_balance_anvil(client, token_in, account, required_amount).await {
                Ok(_) => info!("✅ Successfully set token balance"),
                Err(e) => {
                    warn!("⚠️  Failed to set token balance: {}", e);
                    return Err(e);
                }
            }
        }

        // Approve router to spend tokens
        info!("🔧 Approving V3 router to spend tokens...");
        let approval_amount = amount_in * 10; // Approve 10x for safety
        match approve_token(client, token_in, router_address, approval_amount).await {
            Ok(_) => info!("✅ Successfully approved V3 router"),
            Err(e) => {
                warn!("⚠️  Failed to approve router: {}", e);
                return Err(e);
            }
        }
    }

    Ok(())
}

async fn check_pool_state(client: &Arc<SignerClient>, config: &SwapConfig) -> Result<()> {
    info!("🔍 Checking V3 pool state...");

    let pool_abi: Abi = serde_json::from_str(UNISWAP_V3_POOL_ABI)?;
    let pool_address = Address::from_str(&config.pool_address)?;
    let pool_contract = Contract::new(pool_address, pool_abi, client.clone());

    // Get slot0 (current price and tick)
    let (sqrt_price_x96, tick, _obs_index, _obs_cardinality, _obs_cardinality_next, _fee_protocol, unlocked):
        (U256, i32, u16, u16, u16, u8, bool) = pool_contract
        .method("slot0", ())?
        .call()
        .await?;

    // Get liquidity
    let liquidity: u128 = pool_contract.method("liquidity", ())?.call().await?;

    // Get token addresses
    let token0: Address = pool_contract.method("token0", ())?.call().await?;
    let token1: Address = pool_contract.method("token1", ())?.call().await?;

    // Get fee tier
    let fee_tier: u32 = pool_contract.method("fee", ())?.call().await?;

    info!("📊 V3 Pool State:");
    info!("  Pool Address: {}", pool_address);
    info!("  Token0: {}", token0);
    info!("  Token1: {}", token1);
    info!("  Fee Tier: {} basis points", fee_tier);
    info!("  Current Tick: {}", tick);
    info!("  Sqrt Price X96: {}", sqrt_price_x96);
    info!("  Liquidity: {}", liquidity);
    info!("  Pool Unlocked: {}", unlocked);

    // Calculate approximate price from sqrtPriceX96
    if sqrt_price_x96 > U256::zero() {
        let sqrt_price_f64 = sqrt_price_x96.as_u128() as f64;
        let price = (sqrt_price_f64 / (2_f64.powi(96))).powi(2);
        info!("💱 Approximate Price (token1/token0): {:.6}", price);
    }

    if !unlocked {
        warn!("⚠️  Pool is locked - swaps may fail");
    }

    if liquidity == 0 {
        warn!("⚠️  Pool has no liquidity - swaps will fail");
    }

    Ok(())
}

fn compare_results(config: &SwapConfig, actual_amount_out: &str) {
    let comparison = config.compare_result(actual_amount_out);

    info!("📊 V3 Swap Result Comparison:");
    info!("  Expected Amount Out: {:.6} tokens", comparison.expected / 1e18);
    info!("  Actual Amount Out: {:.6} tokens", comparison.actual / 1e18);
    info!("  Difference: {:.2}%", comparison.difference_pct);

    if comparison.is_within_tolerance {
        info!("🎉 V3 swap simulation matches expected results!");
    } else {
        warn!("⚠️  Significant difference detected - this may be due to:");
        warn!("    • Pool state changes between blocks");
        warn!("    • Concentrated liquidity effects in V3");
        warn!("    • Price impact from large trades");
        warn!("    • Different fee calculations in V3");
    }
}
