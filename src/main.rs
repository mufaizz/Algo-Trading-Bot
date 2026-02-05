use tokio::sync::mpsc;
use log::{info, error, warn};
use std::env;
use dotenv::dotenv;
use std::time::{Duration, Instant};

// Module declarations
mod binance_client;
mod model;
mod telegram;
mod news_filter; // Added Phase 2 module

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1. Initialize Environment
    dotenv().ok();
    env_logger::builder().filter_level(log::LevelFilter::Info).init();
    
    let token = env::var("TELEGRAM_TOKEN").expect("TELEGRAM_TOKEN not set");
    let chat_id = env::var("CHAT_ID").expect("CHAT_ID not set");
    
    info!("🚀 MUFAIZ IS GOAT");
    
    // 2. Create Channels
    let (tx_price, mut rx_price) = mpsc::channel::<f64>(100);
    
    // 3. Spawn Telegram Bot
    let bot = telegram::TelegramBot::new(token, chat_id);
    let bot_clone = bot.clone();
    
    // 4. Spawn Market Data Stream
    let market_handle = tokio::spawn(async move {
        if let Err(e) = binance_client::start_market_stream(tx_price).await {
            error!("CRITICAL: Market stream failed: {}", e);
        }
    });

    // 5. The Main Logic Loop
    let logic_handle = tokio::spawn(async move {
        let mut price_history: Vec<f64> = Vec::new();
        let oracle = news_filter::NewsOracle::new();
        let mut last_news_check = Instant::now();
        let mut is_danger_mode = false;
        
        while let Some(current_price) = rx_price.recv().await {
            // --- NEWS FILTER CHECK (Every 15 Minutes) ---
            if last_news_check.elapsed() > Duration::from_secs(900) {
                is_danger_mode = oracle.check_danger().await;
                last_news_check = Instant::now();
                if is_danger_mode {
                    warn!("⚠️ MARKET HALT: High Impact News Event Detected.");
                }
            }

            if is_danger_mode {
                continue; // Skip trading logic if news is active
            }

            // --- CORE LOGIC ---
            price_history.push(current_price);
            
            // Keep only last 60 ticks
            if price_history.len() > 60 {
                price_history.remove(0);
            }

            // Require data depth before calculating
            if price_history.len() >= 20 {
                let min = price_history.iter().fold(f64::INFINITY, |a, &b| a.min(b));
                let max = price_history.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
                let volatility = (max - min) / 2.0;

                let signal = model::QuantumSignal::run_monte_carlo(current_price, volatility);

                if signal.confidence > 90.0 {
                    let msg = format!(
                        "🚀 *TRADE SIGNAL*\nPair: EUR/USD\nAction: *{}*\nConf: {:.1}%\nPrice: {:.5}",
                        signal.direction, signal.confidence, current_price
                    );
                    bot_clone.send_signal(&msg).await;
                    tokio::time::sleep(Duration::from_secs(60)).await; // Prevent spamming same signal
                } else if signal.confidence > 75.0 && signal.confidence < 80.0 {
                     let msg = format!(
                        "⚠️ *PREPARE*\nPossible {} Setup\nConf: {:.1}%",
                        signal.direction, signal.confidence
                    );
                    bot_clone.send_signal(&msg).await;
                    tokio::time::sleep(Duration::from_secs(10)).await;
                }
            }
        }
    });

    let _ = tokio::join!(market_handle, logic_handle);
    Ok(())
}