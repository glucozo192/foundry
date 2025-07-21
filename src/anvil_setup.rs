use std::sync::Arc;
use std::str::FromStr;
use ethers::{
    providers::{Provider, Http, Middleware},
    signers::{LocalWallet, Signer},
    middleware::SignerMiddleware,
    types::{Address, U256},
    contract::Contract,
    abi::Abi,
    utils::{Anvil, AnvilInstance, keccak256, hex},
};
use eyre::Result;
use tracing::{info, error};
use crate::config::simple_config::Config;

// Type aliases
pub type SignerClient = SignerMiddleware<Provider<Http>, LocalWallet>;

// Constants
const RPC_URL: &str = "https://api.zan.top/node/v1/bsc/mainnet/2d661fce966a44139a2d4c61d373851f";

const ERC20_ABI: &str = r#"[
    {
        "inputs": [{"internalType": "address", "name": "account", "type": "address"}],
        "name": "balanceOf",
        "outputs": [{"internalType": "uint256", "name": "", "type": "uint256"}],
        "stateMutability": "view",
        "type": "function"
    },
    {
        "inputs": [
            {"internalType": "address", "name": "spender", "type": "address"},
            {"internalType": "uint256", "name": "amount", "type": "uint256"}
        ],
        "name": "approve",
        "outputs": [{"internalType": "bool", "name": "", "type": "bool"}],
        "stateMutability": "nonpayable",
        "type": "function"
    }
]"#;

pub async fn setup_blockchain(config: &Config) -> Result<(AnvilInstance, Arc<SignerClient>)> {
    let fork_block = config.get_fork_block();

    // Start Anvil fork
    let anvil = Anvil::new()
        .fork(RPC_URL)
        .fork_block_number(fork_block)
        .spawn();

    // Setup provider
    let provider = Provider::<Http>::try_from(anvil.endpoint())?;

    // Setup wallet
    let wallet: LocalWallet = anvil.keys()[0].clone().into();
    let wallet = wallet.with_chain_id(anvil.chain_id());

    // Create client
    let client = Arc::new(SignerMiddleware::new(provider, wallet));

    Ok((anvil, client))
}

pub async fn get_token_balance(
    client: &Arc<SignerClient>,
    token_address: Address,
    account: Address,
) -> Result<U256> {
    let token_abi: ethers::abi::Abi = serde_json::from_str(ERC20_ABI)?;
    let token_contract = Contract::new(token_address, token_abi, client.clone());

    let balance: U256 = token_contract
        .method("balanceOf", account)?
        .call()
        .await?;

    Ok(balance)
}

pub async fn approve_token(
    client: &Arc<SignerClient>,
    token_address: Address,
    spender: Address,
    amount: U256,
) -> Result<()> {
    let token_abi: ethers::abi::Abi = serde_json::from_str(ERC20_ABI)?;
    let token_contract = Contract::new(token_address, token_abi, client.clone());

    let _tx = token_contract
        .method::<_, bool>("approve", (spender, amount))?
        .send()
        .await?
        .await?;

    info!("Token approval successful");
    Ok(())
}

pub async fn set_token_balance_anvil(
    client: &Arc<SignerClient>,
    token_address: Address,
    account: Address,
    amount: U256,
) -> Result<()> {
    // Try more storage slots for different token implementations
    for slot in 0..10 {
        match set_erc20_balance(client, token_address, account, amount, slot).await {
            Ok(_) => {
                // Verify the balance was set
                let new_balance = get_token_balance(client, token_address, account).await?;
                
                if new_balance >= amount {
                    return Ok(());
                }
            }
            Err(e) => {
                error!("⚠️  Slot {} failed: {}", slot, e);
                continue;
            }
        }
    }
    
    Err(eyre::eyre!("Failed to set token balance using any common slot"))
}

