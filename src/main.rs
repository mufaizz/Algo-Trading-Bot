use tokio::sync::mpsc;
use log::{info, error, warn};
use std::env;
use dotenv::dotenv;
use std::time::{Duration, Instant};

mod binance_client;
mod model;
mod telegram;
mod news_filter;
mod logger;
mod simulator; // NEW MODULE

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();
    if env::var("RUST_LOG").is_err() { env::set_var("RUST_LOG", "info"); }
    env_logger::init();
    
    let token = env::var("TELEGRAM_TOKEN").expect("TELEGRAM_TOKEN not set");
    let chat_id = env::var("CHAT_ID").expect("CHAT_ID not set");
    
    info!("🚀 MUFAIZ THE GOAT");
    
    let (tx_data, mut rx_data) = mpsc::channel::<model::TradeData>(100);
    let bot = telegram::TelegramBot::new(token, chat_id);
    let bot_clone = bot.clone();
    
    // START MARKET STREAM
    tokio::spawn(async move {
        if let Err(e) = binance_client::start_market_stream(tx_data).await {
            error!("CRITICAL: Market stream died: {}", e);
        }
    });

    let logic_handle = tokio::spawn(async move {
        let mut microstructure = model::MarketMicrostructure::new();
        let oracle = news_filter::NewsOracle::new();
        // INITIALIZE WALLET
        let mut wallet = simulator::PaperWallet::new();
        
        let mut last_news_check = Instant::now();
        let mut trades_processed = 0;

        loop {
            tokio::select! {
                Some(trade) = rx_data.recv() => {
                    microstructure.update(&trade);
                    trades_processed += 1;
                    
                    // CHECK FOR EXPIRED TRADES (Every tick)
                    wallet.update(trade.price);

                    if microstructure.prices.len() >= 20 {
                        let current_price = trade.price;
                        let min = microstructure.prices.iter().fold(f64::INFINITY, |a, &b| a.min(b));
                        let max = microstructure.prices.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
                        let volatility = (max - min) / 2.0;

                        // DYNAMIC THRESHOLD
                        let mut required_conf = 90.0;
                        if volatility > 100.0 { required_conf = 94.0; } 
                        else if volatility < 20.0 { required_conf = 88.0; }

                        let signal = model::QuantumSignal::analyze(&microstructure, current_price, volatility);

                        if signal.confidence > required_conf {
                            if signal.is_whale_confirmed {
                                // 1. Calculate Stake
                                let stake_str = signal.calculate_stake(); 
                                // Clean string "4.5%" -> 4.5
                                let stake_val = stake_str.trim_end_matches('%').parse::<f64>().unwrap_or(1.0);

                                // 2. EXECUTE PAPER TRADE
                                wallet.open_trade(signal.direction.clone(), current_price, stake_val);

                                logger::TradeLogger::log_signal(&signal.direction, signal.confidence, current_price, true);
                                
                                let msg = format!(
                                    "🔥 *ELITE SIGNAL*\nPair: BTC/USDT\nAction: *{}*\nConf: {:.1}%\n💰 Stake: *{}*\nPrice: {:.2}", 
                                    signal.direction, signal.confidence, stake_str, current_price
                                );
                                bot_clone.send_signal(&msg).await;
                                info!("🚀 SIGNAL FIRED | Opening Trade on Wallet...");
                                tokio::time::sleep(Duration::from_secs(60)).await;
                            }
                        }
                    }
                }

                _ = tokio::time::sleep(Duration::from_secs(3)) => {
                     if trades_processed == 0 {
                         info!("⏳ CONNECTING...");
                     } else if microstructure.prices.len() < 20 {
                         info!("⏳ CALIBRATING BTC... ({}/20 ticks)", microstructure.prices.len());
                     } else {
                        let ofi = microstructure.calculate_ofi();
                        let status = if ofi > 0.2 { "🐂" } else if ofi < -0.2 { "🐻" } else { "🦀" };
                        let price = microstructure.prices.back().unwrap_or(&0.0);
                        let min = microstructure.prices.iter().fold(f64::INFINITY, |a, &b| a.min(b));
                        let max = microstructure.prices.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
                        let vol = (max - min) / 2.0;

                        // DASHBOARD NOW SHOWS BALANCE
                        info!("⚡ BTC: {:.2} | Vol: {:.2} | 💰 Bal: ${:.0} (WR: {}%) | {}", 
                            price, vol, wallet.balance, 
                            if wallet.wins + wallet.losses > 0 { (wallet.wins as f64 / (wallet.wins + wallet.losses) as f64 * 100.0) as u64 } else { 0 },
                            status
                        );
                     }
                }
            }

            if last_news_check.elapsed() > Duration::from_secs(900) {
                if oracle.check_danger().await {
                    warn!("⚠️ MARKET HALT: High Impact News.");
                    tokio::time::sleep(Duration::from_secs(300)).await;
                }
                last_news_check = Instant::now();
            }
        }
    });

    logic_handle.await?;
    Ok(())
}