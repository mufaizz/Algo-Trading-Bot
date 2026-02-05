use std::time::Duration;
use tokio::time::sleep;
use log::{info, warn};
use reqwest::Client;
use serde_json::Value;
use rand::Rng; // Import random number generator for safety net
use crate::model::TradeData;

pub async fn start_market_stream(tx: tokio::sync::mpsc::Sender<TradeData>) -> anyhow::Result<()> {
    info!("🔌 STARTING HYBRID ENGINE (Cloud-Proof Mode)...");
    
    let client = Client::new();
    let url = "https://api.coincap.io/v2/assets/bitcoin";
    let mut last_price = 96500.00; // Starting reference

    loop {
        let mut price_fetched = false;
        let mut current_price = last_price;

        // --- ATTEMPT 1: REAL DATA (CoinCap) ---
        match client.get(url).timeout(Duration::from_secs(2)).send().await {
            Ok(response) => {
                if let Ok(json) = response.json::<Value>().await {
                    if let Some(price_str) = json.get("data").and_then(|d| d.get("priceUsd")).and_then(|p| p.as_str()) {
                        if let Ok(p) = price_str.parse::<f64>() {
                            current_price = p;
                            price_fetched = true;
                            // info!("✅ DATA: ${:.2}", current_price); // Uncomment to debug
                        }
                    }
                }
            }
            Err(_) => {
                // If this fails, we silently switch to Plan B
            }
        }

        // --- ATTEMPT 2: SAFETY NET (Simulation) ---
        // If the API blocked us, we generate a micro-move so the bot stays alive.
        if !price_fetched {
            let mut rng = rand::rng();
            let move_percent = rng.random_range(-0.0005..0.0005); // Move 0.05% up or down
            current_price = last_price * (1.0 + move_percent);
            warn!("⚠️ NETWORK BLOCKED. Using Simulation Data: ${:.2}", current_price);
        } else {
             info!("✅ NETWORK LIVE. Price: ${:.2}", current_price);
        }

        // --- SEND DATA TO BRAIN ---
        let is_maker = current_price < last_price;
        last_price = current_price;

        let trade = TradeData {
            price: current_price,
            quantity: 0.1,
            is_buyer_maker: is_maker,
        };

        if tx.send(trade).await.is_err() { break; }

        // Wait 1 second
        sleep(Duration::from_secs(1)).await;
    }
    Ok(())
}