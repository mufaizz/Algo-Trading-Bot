use tokio::sync::mpsc;
use log::{info, error, warn};
use std::env;
use dotenv::dotenv;
use std::time::{Duration, Instant};

// --- MODULE DECLARATIONS ---
mod binance_client;
mod model;
mod telegram;
mod news_filter;
mod logger;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1. Initialize Environment
    dotenv().ok();
    // Force logs to show if RUST_LOG is not set
    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "info");
    }
    env_logger::init();
    
    // Load Credentials
    let token = env::var("TELEGRAM_TOKEN").expect("TELEGRAM_TOKEN not set");
    let chat_id = env::var("CHAT_ID").expect("CHAT_ID not set");
    
    info!("🚀 MUFAIZ THE GOAT - QUANTUM SYSTEM ONLINE");
    
    // 2. Create Data Channels
    let (tx_data, mut rx_data) = mpsc::channel::<model::TradeData>(100);
    
    // 3. Spawn Telegram Bot
    let bot = telegram::TelegramBot::new(token, chat_id);
    let bot_clone = bot.clone();
    
    // 4. Spawn Market Data Stream
    let market_handle = tokio::spawn(async move {
        if let Err(e) = binance_client::start_market_stream(tx_data).await {
            error!("CRITICAL: Market stream died: {}", e);
        }
    });

    // 5. The Main Logic Loop (The Brain)
    let logic_handle = tokio::spawn(async move {
        let mut microstructure = model::MarketMicrostructure::new();
        let oracle = news_filter::NewsOracle::new();
        
        let mut last_news_check = Instant::now();
        let mut last_heartbeat = Instant::now();
        let mut is_danger_mode = false;
        
        while let Some(trade) = rx_data.recv().await {
            
            // A. UPDATE MARKET STATE
            microstructure.update(&trade);

            // B. REAL-TIME DASHBOARD (Every 3 Seconds)
            if last_heartbeat.elapsed() > Duration::from_secs(3) {
                if microstructure.prices.len() >= 20 {
                    let current_price = trade.price;
                    let ofi = microstructure.calculate_ofi();
                    
                    // Quick Volatility Calc for display
                    let min = microstructure.prices.iter().fold(f64::INFINITY, |a, &b| a.min(b));
                    let max = microstructure.prices.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
                    let volatility = (max - min) / 2.0;

                    let status = if ofi > 0.2 { "🐂 BULLISH" } 
                                 else if ofi < -0.2 { "🐻 BEARISH" } 
                                 else { "🦀 RANGING" };

                    info!(
                        "⚡ LIVE | Price: {:.5} | OFI: {:.3} | Vol: {:.5} | Mode: {}", 
                        current_price, ofi, volatility, status
                    );
                } else {
                    info!("⏳ CALIBRATING... ({}/60 ticks)", microstructure.prices.len());
                }
                last_heartbeat = Instant::now();
            }

            // C. NEWS FILTER
            if last_news_check.elapsed() > Duration::from_secs(900) {
                is_danger_mode = oracle.check_danger().await;
                last_news_check = Instant::now();
                if is_danger_mode {
                    warn!("⚠️ MARKET HALT: High Impact News Event Detected.");
                }
            }

            if is_danger_mode { continue; }

            // D. CORE ANALYSIS
            if microstructure.prices.len() >= 60 {
                let current_price = trade.price;
                
                let min = microstructure.prices.iter().fold(f64::INFINITY, |a, &b| a.min(b));
                let max = microstructure.prices.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
                let volatility = (max - min) / 2.0;

                let signal = model::QuantumSignal::analyze(&microstructure, current_price, volatility);

                if signal.confidence > 90.0 {
                    if signal.is_whale_confirmed {
                        // LOG & ALERT
                        logger::TradeLogger::log_signal(
                            &signal.direction, 
                            signal.confidence, 
                            current_price, 
                            true
                        );
                        
                        let msg = format!(
                            "🔥 *ELITE SIGNAL*\nPair: EUR/USD\nAction: *{}*\nConf: {:.1}%\nWhale: ✅\nPrice: {:.5}", 
                            signal.direction, signal.confidence, current_price
                        );
                        bot_clone.send_signal(&msg).await;
                        info!("🚀 SIGNAL SENT: {} @ {:.5}", signal.direction, current_price);
                        
                        tokio::time::sleep(Duration::from_secs(60)).await;
                    } else {
                        info!("⛔ TRAP DETECTED: Price wants {} but Whales say NO.", signal.direction);
                    }
                } 
                else if signal.confidence > 75.0 && signal.confidence < 80.0 {
                     logger::TradeLogger::log_signal("WARNING", signal.confidence, current_price, false);
                }
            }
        }
    });

    let _ = tokio::join!(market_handle, logic_handle);
    Ok(())
}