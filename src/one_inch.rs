use std::sync::Arc;
use ethers::{
    types::{Address, U256, Bytes, TransactionRequest},
    contract::Contract,
    abi::Abi,
    middleware::Middleware,
};
use eyre::Result;
use tracing::{info, warn, error};
use std::str::FromStr;

use crate::config::simple_config::OneInchOrder;
use crate::anvil_setup::{SignerClient, get_token_balance, approve_token, set_token_balance_anvil};

// 1inch API key for authorization
const ONEINCH_API_KEY: &str = "YOUR_API_KEY_HERE"; // Replace with your actual API key

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
    },
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
            {"internalType": "uint256", "name": "takerTraits", "type": "uint256"},
            {"internalType": "bytes", "name": "args", "type": "bytes"}
        ],
        "name": "fillOrderArgs",
        "outputs": [
            {"internalType": "uint256", "name": "", "type": "uint256"},
            {"internalType": "uint256", "name": "", "type": "uint256"},
            {"internalType": "bytes32", "name": "", "type": "bytes32"}
        ],
        "stateMutability": "payable",
        "type": "function"
    }
]"#;

pub async fn fill_order_args(order_config: &OneInchOrder, extension_data: &str, client: &Arc<SignerClient>) -> Result<()> {
    info!("🔄 Executing 1inch order fill simulation...");

    let router_contract = setup_oneinch_contract(client).await?;

    let salt = U256::from_dec_str(&order_config.salt)?;

    // Convert decimal strings to addresses (they are packed as U256)
    let maker_u256 = U256::from_dec_str(&order_config.maker)?;
    let mut maker_bytes_32 = [0u8; 32];
    maker_u256.to_big_endian(&mut maker_bytes_32);
    let maker_bytes: [u8; 20] = maker_bytes_32[12..].try_into().unwrap(); // Take last 20 bytes
    let maker = Address::from(maker_bytes);

    let receiver_u256 = U256::from_dec_str(&order_config.receiver)?;
    let mut receiver_bytes_32 = [0u8; 32];
    receiver_u256.to_big_endian(&mut receiver_bytes_32);
    let receiver_bytes: [u8; 20] = receiver_bytes_32[12..].try_into().unwrap();
    let receiver = Address::from(receiver_bytes);

    let maker_asset_u256 = U256::from_dec_str(&order_config.maker_asset)?;
    let mut maker_asset_bytes_32 = [0u8; 32];
    maker_asset_u256.to_big_endian(&mut maker_asset_bytes_32);
    let maker_asset_bytes: [u8; 20] = maker_asset_bytes_32[12..].try_into().unwrap();
    let maker_asset = Address::from(maker_asset_bytes);

    let taker_asset_u256 = U256::from_dec_str(&order_config.taker_asset)?;
    let mut taker_asset_bytes_32 = [0u8; 32];
    taker_asset_u256.to_big_endian(&mut taker_asset_bytes_32);
    let taker_asset_bytes: [u8; 20] = taker_asset_bytes_32[12..].try_into().unwrap();
    let taker_asset = Address::from(taker_asset_bytes);

    let making_amount = U256::from_dec_str(&order_config.making_amount)?;
    let taking_amount = U256::from_dec_str(&order_config.taking_amount)?;
    let maker_traits = U256::from_dec_str(&order_config.maker_traits)?;

    let amount = U256::from_dec_str(&order_config.amount)?;


    let r = hex::decode(&order_config.r[2..])
        .map_err(|e| eyre::eyre!("Failed to decode r: {}", e))?;
    let vs = hex::decode(&order_config.vs[2..])
        .map_err(|e| eyre::eyre!("Failed to decode vs: {}", e))?;

    let r: [u8; 32] = r.try_into()
        .map_err(|_| eyre::eyre!("Invalid r length"))?;
    let vs: [u8; 32] = vs.try_into()
        .map_err(|_| eyre::eyre!("Invalid vs length"))?;

    // Convert Address to U256 properly (pad to 32 bytes)
    let mut maker_bytes = [0u8; 32];
    maker_bytes[12..].copy_from_slice(maker.as_bytes());
    let mut receiver_bytes = [0u8; 32];
    receiver_bytes[12..].copy_from_slice(receiver.as_bytes());
    let mut maker_asset_bytes = [0u8; 32];
    maker_asset_bytes[12..].copy_from_slice(maker_asset.as_bytes());
    let mut taker_asset_bytes = [0u8; 32];
    taker_asset_bytes[12..].copy_from_slice(taker_asset.as_bytes());

    let order_tuple = (
        salt,
        U256::from(maker_bytes),
        U256::from(receiver_bytes),
        U256::from(maker_asset_bytes),
        U256::from(taker_asset_bytes),
        making_amount,
        taking_amount,
        maker_traits,
    );

    info!("💰 Adding ERC20 tokens to wallet: {}", client.address());

    // For now, try common ACCESS_TOKEN candidates
    let access_token_candidates = vec![
        Address::from_str("0x0e09fabb73bd3ade0a17ecc321fd13a19e81ce82")?, // CAKE
        Address::from_str("0x55d398326f99059ff775485246999027b3197955")?, // USDT
        Address::from_str("0xbb4cdb9cbd36b01bd1cbaebf2de08d9173bc095c")?, // WBNB
    ];

    for (i, candidate_address) in access_token_candidates.iter().enumerate() {
        info!("🧪 Testing ACCESS_TOKEN candidate #{}: {}", i + 1, candidate_address);

        // Add tokens to wallet
        let access_token_amount = U256::from(1000000) * U256::exp10(18); // 1M tokens
        set_token_balance_anvil(client, *candidate_address, client.address(), access_token_amount).await?;

        let balance = get_token_balance(client, *candidate_address, client.address()).await?;
        info!("✅ Added {} tokens for candidate #{}", balance.as_u128() as f64 / 1e18, i + 1);
    }

    // // Verify all balances before order execution
    // info!("🔍 Verifying ACCESS_TOKEN balances before order execution:");
    // for (i, candidate_address) in access_token_candidates.iter().enumerate() {
    //     let balance = get_token_balance(client, *candidate_address, client.address()).await?;
    //     let balance_f64 = balance.as_u128() as f64 / 1e18;

    //     if balance > U256::zero() {
    //         info!("✅ Candidate #{} ({}): {} tokens (balanceOf != 0)", i + 1, candidate_address, balance_f64);
    //     } else {
    //         warn!("❌ Candidate #{} ({}): {} tokens (balanceOf == 0)", i + 1, candidate_address, balance_f64);
    //     }
    // }

    // Parse extension data
    let extension_bytes = if extension_data.starts_with("0x") {
        hex::decode(&extension_data[2..])
            .map_err(|e| eyre::eyre!("Failed to decode extension: {}", e))?
    } else {
        hex::decode(extension_data)
            .map_err(|e| eyre::eyre!("Failed to decode extension: {}", e))?
    };

    let built_taker_traits = build_taker_traits_with_extension(&extension_bytes);

    let built_args = build_fillorder_args(&extension_bytes, None, None);

    return execute_fill_order_args(
        client,
        &router_contract,
        order_tuple,
        r, vs, amount, built_taker_traits,
        ethers::types::Bytes::from(built_args)
    ).await;
}