async fn set_erc20_balance(
    client: &Arc<SignerClient>,
    token_address: Address,
    account: Address,
    amount: U256,
    balance_slot: u8,
) -> Result<()> {
    use ethers::utils::keccak256;
    
    // Calculate storage slot for balance: keccak256(account + balance_slot)
    let mut key = [0u8; 64];
    key[12..32].copy_from_slice(account.as_bytes()); // account (20 bytes, right-padded to 32)
    key[63] = balance_slot; // slot number in last byte
    
    let storage_key = keccak256(&key);
    
    // Convert amount to 32-byte array
    let mut value = [0u8; 32];
    amount.to_big_endian(&mut value);
    
    // Use Anvil's setStorageAt RPC call
    let provider = client.provider();
    let _result: bool = provider
        .request("anvil_setStorageAt", [
            format!("0x{}", hex::encode(token_address.as_bytes())),
            format!("0x{}", hex::encode(storage_key)),
            format!("0x{}", hex::encode(value)),
        ])
        .await?;
    
    Ok(())
}

/// Replace token contract with mock that returns unlimited balance
async fn replace_with_mock_token(
    client: &Arc<SignerClient>,
    token_address: Address,
    _account: Address,
    _amount: U256,
) -> Result<()> {
    info!("Replacing token with mock contract...");

    // Complete mock ERC20 with balanceOf, approve, allowance functions
    let mock_bytecode = "0x608060405234801561001057600080fd5b50600436106100575760003560e01c806306fdde031461005c578063095ea7b31461007a57806318160ddd1461009a57806323b872dd146100a2578063313ce567146100b557806370a08231146100c4578063a9059cbb146100d7578063dd62ed3e146100ea575b600080fd5b6100646100fd565b6040516100719190610139565b60405180910390f35b61008d610088366004610194565b610134565b60405161007191906101be565b61008d6101c9565b61008d6100b03660046101c9565b6101ce565b61008d6101d3565b61008d6100d23660046101d8565b6101d8565b61008d6100e5366004610194565b6101dd565b61008d6100f83660046101fa565b6101e2565b60408051808201909152600881526714d85b9d0813919560c21b602082015290565b600060019392505050565b600019919050565b600060019392505050565b600090565b600019919050565b600060019392505050565b600019919050565b600060208083528351808285015260005b8181101561016657858101830151858201604001528201610149565b506000604082860101526040601f19601f8301168501019250505092915050565b80356001600160a01b038116811461018f57600080fd5b919050565b600080604083850312156101a757600080fd5b6101b083610178565b946020939093013593505050565b901515815260200190565b6000602082840312156101db57600080fd5b6101e482610178565b9392505050565b600080604083850312156101fe57600080fd5b61020783610178565b915061021560208401610178565b9050925092905056fea2646970667358221220abcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdef64736f6c63430008110033";

    client.provider().request::<_, ()>(
        "anvil_setCode",
        (token_address, mock_bytecode)
    ).await?;

    info!("✅ Mock token deployed");
    Ok(())
}

/// Calculate balance storage slot for mapping(address => uint256) at given slot
fn calculate_balance_slot(account: Address, slot: u64) -> [u8; 32] {
    let mut data = [0u8; 64];
    // address (left-padded to 32 bytes)
    data[12..32].copy_from_slice(account.as_bytes());
    // slot (right-padded to 32 bytes)
    data[63] = slot as u8;
    keccak256(&data)
}

/// Calculate balance storage slot with reverse order
fn calculate_balance_slot_reverse(account: Address, slot: u64) -> [u8; 32] {
    let mut data = [0u8; 64];
    // slot first
    data[31] = slot as u8;
    // address second
    data[32..52].copy_from_slice(account.as_bytes());
    keccak256(&data)
}

