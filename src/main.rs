use tokio::sync::mpsc;
use log::{info, error};
use std::env;
use dotenv::dotenv;

// Module declarations
mod binance_client;
mod model;
mod telegram;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1. Initialize Environment
    dotenv().ok();
    env_logger::builder().filter_level(log::LevelFilter::Info).init();
    
    let token = env::var("TELEGRAM_TOKEN").expect("TELEGRAM_TOKEN not set");
    let chat_id = env::var("CHAT_ID").expect("CHAT_ID not set");
    
    info!("🚀 QUANTUM ENGINE INITIALIZED");
    
    // 2. Create Channels (The "Nervous System")
    // tx_price: Sends price from Binance -> Math Engine
    let (tx_price, mut rx_price) = mpsc::channel::<f64>(100);
    
    // 3. Spawn Telegram Bot
    let bot = telegram::TelegramBot::new(token, chat_id);
    let bot_clone = bot.clone();
    
    // 4. Spawn Market Data Stream (The "Eyes")
    let market_handle = tokio::spawn(async move {
        if let Err(e) = binance_client::start_market_stream(tx_price).await {
            error!("CRITICAL: Market stream failed: {}", e);
        }
    });

    // 5. The Main Logic Loop (The "Brain")
    let logic_handle = tokio::spawn(async move {
        let mut price_history: Vec<f64> = Vec::new();
        
        while let Some(current_price) = rx_price.recv().await {
            price_history.push(current_price);
            
            // Keep only last 60 ticks (approx 1 min of data for volatility calc)
            if price_history.len() > 60 {
                price_history.remove(0);
            }

            // Require at least 20 ticks to start calculating volatility
            if price_history.len() >= 20 {
                // Calculate simple volatility (standard deviation proxy)
                let min = price_history.iter().fold(f64::INFINITY, |a, &b| a.min(b));
                let max = price_history.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
                let volatility = (max - min) / 2.0;

                // Run Quantum Math
                let signal = model::QuantumSignal::run_monte_carlo(current_price, volatility);

                // DECISION MATRIX
                if signal.confidence > 90.0 {
                    let msg = format!(
                        "🚀 *TRADE SIGNAL*\nPair: EUR/USD (Proxy)\nAction: *{}*\nConf: {:.1}%\nPrice: {:.5}",
                        signal.direction, signal.confidence, current_price
                    );
                    bot_clone.send_signal(&msg).await;
                    
                    // Anti-Spam: Sleep logic would go here in full version
                } else if signal.confidence > 75.0 && signal.confidence < 80.0 {
                     // Warning Zone
                     let msg = format!(
                        "⚠️ *PREPARE*\nPossible {} Setup\nConf: {:.1}%",
                        signal.direction, signal.confidence
                    );
                    bot_clone.send_signal(&msg).await;
                }
            }
        }
    });

    // Keep the main process alive
    let _ = tokio::join!(market_handle, logic_handle);
    Ok(())
}