pub async fn fill_order(order_config: &OneInchOrder, extension_data: &str, client: &Arc<SignerClient>) -> Result<()> {
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

    let amount =  U256::from_dec_str(&order_config.amount)?;
    let r = hex::decode(&order_config.r[2..])
        .map_err(|e| eyre::eyre!("Failed to decode r: {}", e))?;
    let vs = hex::decode(&order_config.vs[2..])
        .map_err(|e| eyre::eyre!("Failed to decode vs: {}", e))?;

    let r: [u8; 32] = r.try_into()
        .map_err(|_| eyre::eyre!("Invalid r length"))?;
    let vs: [u8; 32] = vs.try_into()
        .map_err(|_| eyre::eyre!("Invalid vs length"))?;

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

    info!("💰 Adding ERC20 tokens to wallet: {}", client.address());

    let built_taker_traits = U256::zero();

    let taker_adress = client.address();

    return execute_fill_order_standard(
        &router_contract,
        order_tuple,
        r, vs, amount, built_taker_traits
    ).await;
}


/// Execute standard fillOrder (8 fields)
async fn execute_fill_order_standard(
    router_contract: &Contract<SignerClient>,
    order_tuple: (U256, U256, U256, U256, U256, U256, U256, U256),
    r: [u8; 32],
    vs: [u8; 32],
    amount: U256,
    taker_traits: U256,
) -> Result<()> {
    info!("� Executing fillOrder...");

    // Setup taker with required tokens and allowance
    let client = router_contract.client();
    let taker = client.address();
    
    // Convert packed addresses back to Address type for balance checks
    let mut taker_asset_bytes = [0u8; 32];
    order_tuple.4.to_big_endian(&mut taker_asset_bytes); // taker_asset is 5th element
    let mut addr_bytes = [0u8; 20];
    addr_bytes.copy_from_slice(&taker_asset_bytes[12..32]); // Take last 20 bytes
    let taker_asset_addr = Address::from(addr_bytes);

    // Check current balance
    let current_balance = get_token_balance(&client, taker_asset_addr, taker).await?;
    
    // We need at least 'amount' tokens to fill the order
    if current_balance < amount {
        info!("Insufficient balance. Need {} wei, have {} wei", amount, current_balance);
        // Use Anvil's setBalance for ETH or try to get tokens from a whale
        if taker_asset_addr == Address::zero() {
            info!("Using ETH, balance should be sufficient");
        } else {
            // Set token balance directly using Anvil for any ERC20 token
            let required_amount: U256 = amount * 2; // Get 2x what we need for safety
            info!("Setting {} tokens for taker", required_amount.as_u128() as f64 / 1e18);

            // Use Anvil's setBalance to directly give taker the required tokens
            match set_token_balance_anvil(&client, taker_asset_addr, taker, required_amount).await {
                Ok(_) => info!("✅ Successfully set token balance for taker"),
                Err(e) => {
                    warn!("⚠️  Failed to set token balance: {}", e);
                    return Err(e);
                }
            }
        }
    } else {
        info!("✅ Sufficient balance available");
    }

    let recheck_current_balance = get_token_balance(&client, taker_asset_addr, taker).await?;
    info!("Recheck Current taker asset balance: {} wei", recheck_current_balance);

    let allowance_amount: U256 = amount * 10; // Approve 10x for safety
    match approve_token(&client, taker_asset_addr, router_contract.address(), allowance_amount).await {
        Ok(_) => info!("Successfully approved 1inch router"),
        Err(e) => {
            warn!("Failed to approve router: {}", e);
            return Err(e);
        }
    }
    
    // Debug: Print all parameters before calling
    info!("  Debug fillOrder parameters:");
    info!("  Order tuple: {:?}", order_tuple);
    info!("  R: 0x{}", hex::encode(r));
    info!("  VS: 0x{}", hex::encode(vs));
    info!("  Amount: {}", amount);
    info!("  Taker traits: {}", taker_traits);

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
            info!(" Order fill simulation successful!");
            info!(" Actual Making Amount: {} wei ({:.6} tokens)", 
                  actual_making_amount, actual_making_amount.as_u128() as f64 / 1e18);
            info!(" Actual Taking Amount: {} wei ({:.6} tokens)", 
                  actual_taking_amount, actual_taking_amount.as_u128() as f64 / 1e18);
            info!(" Order Hash: 0x{}", hex::encode(order_hash));
        }
        Err(e) => {
            error!(" Order fill simulation failed: {}", e);
            return Err(eyre::eyre!("Order fill failed: {}", e));
        }
    }

    Ok(())
}

