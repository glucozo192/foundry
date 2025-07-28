// Simple configuration for Uniswap V2 swaps
use serde::{Deserialize, Serialize};
use std::fs;
use eyre::Result;
use ethers::types::U256;

/// Pool type enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PoolType {
    #[serde(rename = "Univ2")]
    UniswapV2,
    #[serde(rename = "Univ3")]
    UniswapV3,
    #[serde(rename = "PancakeV2")]
    PancakeSwapV2,
    #[serde(rename = "PancakeV3")]
    PancakeSwapV3,
    #[serde(rename = "OneInch")]
    OneInch,
}

/// Transaction info for debugging
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionInfo {
    pub hash: String,
    pub note: String,
    pub method: String,
    pub is_complex: bool,
}

/// Simple swap configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapConfig {
    pub token1: String,           // Token in address
    pub token2: String,           // Token out address
    pub amount_in: String,        // Amount to swap in
    pub pool_address: String,     // Pool address
    pub expected_amount_out: String, // Expected output amount
    pub fee: u32,                 // Fee in basis points
    #[serde(rename = "type")]
    pub pool_type: PoolType,      // Pool type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transaction_info: Option<TransactionInfo>, // Debug info
}

/// 1inch Order configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OneInchOrder {
    pub salt: String,                    // Order salt
    pub maker: String,                   // Maker (packed as uint256)
    pub receiver: String,                // Receiver (packed as uint256)
    pub maker_asset: String,             // Maker asset (packed as uint256)
    pub taker_asset: String,             // Taker asset (packed as uint256)
    pub making_amount: String,           // Amount maker is offering
    pub taking_amount: String,           // Amount maker wants to receive
    pub maker_traits: String,            // Maker traits
    pub r: String,                       // Signature r component
    pub vs: String,                      // Signature vs component
    pub amount: String,                  // Amount to fill
    pub taker_traits: String,            // Taker traits (can be 0)
    pub expected_amount_out: String,     // Expected amount out from fill
    pub expected_remaining_amount: String, // Expected remaining amount after fill
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transaction_info: Option<TransactionInfo>, // Debug info
}

/// Root configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub block: u64,                // Block number when transaction occurred
    pub swaps: Vec<SwapConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub orders: Option<Vec<OneInchOrder>>, // 1inch orders
}

impl Config {
    /// Load configuration from JSON file
    pub fn load_from_file(path: &str) -> Result<Self> {
        let content = fs::read_to_string(path)?;
        let config: Config = serde_json::from_str(&content)?;
        Ok(config)
    }

    /// Get first swap config
    pub fn get_default_swap(&self) -> Option<&SwapConfig> {
        self.swaps.first()
    }

    /// Get swap config by index
    pub fn get_swap(&self, index: usize) -> Option<&SwapConfig> {
        self.swaps.get(index)
    }

    /// Get all swap configs
    pub fn get_all_swaps(&self) -> &[SwapConfig] {
        &self.swaps
    }

    /// Count of swap configs
    pub fn swap_count(&self) -> usize {
        self.swaps.len()
    }

    /// Get first order config
    pub fn get_default_order(&self) -> Option<&OneInchOrder> {
        self.orders.as_ref()?.first()
    }

    /// Get order config by index
    pub fn get_order(&self, index: usize) -> Option<&OneInchOrder> {
        self.orders.as_ref()?.get(index)
    }

    /// Get all order configs
    pub fn get_all_orders(&self) -> &[OneInchOrder] {
        self.orders.as_ref().map(|o| o.as_slice()).unwrap_or(&[])
    }

    /// Count of order configs
    pub fn order_count(&self) -> usize {
        self.orders.as_ref().map(|o| o.len()).unwrap_or(0)
    }

    /// Get block number for forking (block - 1 for pre-transaction state)
    pub fn get_fork_block(&self) -> u64 {
        if self.block > 0 {
            self.block - 1
        } else {
            self.block
        }
    }

    /// Get transaction block number
    pub fn get_transaction_block(&self) -> u64 {
        self.block
    }
}

