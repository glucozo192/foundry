use eyre::{Result, eyre};
use tracing::{info,  error};

// Import modules
use crate::config::simple_config::{Config, MevConfig};
use crate::anvil_setup::{setup_blockchain};
use crate::one_inch::{fill_order, fill_order_args};
use std::sync::Arc;

mod config;
mod anvil_setup;
mod one_inch;
mod pancake_v2;
mod uniswap_v3;


#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    // Load MEV configuration
    let mev_config = match MevConfig::load_from_file("mev_data_updated.json") {
        Ok(config) => {
            info!("Loaded MEV configuration from mev_data_updated.json");
            config
        }
        Err(e) => {
            error!("Failed to load data_mev_v2.json: {}", e);
                    return Err(e);
                }
            };

            if mev_config.one_inch_orders.is_empty() {
                return Err(eyre!("No MEV order configuration found"));
            }

    info!("Found {} MEV order(s)", mev_config.one_inch_orders.len());
    info!("Block Number: {}", mev_config.block_number);

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
    //         Ok(_) => info!("Swap #{} completed successfully", i + 1),
    //         Err(e) => error!("Swap #{} failed: {}", i + 1, e),
    //     }
    //     info!("");
    // }

    // Execute MEV orders
    for (mev_order_index, mev_order) in mev_config.one_inch_orders.iter().enumerate() {
        match mev_order.to_standard_order(&mev_config.taker_traits) {
            Ok(order_config) => {
                match fill_order_args(&order_config, &mev_order.order.extension, &client).await {
                    Ok(_) => info!("MEV Order #{} completed successfully", mev_order_index + 1),
                    Err(e) => error!("MEV Order #{} failed: {}", mev_order_index + 1, e),
                }
            }
            Err(e) => error!("Failed to convert MEV order to standard order: {}", e),
            }
            info!("");
    }

    info!("MEV Demo completed successfully");
    Ok(())
}