/// Execute fillOrderArgs for orders with extension data
async fn execute_fill_order_args(
    client: &Arc<SignerClient>,
    router_contract: &Contract<SignerClient>,
    order_tuple: (U256, U256, U256, U256, U256, U256, U256, U256),
    r: [u8; 32],
    vs: [u8; 32],
    amount: U256,
    taker_traits: U256,
    extension_bytes: ethers::types::Bytes,
) -> Result<()> {
    info!("🔄 Executing fillOrderArgs with extension...");

    // Setup taker with required tokens and allowance

    // Convert packed addresses back to Address type for balance checks
    let mut taker_asset_bytes = [0u8; 32];
    order_tuple.4.to_big_endian(&mut taker_asset_bytes); // taker_asset is 5th element
    let mut addr_bytes = [0u8; 20];
    addr_bytes.copy_from_slice(&taker_asset_bytes[12..32]); // Take last 20 bytes
    let taker_asset_addr = Address::from(addr_bytes);

    let taker = client.address();

    // Check current balance
    let current_balance = get_token_balance(&client, taker_asset_addr, taker).await?;
    info!("Current taker asset balance: {} wei", current_balance);

    // We need at least 'amount' tokens to fill the order
    if current_balance < amount {
        info!("Insufficient balance. Need {} wei, have {} wei", amount, current_balance);
        let required_amount: U256 = amount * 2; // Get 2x what we need for safety
        info!("Setting {} tokens for taker", required_amount.as_u128() as f64 / 1e18);

        match set_token_balance_anvil(&client, taker_asset_addr, taker, required_amount).await {
            Ok(_) => info!("Successfully set token balance for taker"),
            Err(e) => {
                warn!("Failed to set token balance: {}", e);
                return Err(e);
            }
        }

        // Recheck balance
        let new_balance = get_token_balance(&client, taker_asset_addr, taker).await?;
        info!("Recheck Current taker asset balance: {} wei", new_balance);
    }

    let allowance_amount: U256 = amount * 10; // Approve 10x for safety
    match approve_token(&client, taker_asset_addr, router_contract.address(), allowance_amount).await {
        Ok(_) => info!("Successfully approved 1inch router"),
        Err(e) => {
            warn!("Failed to approve router: {}", e);
            return Err(e);
        }
    }


    let result = router_contract
        .method::<_, (U256, U256, [u8; 32])>(
            "fillOrderArgs",
            (
                order_tuple,
                r,
                vs,
                amount,
                taker_traits,
                extension_bytes
            ),
        )?
        .call()
        .await;

    match result {
        Ok((actual_making_amount, actual_taking_amount, order_hash)) => {
            info!("✅ fillOrderArgs successful!");
            info!("  Actual Making Amount: {} wei ({:.6} tokens)", actual_making_amount, actual_making_amount.as_u128() as f64 / 1e18);
            info!("  Actual Taking Amount: {} wei ({:.6} tokens)", actual_taking_amount, actual_taking_amount.as_u128() as f64 / 1e18);
            info!("  Order Hash: 0x{}", hex::encode(order_hash));
        }
        Err(e) => {
            error!("❌ fillOrderArgs simulation failed: {}", e);
            return Err(eyre::eyre!("Order fill failed: {}", e));
        }
    }

    Ok(())
}