impl PoolType {
    /// Get router address for the pool type
    pub fn get_router_address(&self) -> &'static str {
        match self {
            PoolType::UniswapV2 | PoolType::PancakeSwapV2 => "0x10ED43C718714eb63d5aA57B78B54704E256024E", // PancakeSwap V2 Router
            PoolType::UniswapV3 | PoolType::PancakeSwapV3 => "0x1b81D678ffb9C0263b24A97847620C99d213eB14", // PancakeSwap V3 SwapRouter
            PoolType::OneInch => "0x111111125421ca6dc452d289314280a0f8842a65", // 1inch Aggregation Router V5
        }
    }

    /// Get display name
    pub fn display_name(&self) -> &'static str {
        match self {
            PoolType::UniswapV2 => "Uniswap V2",
            PoolType::UniswapV3 => "Uniswap V3",
            PoolType::PancakeSwapV2 => "PancakeSwap V2",
            PoolType::PancakeSwapV3 => "PancakeSwap V3",
            PoolType::OneInch => "1inch",
        }
    }

    /// Check if this is a V3 pool (concentrated liquidity)
    pub fn is_v3(&self) -> bool {
        matches!(self, PoolType::UniswapV3 | PoolType::PancakeSwapV3)
    }
}

impl SwapConfig {
    /// Format amount in for display
    pub fn format_amount_in(&self) -> String {
        let amount = self.amount_in.parse::<f64>().unwrap_or(0.0) / 1e18;
        format!("{:.6}", amount)
    }

    /// Format expected amount out for display
    pub fn format_expected_out(&self) -> String {
        let amount = self.expected_amount_out.parse::<f64>().unwrap_or(0.0) / 1e18;
        format!("{:.6}", amount)
    }

    /// Get swap path
    pub fn get_path(&self) -> Vec<String> {
        vec![self.token1.clone(), self.token2.clone()]
    }

    /// Get router address based on pool type
    pub fn get_router_address(&self) -> &'static str {
        self.pool_type.get_router_address()
    }

    /// Compare actual result with expected
    pub fn compare_result(&self, actual_amount_out: &str) -> ComparisonResult {
        let expected = self.expected_amount_out.parse::<f64>().unwrap_or(0.0);
        let actual = actual_amount_out.parse::<f64>().unwrap_or(0.0);
        let difference_pct = if expected > 0.0 {
            ((actual - expected) / expected * 100.0).abs()
        } else {
            0.0
        };
        
        ComparisonResult {
            expected,
            actual,
            difference_pct,
            is_within_tolerance: difference_pct < 1.0, // 1% tolerance
        }
    }
}

/// Comparison result
#[derive(Debug, Clone)]
pub struct ComparisonResult {
    pub expected: f64,
    pub actual: f64,
    pub difference_pct: f64,
    pub is_within_tolerance: bool,
}