/// Transfer tokens from whale account
async fn transfer_from_whale(
    client: &Arc<SignerClient>,
    token_address: Address,
    to_account: Address,
    amount: U256,
) -> Result<()> {
    info!("🐋 Attempting whale transfer for token: {}", token_address);

    // Common whale addresses for different tokens
    let mut whale_addresses = vec![
        "0x8894e0a0c962cb723c1976a4421c95949be2d4e3", // Binance hot wallet
        "0xf977814e90da44bfa03b6295a0616a897441acec", // Binance 8
        "0x28c6c06298d514db089934071355e5743bf21d60", // Binance 14
        "0x21a31ee1afc51d94c2efccaa2092ad1028285549", // Binance 15
        "0x47ac0fb4f2d84898e4d9e7b4dab3c24507a6d503", // Binance 16
    ];

    // Add HIGH token specific addresses
    let high_token = Address::from_str("0x5f4bde007dc06b867f86ebfe4802e34a1ffeed63")?;
    if token_address == high_token {
        whale_addresses.extend(vec![
            "0x0ed943ce24baebf257488771759f9bf482c39706", // HIGH deployer
            "0x1f3fe24342bbc6725cf1ab342059b645dd4eb743", // HIGH whale 1
            "0xd9f4fbfbb394878fd59391abed6f746991746f6d", // HIGH whale 2 (maker)
        ]);
    }

    const ERC20_ABI: &str = r#"[
        {
            "inputs": [
                {"name": "to", "type": "address"},
                {"name": "amount", "type": "uint256"}
            ],
            "name": "transfer",
            "outputs": [{"name": "", "type": "bool"}],
            "stateMutability": "nonpayable",
            "type": "function"
        },
        {
            "inputs": [{"name": "account", "type": "address"}],
            "name": "balanceOf",
            "outputs": [{"name": "", "type": "uint256"}],
            "stateMutability": "view",
            "type": "function"
        }
    ]"#;

    let token_abi: Abi = serde_json::from_str(ERC20_ABI)?;
    let token_contract = Contract::new(token_address, token_abi, client.clone());

    // Try each whale address
    for whale_addr in whale_addresses {
        let whale_address = Address::from_str(whale_addr)?;

        // Check whale balance
        let whale_balance: U256 = token_contract
            .method("balanceOf", whale_address)?
            .call()
            .await?;

        if whale_balance >= amount {
            info!("🐋 Found whale {} with sufficient balance: {} tokens",
                  whale_addr, whale_balance.as_u128() as f64 / 1e18);

            // Impersonate whale account
            let impersonate_result = client.provider().request::<_, ()>(
                "anvil_impersonateAccount",
                [whale_address]
            ).await;

            if impersonate_result.is_ok() {
                info!("✅ Successfully impersonated whale account");

                // Transfer tokens
                let transfer_call = token_contract
                    .method::<_, bool>("transfer", (to_account, amount))?
                    .from(whale_address);

                let transfer_result = transfer_call.send().await;

                match transfer_result {
                    Ok(_) => {
                        info!("✅ Successfully transferred {} tokens from whale",
                              amount.as_u128() as f64 / 1e18);
                        return Ok(());
                    }
                    Err(e) => {
                        info!("⚠️  Transfer failed from whale {}: {}", whale_addr, e);
                    }
                }
            }
        }
    }

    Err(eyre::eyre!("Failed to transfer from any whale account"))
}

/// Use Foundry's deal cheatcode to set token balance
async fn deal_tokens(
    client: &Arc<SignerClient>,
    token_address: Address,
    account: Address,
    amount: U256,
) -> Result<()> {
    info!("🎯 Using Foundry deal cheatcode for token: {}", token_address);

    // Foundry deal cheatcode: deal(address token, address to, uint256 give)
    const DEAL_SIGNATURE: &str = "deal(address,address,uint256)";

    // Calculate function selector
    let selector = &keccak256(DEAL_SIGNATURE.as_bytes())[0..4];

    // Encode parameters: token, account, amount
    let mut calldata = Vec::new();
    calldata.extend_from_slice(selector);

    // Encode token address (32 bytes)
    let mut token_bytes = [0u8; 32];
    token_bytes[12..32].copy_from_slice(token_address.as_bytes());
    calldata.extend_from_slice(&token_bytes);

    // Encode account address (32 bytes)
    let mut account_bytes = [0u8; 32];
    account_bytes[12..32].copy_from_slice(account.as_bytes());
    calldata.extend_from_slice(&account_bytes);

    // Encode amount (32 bytes)
    let mut amount_bytes = [0u8; 32];
    amount.to_big_endian(&mut amount_bytes);
    calldata.extend_from_slice(&amount_bytes);

    // Call deal cheatcode (address 0x7109709ECfa91a80626fF3989D68f67F5b1DD12D)
    let cheatcode_address = Address::from_str("0x7109709ECfa91a80626fF3989D68f67F5b1DD12D")?;

    let tx = client.send_transaction(
        ethers::types::TransactionRequest::new()
            .to(cheatcode_address)
            .data(calldata),
        None
    ).await?;

    tx.await?;

    // Verify balance was set
    let new_balance = get_token_balance(client, token_address, account).await?;
    if new_balance >= amount {
        info!("✅ Deal successful! Balance: {} tokens", new_balance.as_u128() as f64 / 1e18);
        Ok(())
    } else {
        Err(eyre::eyre!("Deal failed - balance not set correctly"))
    }
}