async fn setup_oneinch_contract(client: &Arc<SignerClient>) -> Result<Contract<SignerClient>> {
    let router_abi: Abi = serde_json::from_str(ONEINCH_ROUTER_ABI)?;
    let router_address = Address::from_str("0x111111125421ca6dc452d289314280a0f8842a65")?;
    let contract = Contract::new(router_address, router_abi, client.clone());
    Ok(contract)
}

pub struct TakerTraitsOptions {
    pub maker_amount_flag: bool,
    pub unwrap_weth_flag: bool,
    pub use_permit2_flag: bool,
    pub args_has_target: bool,
    pub args_extension_length: u32,   // max 24 bits
    pub args_interaction_length: u32, // max 24 bits
    pub threshold: U256,              // max 185 bits
}

impl Default for TakerTraitsOptions {
    fn default() -> Self {
        Self {
            maker_amount_flag: false,
            unwrap_weth_flag: false,
            use_permit2_flag: false,
            args_has_target: false,
            args_extension_length: 0,
            args_interaction_length: 0,
            threshold: U256::zero(),
        }
    }
}

/// Build TakerTraits with comprehensive options
fn build_taker_traits_comprehensive(options: &TakerTraitsOptions) -> U256 {
    let mut traits = U256::zero();

    // Bit layout according to 1inch V6 (corrected based on working values):
    // 255: MAKER_AMOUNT_FLAG
    // 254: UNWRAP_WETH_FLAG
    // 253: USE_PERMIT2_FLAG
    // 251: ARGS_HAS_TARGET (corrected from 252 to 251)
    // 248-224: ARGS_EXTENSION_LENGTH (24 bits)
    // 224-200: ARGS_INTERACTION_LENGTH (24 bits)
    // 199-0: THRESHOLD (200 bits, but we use 185 for safety)

    if options.maker_amount_flag {
        traits |= U256::from(1) << 255;
    }

    if options.unwrap_weth_flag {
        traits |= U256::from(1) << 254;
    }

    if options.use_permit2_flag {
        traits |= U256::from(1) << 253;
    }

    if options.args_has_target {
        traits |= U256::from(1) << 251;  // Fixed: 251 instead of 252
    }

    // ARGS_EXTENSION_LENGTH (24 bits at position 248-224)
    let ext_len = (options.args_extension_length as u64) & 0xFFFFFF; // Mask to 24 bits
    traits |= U256::from(ext_len) << 224;

    // ARGS_INTERACTION_LENGTH (24 bits at position 223-200)
    let int_len = (options.args_interaction_length as u64) & 0xFFFFFF; // Mask to 24 bits
    traits |= U256::from(int_len) << 200;

    // THRESHOLD (185 bits at position 199-0)
    // Mask threshold to 185 bits for safety
    let threshold_mask = (U256::from(1) << 185) - 1;
    let masked_threshold = options.threshold & threshold_mask;
    traits |= masked_threshold;

    traits
}

