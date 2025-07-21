use std::sync::Arc;
use ethers::{
    types::{Address, U256},
    contract::Contract,
    abi::Abi,
};
use eyre::Result;
use tracing::{info, warn, error};
use std::str::FromStr;

use crate::config::simple_config::OneInchOrder;
use crate::anvil_setup::{SignerClient, get_token_balance, approve_token, set_token_balance_anvil};

const ONEINCH_ROUTER_ABI: &str = r#"[
    {
        "inputs": [
            {
                "components": [
                    {"internalType": "uint256", "name": "salt", "type": "uint256"},
                    {"internalType": "uint256", "name": "maker", "type": "uint256"},
                    {"internalType": "uint256", "name": "receiver", "type": "uint256"},
                    {"internalType": "uint256", "name": "makerAsset", "type": "uint256"},
                    {"internalType": "uint256", "name": "takerAsset", "type": "uint256"},
                    {"internalType": "uint256", "name": "makingAmount", "type": "uint256"},
                    {"internalType": "uint256", "name": "takingAmount", "type": "uint256"},
                    {"internalType": "uint256", "name": "makerTraits", "type": "uint256"}
                ],
                "internalType": "struct OrderLib.Order",
                "name": "order",
                "type": "tuple"
            },
            {"internalType": "bytes32", "name": "r", "type": "bytes32"},
            {"internalType": "bytes32", "name": "vs", "type": "bytes32"},
            {"internalType": "uint256", "name": "amount", "type": "uint256"},
            {"internalType": "uint256", "name": "takerTraits", "type": "uint256"}
        ],
        "name": "fillOrder",
        "outputs": [
            {"internalType": "uint256", "name": "makingAmount", "type": "uint256"},
            {"internalType": "uint256", "name": "takingAmount", "type": "uint256"},
            {"internalType": "bytes32", "name": "orderHash", "type": "bytes32"}
        ],
        "stateMutability": "payable",
        "type": "function"
    }
]"#;

pub async fn fill_order(order_config: &OneInchOrder, client: &Arc<SignerClient>) -> Result<()> {
    info!("🔄 Executing 1inch order fill simulation...");

    let router_contract = setup_oneinch_contract(client).await?;

    let salt = U256::from_dec_str(&order_config.salt)?;
    let maker = U256::from_dec_str(&order_config.maker)?;
    let receiver = U256::from_dec_str(&order_config.receiver)?;
    let maker_asset = U256::from_dec_str(&order_config.maker_asset)?;
    let taker_asset = U256::from_dec_str(&order_config.taker_asset)?;
    let making_amount = U256::from_dec_str(&order_config.making_amount)?;
    let taking_amount = U256::from_dec_str(&order_config.taking_amount)?;
    let maker_traits = U256::from_dec_str(&order_config.maker_traits)?;
    let amount = U256::from_dec_str(&order_config.amount)?;
    let taker_traits = U256::from_dec_str(&order_config.taker_traits)?;

    // Create order tuple
    let order_tuple = (
        salt,
        maker,
        receiver,
        maker_asset,
        taker_asset,
        making_amount,
        taking_amount,
        maker_traits,
    );

    // Parse signature components
    let r_bytes = hex::decode(&order_config.r.trim_start_matches("0x"))
        .map_err(|e| eyre::eyre!("Failed to decode r: {}", e))?;
    let vs_bytes = hex::decode(&order_config.vs.trim_start_matches("0x"))
        .map_err(|e| eyre::eyre!("Failed to decode vs: {}", e))?;

    let mut r = [0u8; 32];
    let mut vs = [0u8; 32];
    r.copy_from_slice(&r_bytes);
    vs.copy_from_slice(&vs_bytes);

    
    // Setup taker with required tokens and allowance
    let taker = client.address();
    
    // Convert packed addresses back to Address type for balance checks
    let mut taker_asset_bytes = [0u8; 32];
    taker_asset.to_big_endian(&mut taker_asset_bytes);
    let mut addr_bytes = [0u8; 20];
    addr_bytes.copy_from_slice(&taker_asset_bytes[12..32]); // Take last 20 bytes
    let taker_asset_addr = Address::from(addr_bytes);
    
    // Check current balance
    let current_balance = get_token_balance(client, taker_asset_addr, taker).await?;
    info!("📊 Current taker asset balance: {} wei", current_balance);
    
    // We need at least 'amount' tokens to fill the order
    if current_balance < amount {
        info!("⚠️  Insufficient balance. Need {} wei, have {} wei", amount, current_balance);
        // Use Anvil's setBalance for ETH or try to get tokens from a whale
        if taker_asset_addr == Address::zero() {
            // This is ETH, we already have enough
            info!("✅ Using ETH, balance should be sufficient");
        } else {
            // Set token balance directly using Anvil for any ERC20 token
            let required_amount: U256 = amount * 2; // Get 2x what we need for safety
            info!("Setting {} tokens for taker", required_amount.as_u128() as f64 / 1e18);

            // Use Anvil's setBalance to directly give taker the required tokens
            match set_token_balance_anvil(client, taker_asset_addr, taker, required_amount).await {
                Ok(_) => info!("✅ Successfully set token balance for taker"),
                Err(e) => warn!("⚠️  Failed to set token balance: {}", e),
            }
        }
    } else {
        info!("✅ Sufficient balance available");
    }
    
    // Setup allowance for 1inch router
    info!("🔧 Setting up allowance for 1inch router...");
    let router_address = Address::from_str("0x111111125421ca6dc452d289314280a0f8842a65")?;
    match approve_token(client, taker_asset_addr, router_address, amount * 10).await {
        Ok(_) => info!("✅ Successfully approved 1inch router"),
        Err(e) => warn!("⚠️  Failed to approve router: {}", e),
    }
    
    let result = router_contract
        .method::<_, (U256, U256, [u8; 32])>(
            "fillOrder",
            (
                order_tuple,
                r,
                vs,
                amount,
                taker_traits,
            ),
        )?
        .call()
        .await;

    match result {
        Ok((actual_making_amount, actual_taking_amount, order_hash)) => {
            info!("✅ Order fill simulation successful!");
            info!("  Actual Making Amount: {} wei ({:.6} tokens)", 
                  actual_making_amount, actual_making_amount.as_u128() as f64 / 1e18);
            info!("  Actual Taking Amount: {} wei ({:.6} tokens)", 
                  actual_taking_amount, actual_taking_amount.as_u128() as f64 / 1e18);
            info!("  Order Hash: 0x{}", hex::encode(order_hash));
            info!(" Expected Amount Out: {} wei ({:.6} tokens)", 
                  order_config.expected_amount_out, order_config.format_expected_amount_out());
        }
        Err(e) => {
            error!("❌ Order fill simulation failed: {}", e);
            return Err(eyre::eyre!("Order fill failed: {}", e));
        }
    }

    Ok(())
}

async fn setup_oneinch_contract(client: &Arc<SignerClient>) -> Result<Contract<SignerClient>> {
    let router_abi: Abi = serde_json::from_str(ONEINCH_ROUTER_ABI)?;
    let router_address = Address::from_str("0x111111125421ca6dc452d289314280a0f8842a65")?;
    let contract = Contract::new(router_address, router_abi, client.clone());

    info!("📍 Using 1inch Aggregation Router V6: 0x111111125421ca6dc452d289314280a0f8842a65");

    Ok(contract)
}
