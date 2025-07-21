use std::sync::Arc;
use ethers::{
    types::{Address, U256},
    contract::Contract,
    abi::Abi,
};
use eyre::Result;
use tracing::{info, warn, error};
use std::str::FromStr;

use crate::config::simple_config::SwapConfig;
use crate::anvil_setup::SignerClient;

const UNISWAP_V2_ROUTER_ABI: &str = r#"[
    {
        "inputs": [
            {"internalType": "uint256", "name": "amountIn", "type": "uint256"},
            {"internalType": "address[]", "name": "path", "type": "address[]"},
            {"internalType": "address", "name": "to", "type": "address"},
            {"internalType": "uint256", "name": "deadline", "type": "uint256"}
        ],
        "name": "swapExactETHForTokens",
        "outputs": [{"internalType": "uint256[]", "name": "amounts", "type": "uint256[]"}],
        "stateMutability": "payable",
        "type": "function"
    },
    {
        "inputs": [
            {"internalType": "uint256", "name": "amountOut", "type": "uint256"},
            {"internalType": "address[]", "name": "path", "type": "address[]"},
            {"internalType": "address", "name": "to", "type": "address"},
            {"internalType": "uint256", "name": "deadline", "type": "uint256"}
        ],
        "name": "swapETHForExactTokens",
        "outputs": [{"internalType": "uint256[]", "name": "amounts", "type": "uint256[]"}],
        "stateMutability": "payable",
        "type": "function"
    },
    {
        "inputs": [
            {"internalType": "uint256", "name": "amountIn", "type": "uint256"},
            {"internalType": "uint256", "name": "amountOutMin", "type": "uint256"},
            {"internalType": "address[]", "name": "path", "type": "address[]"},
            {"internalType": "address", "name": "to", "type": "address"},
            {"internalType": "uint256", "name": "deadline", "type": "uint256"}
        ],
        "name": "swapExactTokensForTokens",
        "outputs": [{"internalType": "uint256[]", "name": "amounts", "type": "uint256[]"}],
        "stateMutability": "nonpayable",
        "type": "function"
    }
]"#;

const UNISWAP_V2_PAIR_ABI: &str = r#"[
    {
        "inputs": [],
        "name": "getReserves",
        "outputs": [
            {"internalType": "uint112", "name": "_reserve0", "type": "uint112"},
            {"internalType": "uint112", "name": "_reserve1", "type": "uint112"},
            {"internalType": "uint32", "name": "_blockTimestampLast", "type": "uint32"}
        ],
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
    }
]"#;

pub async fn execute_swap(config: &SwapConfig, client: &Arc<SignerClient>) -> Result<()> {
    info!("🔄 Executing swap simulation...");

    // Setup router contract
    let router_contract = setup_router_contract(client, config).await?;

    // Check pool reserves first
    check_pool_reserves(client, config).await?;

    // Parse amounts
    let amount_in = U256::from_dec_str(&config.amount_in)?;
    let expected_amount_out = U256::from_dec_str(&config.expected_amount_out)?;

    info!("📊 Swap Details:");
    info!("  Amount In: {} wei ({:.6} tokens)", amount_in, amount_in.as_u128() as f64 / 1e18);
    info!("  Expected Out: {} wei ({:.6} tokens)", expected_amount_out, expected_amount_out.as_u128() as f64 / 1e18);

    // Determine swap type and execute
    let token1_addr = Address::from_str(&config.token1)?;
    let token2_addr = Address::from_str(&config.token2)?;
    let wbnb_address = Address::from_str("0xbb4CdB9CBd36B01bD1cBaEBF2De08d9173bc095c")?;

    if token1_addr == wbnb_address {
        // ETH to Token swap
        execute_eth_to_token_swap(&router_contract, config, amount_in, expected_amount_out).await?;
    } else if token2_addr == wbnb_address {
        // Token to ETH swap (not implemented in this example)
        info!("⚠️  Token to ETH swap not implemented in this demo");
    } else {
        // Token to Token swap
        execute_token_to_token_swap(&router_contract, config, amount_in, expected_amount_out).await?;
    }

    Ok(())
}

async fn setup_router_contract(client: &Arc<SignerClient>, config: &SwapConfig) -> Result<Contract<SignerClient>> {
    let router_abi: Abi = serde_json::from_str(UNISWAP_V2_ROUTER_ABI)?;
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
    info!("🔄 Executing ETH to Token swap...");

    let token2_addr = Address::from_str(&config.token2)?;
    let wbnb_address = Address::from_str("0xbb4CdB9CBd36B01bD1cBaEBF2De08d9173bc095c")?;
    let path = vec![wbnb_address, token2_addr];

    // Try swapETHForExactTokens first (more precise)
    match execute_swap_eth_for_exact_tokens(router_contract, &path, expected_amount_out, amount_in).await {
        Ok(amounts) => {
            info!("✅ swapETHForExactTokens successful!");
            info!("  Amounts: {:?}", amounts);
            compare_results(config, &amounts[1].to_string());
        }
        Err(e) => {
            warn!("⚠️  swapETHForExactTokens failed: {}", e);
            info!("🔄 Trying swapExactETHForTokens...");
            
            match execute_swap_exact_eth_for_tokens(router_contract, &path, amount_in).await {
                Ok(amounts) => {
                    info!("✅ swapExactETHForTokens successful!");
                    info!("  Amounts: {:?}", amounts);
                    compare_results(config, &amounts[1].to_string());
                }
                Err(e) => {
                    error!("❌ Both swap methods failed. Last error: {}", e);
                    return Err(e);
                }
            }
        }
    }

    Ok(())
}