/// Build TakerTraits with extension (simplified interface)
fn build_taker_traits_with_extension(ext: &[u8]) -> U256 {
    let options = TakerTraitsOptions {
        maker_amount_flag: false,
        unwrap_weth_flag: false,
        use_permit2_flag: false,
        args_has_target: false,  // Set target flag for extension orders
        args_extension_length:  184 as u32,
        args_interaction_length: 0,
        threshold: U256::zero(),
    };

    let built_value = build_taker_traits_comprehensive(&options);

    built_value
}


/// Build args parameter for fillOrderArgs according to 1inch V6 specification
/// Args format: [target_address?][extension_data][interaction_data?]
fn build_fillorder_args(
    extension_data: &[u8],
    target_address: Option<Address>,
    interaction_data: Option<&[u8]>
) -> Vec<u8> {
    let mut args = Vec::new();

    // Add target address if present (20 bytes)
    if let Some(target) = target_address {
        args.extend_from_slice(target.as_bytes());
    }

    // Add extension data
    args.extend_from_slice(extension_data);

    // Add interaction data if present
    if let Some(interaction) = interaction_data {
        args.extend_from_slice(interaction);
    }

    args
}

/// Build fillOrder args with API-KEY for authorization
fn build_fillorder_args_with_api_key(
    extension_data: &[u8],
    api_key: &[u8],
    target_address: Option<Address>,
    interaction_data: Option<&[u8]>
) -> Vec<u8> {
    let mut args = Vec::new();

    // Method 1: Prepend API-KEY to args
    args.extend_from_slice(api_key);

    // Add target address if present (20 bytes)
    if let Some(target) = target_address {
        args.extend_from_slice(target.as_bytes());
    }

    // Add extension data
    args.extend_from_slice(extension_data);

    // Add interaction data if present
    if let Some(interaction) = interaction_data {
        args.extend_from_slice(interaction);
    }

    args
}

/// Build TakerTraits with complete args specification
/// This calculates the correct bit layout for all args components
fn build_complete_taker_traits(
    extension_length: u64,
    interaction_length: u64,
    has_target: bool
) -> U256 {
    let mut taker_traits = U256::zero();

    // 1inch V6 TakerTraits complete bit layout:
    // Bits 224-247: ARGS_EXTENSION_LENGTH (24 bits)
    // Bits 248-255: ARGS_INTERACTION_LENGTH (8 bits)
    // Bit 255: ARGS_HAS_TARGET flag
    // Other bits: reserved/other flags

    // Set ARGS_EXTENSION_LENGTH at bits 224-247 (24 bits)
    let extension_bits = U256::from(extension_length & 0xFFFFFF);
    taker_traits |= extension_bits << 224;

    // Set ARGS_INTERACTION_LENGTH at bits 248-255 (8 bits)
    let interaction_bits = U256::from(interaction_length & 0xFF);
    taker_traits |= interaction_bits << 248;

    // Set ARGS_HAS_TARGET flag if needed
    if has_target {
        taker_traits |= U256::from(1) << 256; // Bit for target flag
    }

    taker_traits
}
