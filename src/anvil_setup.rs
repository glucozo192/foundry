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