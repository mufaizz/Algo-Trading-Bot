use tokio::sync::mpsc;
use log::{info, error, warn};
use std::env;
use dotenv::dotenv;
use std::time::{Duration, Instant};

mod binance_client;
mod model;
mod telegram;
mod news_filter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();
    env_logger::builder().filter_level(log::LevelFilter::Info).init();
    
    let token = env::var("TELEGRAM_TOKEN").expect("TELEGRAM_TOKEN not set");
    let chat_id = env::var("CHAT_ID").expect("CHAT_ID not set");
    
    info!("🚀 QUANTUM ENGINE + WHALE DETECTOR INITIALIZED");
    
    // Change channel type to TradeData
    let (tx_data, mut rx_data) = mpsc::channel::<model::TradeData>(100);
    
    let bot = telegram::TelegramBot::new(token, chat_id);
    let bot_clone = bot.clone();
    
    let market_handle = tokio::spawn(async move {
        if let Err(e) = binance_client::start_market_stream(tx_data).await {
            error!("Market stream died: {}", e);
        }
    });

    let logic_handle = tokio::spawn(async move {
        // Initialize Analysis Engine
        let mut microstructure = model::MarketMicrostructure::new();
        let oracle = news_filter::NewsOracle::new();
        
        let mut last_news_check = Instant::now();
        let mut is_danger_mode = false;
        
        while let Some(trade) = rx_data.recv().await {
            // 1. News Filter
            if last_news_check.elapsed() > Duration::from_secs(900) {
                is_danger_mode = oracle.check_danger().await;
                last_news_check = Instant::now();
            }
            if is_danger_mode { continue; }

            // 2. Update Market State (Price + Volume)
            microstructure.update(&trade);

            // 3. Core Logic (Run only if we have enough history)
            if microstructure.prices.len() >= 60 {
                let current_price = trade.price;
                
                // Calculate Volatility
                let min = microstructure.prices.iter().fold(f64::INFINITY, |a, &b| a.min(b));
                let max = microstructure.prices.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
                let volatility = (max - min) / 2.0;

                // Run Analysis
                let signal = model::QuantumSignal::analyze(&microstructure, current_price, volatility);

                // 4. ELITE DECISION MATRIX
                if signal.confidence > 90.0 {
                    if signal.is_whale_confirmed {
                        // A: High Confidence + Whale Backing = TRADE
                        let msg = format!(
                            "🔥 *ELITE SIGNAL*\nPair: EUR/USD\nAction: *{}*\nConf: {:.1}%\nwhale: ✅ CONFIRMED\nPrice: {:.5}",
                            signal.direction, signal.confidence, current_price
                        );
                        bot_clone.send_signal(&msg).await;
                        tokio::time::sleep(Duration::from_secs(60)).await;
                    } else {
                        // B: High Confidence + NO Whale = TRAP (Log but don't trade)
                        info!("⛔ TRAP AVOIDED: Price signals {} but Whales are selling.", signal.direction);
                    }
                }
            }
        }
    });

    let _ = tokio::join!(market_handle, logic_handle);
    Ok(())
}