/// Fund account via DEX swap (most reliable for test environment)
async fn fund_via_dex_swap(
    client: &Arc<SignerClient>,
    token_address: Address,
    account: Address,
    amount: U256,
) -> Result<()> {
    info!("💰 Funding via DEX swap for token: {}", token_address);

    // Step 1: Give account massive ETH balance
    let eth_amount = U256::from(1000) * U256::exp10(18); // 1000 ETH

    client.provider().request::<_, ()>(
        "anvil_setBalance",
        (account, format!("0x{:x}", eth_amount))
    ).await?;

    info!("✅ Set ETH balance: {} ETH", eth_amount.as_u128() as f64 / 1e18);

    // Step 2: Use PancakeSwap to swap ETH -> WBNB -> Target Token
    let pancake_router = Address::from_str("0x10ED43C718714eb63d5aA57B78B54704E256024E")?; // PancakeSwap V2 Router
    let wbnb = Address::from_str("0xbb4CdB9CBd36B01bD1cBaEBF2De08d9173bc095c")?;

    // Router ABI for swapExactETHForTokens
    const ROUTER_ABI: &str = r#"[
        {
            "inputs": [
                {"name": "amountOutMin", "type": "uint256"},
                {"name": "path", "type": "address[]"},
                {"name": "to", "type": "address"},
                {"name": "deadline", "type": "uint256"}
            ],
            "name": "swapExactETHForTokens",
            "outputs": [{"name": "amounts", "type": "uint256[]"}],
            "stateMutability": "payable",
            "type": "function"
        }
    ]"#;

    let router_abi: Abi = serde_json::from_str(ROUTER_ABI)?;
    let router_contract = Contract::new(pancake_router, router_abi, client.clone());

    // Create swap path: ETH -> WBNB -> Target Token
    let path = vec![wbnb, token_address];
    let deadline = U256::from(chrono::Utc::now().timestamp() + 3600); // 1 hour from now
    let swap_amount = U256::from(100) * U256::exp10(18); // Use 100 ETH for swap

    info!("🔄 Swapping {} ETH for {} tokens via PancakeSwap",
          swap_amount.as_u128() as f64 / 1e18, token_address);

    let tx_call = router_contract
        .method::<_, Vec<U256>>("swapExactETHForTokens", (
            U256::zero(), // amountOutMin = 0 (accept any amount)
            path,
            account,
            deadline,
        ))?
        .value(swap_amount)
        .from(account);

    let pending_tx = tx_call.send().await?;
    let receipt = pending_tx.await?;
    if let Some(receipt) = receipt {
        info!("✅ Swap transaction successful: {:?}", receipt.transaction_hash);
    } else {
        info!("✅ Swap transaction sent successfully");
    }

    // Verify balance
    let new_balance = get_token_balance(client, token_address, account).await?;
    if new_balance > U256::zero() {
        info!("✅ DEX swap successful! New balance: {} tokens",
              new_balance.as_u128() as f64 / 1e18);
        Ok(())
    } else {
        Err(eyre::eyre!("DEX swap failed - no tokens received"))
    }
}