async fn execute_swap_eth_for_exact_tokens(
    router_contract: &Contract<SignerClient>,
    path: &[Address],
    amount_out: U256,
    max_amount_in: U256,
) -> Result<Vec<U256>> {
    let deadline = U256::from(chrono::Utc::now().timestamp() + 300); // 5 minutes from now
    let to = router_contract.client().address();

    info!("🔄 Calling swapETHForExactTokens...");
    info!("  Amount Out: {} wei", amount_out);
    info!("  Max Amount In: {} wei", max_amount_in);
    info!("  Path: {:?}", path);
    info!("  To: {}", to);
    info!("  Deadline: {}", deadline);

    let call = router_contract
        .method::<_, Vec<U256>>("swapETHForExactTokens", (amount_out, path.to_vec(), to, deadline))?
        .value(max_amount_in);

    let result = call.call().await?;
    Ok(result)
}

async fn execute_swap_exact_eth_for_tokens(
    router_contract: &Contract<SignerClient>,
    path: &[Address],
    amount_in: U256,
) -> Result<Vec<U256>> {
    let deadline = U256::from(chrono::Utc::now().timestamp() + 300); // 5 minutes from now
    let to = router_contract.client().address();
    let amount_out_min = U256::zero(); // Accept any amount of tokens out

    info!("🔄 Calling swapExactETHForTokens...");
    info!("  Amount In: {} wei", amount_in);
    info!("  Amount Out Min: {} wei", amount_out_min);
    info!("  Path: {:?}", path);
    info!("  To: {}", to);
    info!("  Deadline: {}", deadline);

    let call = router_contract
        .method::<_, Vec<U256>>("swapExactETHForTokens", (amount_out_min, path.to_vec(), to, deadline))?
        .value(amount_in);

    let result = call.call().await?;
    Ok(result)
}

async fn execute_token_to_token_swap(
    router_contract: &Contract<SignerClient>,
    config: &SwapConfig,
    amount_in: U256,
    _expected_amount_out: U256,
) -> Result<()> {
    info!("🔄 Executing Token to Token swap...");

    let token1_addr = Address::from_str(&config.token1)?;
    let token2_addr = Address::from_str(&config.token2)?;
    let path = vec![token1_addr, token2_addr];

    let deadline = U256::from(chrono::Utc::now().timestamp() + 300); // 5 minutes from now
    let to = router_contract.client().address();
    let amount_out_min = U256::zero(); // Accept any amount of tokens out

    info!("🔄 Calling swapExactTokensForTokens...");
    info!("  Amount In: {} wei", amount_in);
    info!("  Amount Out Min: {} wei", amount_out_min);
    info!("  Path: {:?}", path);
    info!("  To: {}", to);
    info!("  Deadline: {}", deadline);

    let result = router_contract
        .method::<_, Vec<U256>>("swapExactTokensForTokens", (amount_in, amount_out_min, path, to, deadline))?
        .call()
        .await?;

    info!("✅ swapExactTokensForTokens successful!");
    info!("  Amounts: {:?}", result);
    compare_results(config, &result[1].to_string());

    Ok(())
}

async fn check_pool_reserves(client: &Arc<SignerClient>, config: &SwapConfig) -> Result<()> {
    info!("🔍 Checking pool reserves...");

    let pair_abi: Abi = serde_json::from_str(UNISWAP_V2_PAIR_ABI)?;
    let pool_address = Address::from_str(&config.pool_address)?;
    let pair_contract = Contract::new(pool_address, pair_abi, client.clone());

    // Get reserves
    let (reserve0, reserve1, _): (U256, U256, u32) = pair_contract
        .method("getReserves", ())?
        .call()
        .await?;

    // Get token addresses
    let token0: Address = pair_contract.method("token0", ())?.call().await?;
    let token1: Address = pair_contract.method("token1", ())?.call().await?;

    info!("📊 Pool Reserves:");
    info!("  Token0 ({}): {} wei ({:.6} tokens)", token0, reserve0, reserve0.as_u128() as f64 / 1e18);
    info!("  Token1 ({}): {} wei ({:.6} tokens)", token1, reserve1, reserve1.as_u128() as f64 / 1e18);

    // Calculate price
    if reserve0 > U256::zero() && reserve1 > U256::zero() {
        let price_0_to_1 = reserve1.as_u128() as f64 / reserve0.as_u128() as f64;
        let price_1_to_0 = reserve0.as_u128() as f64 / reserve1.as_u128() as f64;
        info!("💱 Prices:");
        info!("  1 Token0 = {:.6} Token1", price_0_to_1);
        info!("  1 Token1 = {:.6} Token0", price_1_to_0);
    }

    Ok(())
}

fn compare_results(config: &SwapConfig, actual_amount_out: &str) {
    let comparison = config.compare_result(actual_amount_out);
    
    info!("📊 Swap Result Comparison:");
    info!("  Expected Amount Out: {:.6} tokens", comparison.expected / 1e18);
    info!("  Actual Amount Out: {:.6} tokens", comparison.actual / 1e18);
    info!("  Difference: {:.2}%", comparison.difference_pct);
    
    if comparison.is_within_tolerance {
        info!("🎉 Swap simulation matches expected results!");
    } else {
        warn!("⚠️  Significant difference detected - this may be due to:");
        warn!("    • Pool state changes between blocks");
        warn!("    • Token with transfer fees or special mechanics");
        warn!("    • Price volatility in the pool");
    }
}
