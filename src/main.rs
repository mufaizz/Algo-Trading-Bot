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
mod logger; // The Black Box Recorder

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1. Initialize Environment
    dotenv().ok();
    env_logger::builder().filter_level(log::LevelFilter::Info).init();
    
    // Load Credentials
    let token = env::var("TELEGRAM_TOKEN").expect("TELEGRAM_TOKEN not set");
    let chat_id = env::var("CHAT_ID").expect("CHAT_ID not set");
    
    info!("🚀 MUFAIZ THE GOAT");
    
    // 2. Create Data Channels (Now carrying full TradeData, not just price)
    let (tx_data, mut rx_data) = mpsc::channel::<model::TradeData>(100);
    
    // 3. Spawn Telegram Bot
    let bot = telegram::TelegramBot::new(token, chat_id);
    let bot_clone = bot.clone();
    
    // 4. Spawn Market Data Stream (The Eyes)
    let market_handle = tokio::spawn(async move {
        if let Err(e) = binance_client::start_market_stream(tx_data).await {
            error!("CRITICAL: Market stream died: {}", e);
        }
    });

    // 5. The Main Logic Loop (The Brain)
    let logic_handle = tokio::spawn(async move {
        // Initialize Sub-Systems
        let mut microstructure = model::MarketMicrostructure::new();
        let oracle = news_filter::NewsOracle::new();
        
        // State Variables
        let mut last_news_check = Instant::now();
        let mut is_danger_mode = false;
        
        // --- THE INFINITE LOOP ---
        while let Some(trade) = rx_data.recv().await {
            
            // A. NEWS FILTER CHECK (Every 15 Minutes)
            if last_news_check.elapsed() > Duration::from_secs(900) {
                is_danger_mode = oracle.check_danger().await;
                last_news_check = Instant::now();
                if is_danger_mode {
                    warn!("⚠️ MARKET HALT: High Impact News Event Detected.");
                }
            }

            if is_danger_mode {
                continue; // Skip all logic if news is active
            }

            // B. UPDATE MARKET STATE
            // Feeds Price + Volume + Maker Side into the Order Flow Engine
            microstructure.update(&trade);

            // C. CORE ANALYSIS (Run only if we have sufficient history)
            if microstructure.prices.len() >= 60 {
                let current_price = trade.price;
                
                // Calculate Dynamic Volatility (Std Dev of last 60 ticks)
                let min = microstructure.prices.iter().fold(f64::INFINITY, |a, &b| a.min(b));
                let max = microstructure.prices.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
                let volatility = (max - min) / 2.0;

                // RUN THE QUANTUM + WHALE MODELS
                let signal = model::QuantumSignal::analyze(&microstructure, current_price, volatility);

                // D. ELITE DECISION MATRIX
                if signal.confidence > 90.0 {
                    if signal.is_whale_confirmed {
                        // --- SCENARIO 1: FULL GO ---
                        // 1. Log to Black Box
                        logger::TradeLogger::log_signal(
                            &signal.direction, 
                            signal.confidence, 
                            current_price, 
                            true
                        );

                        // 2. Alert User
                        let msg = format!(
                            "🔥 *ELITE SIGNAL*\nPair: EUR/USD\nAction: *{}*\nConf: {:.1}%\nWhale: ✅ CONFIRMED\nPrice: {:.5}",
                            signal.direction, signal.confidence, current_price
                        );
                        bot_clone.send_signal(&msg).await;
                        
                        // 3. Cooldown (Prevent double signals on same candle)
                        tokio::time::sleep(Duration::from_secs(60)).await;
                    } else {
                        // --- SCENARIO 2: TRAP DETECTED ---
                        // Price looks good, but Volume is fake.
                        info!("⛔ TRAP AVOIDED: Price signals {} but Whales are selling.", signal.direction);
                    }
                } 
                else if signal.confidence > 75.0 && signal.confidence < 80.0 {
                    // --- SCENARIO 3: WARNING ZONE ---
                     logger::TradeLogger::log_signal(
                        "WARNING", 
                        signal.confidence, 
                        current_price, 
                        false
                    );
                    let msg = format!("⚠️ PREPARE: Possible {} Setup", signal.direction);
                    bot_clone.send_signal(&msg).await;
                    tokio::time::sleep(Duration::from_secs(10)).await;
                }
            }
        }
    });

    // Keep the process alive
    let _ = tokio::join!(market_handle, logic_handle);
    Ok(())
}