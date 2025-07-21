// Clean 1inch and PancakeSwap Demo
// Loads config from data.json and executes simulations

use eyre::{Result, eyre};
use tracing::{info, warn, error};

// Import modules
use crate::config::simple_config::{Config, OneInchOrder, MevConfig};
use crate::anvil_setup::setup_blockchain;
use crate::one_inch::fill_order;

mod config;
mod anvil_setup;
mod one_inch;
mod pancake_v2;

fn display_order_config(config: &OneInchOrder) {
    info!("📋 1inch Order Configuration:");
    info!("  Maker Asset: {}", config.maker_asset);
    info!("  Taker Asset: {}", config.taker_asset);
    info!("  Maker: {}", config.maker);
    info!("  Receiver: {}", config.receiver);
    info!("  Making Amount: {} tokens", config.format_making_amount());
    info!("  Taking Amount: {} tokens", config.format_taking_amount());
    info!("  Fill Amount: {} tokens", config.format_amount());
    info!("  Expected Amount Out: {} tokens", config.format_expected_amount_out());
    info!("  Expected Remaining Amount: {} tokens", config.format_expected_remaining_amount());
    info!("  Salt: {}", config.salt);
    info!("  Maker Traits: {}", config.maker_traits);
    info!("  Taker Traits: {}", config.taker_traits);

    // Display transaction info if available
    if let Some(tx_info) = &config.transaction_info {
        info!("🔍 Transaction Debug Info:");
        info!("  Hash: {}", tx_info.hash);
        info!("  Method: {}", tx_info.method);
        info!("  Complex: {}", tx_info.is_complex);
        info!("  Note: {}", tx_info.note);

        if tx_info.is_complex {
            warn!("⚠️  This is a complex transaction - simple simulation may not match exactly");
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    // Load MEV configuration
    let mev_config = match MevConfig::load_from_file("mev_data.json") {
        Ok(config) => {
            info!("✅ Loaded MEV configuration from mev_data.json");
            config
        }
        Err(e) => {
            error!("❌ Failed to load data_mev_v2.json: {}", e);
            return Err(e);
        }
    };

    if mev_config.one_inch_orders.is_empty() {
        return Err(eyre!("No MEV order configuration found"));
    }

    info!("📊 Found {} MEV order(s)", mev_config.one_inch_orders.len());
    info!("🎯 Block Number: {}", mev_config.block_number);

    // Create a dummy config for blockchain setup (using BSC mainnet)
    let dummy_config = Config {
        block: mev_config.block_number,
        swaps: vec![],
        orders: None,
    };

    // Setup blockchain connection
    let (_anvil, client) = setup_blockchain(&dummy_config).await?;

    // Execute swap simulation for all configs (commented out for now)
    // for (i, swap_config) in config.get_all_swaps().iter().enumerate() {
    //     info!("🚀 Testing Swap Config #{}: {}", i + 1, swap_config.pool_type.display_name());
    //     match execute_swap(swap_config, &client).await {
    //         Ok(_) => info!("✅ Swap #{} completed successfully", i + 1),
    //         Err(e) => error!("❌ Swap #{} failed: {}", i + 1, e),
    //     }
    //     info!("");
    // }

    // Execute MEV orders
    for (mev_order_index, mev_order) in mev_config.one_inch_orders.iter().enumerate() {
        info!("🚀 Testing MEV Order #{}: {}", mev_order_index + 1, mev_order.order.order_hash);
        match mev_order.to_standard_order(&mev_config.taker_traits) {
            Ok(order_config) => {
                display_order_config(&order_config);
                match fill_order(&order_config, &client).await {
                    Ok(_) => info!("✅ MEV Order #{} completed successfully", mev_order_index + 1),
                    Err(e) => error!("❌ MEV Order #{} failed: {}", mev_order_index + 1, e),
                }
            }
            Err(e) => error!("❌ Failed to convert MEV order to standard order: {}", e),
        }
        info!("");
    }

    info!("✅ MEV Demo completed successfully");
    Ok(())
}