// MEV-specific structures
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct MevConfig {
    pub address: String,
    pub protocol: String,
    pub token0: String,
    pub token1: String,
    pub direct: String,
    pub block_number: u64,
    pub taker_traits: String,
    pub one_inch_orders: Vec<MevOneInchOrder>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct MevOneInchOrder {
    pub amount_in: String,   // hex format
    pub amount_out: String,  // hex format
    pub order: MevOrder,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct MevOrder {
    pub order_hash: String,
    pub salt: String,
    pub maker: String,
    pub receiver: String,
    pub maker_asset: String,
    pub taker_asset: String,
    pub making_amount: String,
    pub remaining_making_amount: String,
    pub taking_amount: String,
    pub maker_traits: String,  // Changed to hex string
    pub extension: String,     // Extension field
    pub signature: String,     // Full signature hex string
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct MevMakerTraits {
    pub no_partial_fills: bool,
    pub allow_multiple_fills: bool,
}

impl MevConfig {
    pub fn load_from_file(path: &str) -> Result<Self> {
        let content = fs::read_to_string(path)?;
        let config: MevConfig = serde_json::from_str(&content)?;
        Ok(config)
    }
}

impl MevOneInchOrder {
    /// Convert hex string to decimal string
    fn hex_to_decimal(hex_str: &str) -> Result<String> {
        // If it's already decimal, return as is
        if !hex_str.starts_with("0x") {
            return Ok(hex_str.to_string());
        }

        let hex_clean = hex_str.trim_start_matches("0x");
        // Use U256 for large numbers
        let decimal = U256::from_str_radix(hex_clean, 16)?;
        Ok(decimal.to_string())
    }

    /// Convert MEV order to standard OneInchOrder format
    pub fn to_standard_order(&self, taker_traits: &str) -> Result<OneInchOrder> {
        // Convert hex amounts to decimal
        let making_amount = Self::hex_to_decimal(&self.order.making_amount)?;
        let taking_amount = Self::hex_to_decimal(&self.order.taking_amount)?;
        let amount = Self::hex_to_decimal(&self.amount_in)?;
        let expected_amount_out = Self::hex_to_decimal(&self.amount_out)?;

        // Convert addresses to packed format (simplified - you may need more complex logic)
        let maker_asset = Self::address_to_packed(&self.order.maker_asset)?;
        let taker_asset = Self::address_to_packed(&self.order.taker_asset)?;
        let maker = Self::address_to_packed(&self.order.maker)?;
        let receiver = if self.order.receiver == "0x0000000000000000000000000000000000000000" {
            "0".to_string()
        } else {
            Self::address_to_packed(&self.order.receiver)?
        };

        Ok(OneInchOrder {
            salt: Self::hex_to_decimal(&self.order.salt)?,
            maker,
            receiver,
            maker_asset,
            taker_asset,
            making_amount,
            taking_amount,
            maker_traits: Self::hex_to_decimal(&self.order.maker_traits)?,
            r: Self::extract_r_from_signature(&self.order.signature)?,
            vs: Self::extract_vs_from_signature(&self.order.signature)?,
            amount,
            taker_traits: Self::hex_to_decimal(taker_traits)?,
            expected_amount_out,
            expected_remaining_amount: Self::hex_to_decimal(&self.order.remaining_making_amount)?,
            transaction_info: Some(TransactionInfo {
                hash: self.order.order_hash.clone(),
                method: "MEV Order".to_string(),
                is_complex: false,
                note: "Converted from MEV data".to_string(),
            }),
        })
    }

    /// Convert address to packed uint256 format (simplified)
    fn address_to_packed(address: &str) -> Result<String> {
        let addr_clean = address.trim_start_matches("0x");
        let addr_bytes = hex::decode(addr_clean)?;

        // Pad to 32 bytes and convert to decimal using U256
        use ethers::types::U256;
        let mut padded = vec![0u8; 32];
        padded[12..32].copy_from_slice(&addr_bytes);

        let result = U256::from_big_endian(&padded);
        Ok(result.to_string())
    }

    /// Extract r component from signature (first 32 bytes)
    fn extract_r_from_signature(signature: &str) -> Result<String> {
        let sig_clean = signature.trim_start_matches("0x");
        if sig_clean.len() != 130 { // 65 bytes * 2 hex chars
            return Err(eyre::eyre!("Invalid signature length"));
        }

        let r_hex = &sig_clean[0..64]; // First 32 bytes
        Ok(format!("0x{}", r_hex))
    }

    /// Extract vs component from signature (last 33 bytes combined)
    fn extract_vs_from_signature(signature: &str) -> Result<String> {
        let sig_clean = signature.trim_start_matches("0x");
        if sig_clean.len() != 130 { // 65 bytes * 2 hex chars
            return Err(eyre::eyre!("Invalid signature length"));
        }

        let s_hex = &sig_clean[64..128]; // Next 32 bytes (s)
        let v_hex = &sig_clean[128..130]; // Last byte (v)

        // Combine v and s into vs format for 1inch
        // vs = (v - 27) << 255 | s
        let v = u8::from_str_radix(v_hex, 16)?;
        let s = U256::from_str_radix(s_hex, 16)?;

        let v_adjusted = if v >= 27 { v - 27 } else { v };
        let vs = if v_adjusted == 1 {
            s | (U256::one() << 255)
        } else {
            s
        };

        Ok(format!("0x{:064x}", vs))
    }
}