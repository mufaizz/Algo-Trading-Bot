use tokio::sync::mpsc;
use log::{info, error, warn};
use std::env;
use dotenv::dotenv;
use std::time::{Duration, Instant};

// --- IMPORTS ---
mod client;
mod model;
mod telegram;
mod news_filter;
mod logger;
mod simulator; // <--- CRITICAL: This imports the simulator file

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();
    if env::var("RUST_LOG").is_err() { env::set_var("RUST_LOG", "info"); }
    env_logger::init();
    
    let token = env::var("TELEGRAM_TOKEN").expect("TELEGRAM_TOKEN not set");
    let chat_id = env::var("CHAT_ID").expect("CHAT_ID not set");
    
    info!("🚀 QUANTUM ENGINE v3.0 (SCOREBOARD ACTIVE)");
    // Add this near the top of main(), around line 26:
    
    
    let (tx_data, mut rx_data) = mpsc::channel::<model::TradeData>(100);
    let bot = telegram::TelegramBot::new(token, chat_id);
    bot.send_signal("🚀 SYSTEM ONLINE: Connected to Binance Global. Scanning for Whales...").await;
    let bot_clone = bot.clone();
    
    // SPAWN MARKET STREAM
    tokio::spawn(async move {
        // We assume binance_client is already fixed and working
        if let Err(e) = client::start_market_stream(tx_data).await {
            error!("CRITICAL: Market stream died: {}", e);
        }
    });

    // SPAWN LOGIC ENGINE
    let logic_handle = tokio::spawn(async move {
        let mut microstructure = model::MarketMicrostructure::new();
        let oracle = news_filter::NewsOracle::new();
        
        // --- INITIALIZE WALLET ---
        let mut wallet = simulator::PaperWallet::new(); 
        info!("💰 VIRTUAL WALLET INITIALIZED: $10,000");

        let mut last_news_check = Instant::now();
        let mut trades_processed = 0;

        loop {
            tokio::select! {
                Some(trade) = rx_data.recv() => {
                    microstructure.update(&trade);
                    trades_processed += 1;
                    
                    // --- UPDATE WALLET (Check for wins/losses) ---
                    wallet.update(trade.price);

                    // Speed Hack: Start showing dashboard after just 20 ticks
                    if microstructure.prices.len() >= 20 {
                        let current_price = trade.price;
                        
                        // Calculate Volatility
                        let min = microstructure.prices.iter().fold(f64::INFINITY, |a, &b| a.min(b));
                        let max = microstructure.prices.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
                        let volatility = (max - min) / 2.0;

                        // Dynamic Threshold Logic
                        let mut required_conf = 90.0;
                        if volatility > 100.0 { required_conf = 94.0; } 
                        else if volatility < 20.0 { required_conf = 88.0; }

                        // Analyze Signal
                        let signal = model::QuantumSignal::analyze(&microstructure, current_price, volatility);

                        if signal.confidence > required_conf {
                            if signal.is_whale_confirmed {
                                // 1. Calculate Bet Size
                                let stake_str = signal.calculate_stake(); 
                                let stake_val = stake_str.trim_end_matches('%').parse::<f64>().unwrap_or(1.0);

                                // 2. EXECUTE TRADE IN SIMULATOR
                                wallet.open_trade(signal.direction.clone(), current_price, stake_val);

                                // 3. Log & Alert
                                logger::TradeLogger::log_signal(&signal.direction, signal.confidence, current_price, true);
                                
                                let msg = format!(
                                    "🔥 *ELITE SIGNAL*\nPair: BTC/USDT\nAction: *{}*\nConf: {:.1}%\n💰 Stake: *{}*\nPrice: {:.2}", 
                                    signal.direction, signal.confidence, stake_str, current_price
                                );
                                bot_clone.send_signal(&msg).await;
                                
                                info!("🚀 SIGNAL FIRED: {} | Stake: {} | Balance: ${:.2}", signal.direction, stake_str, wallet.balance);
                                
                                // Cool down to prevent spam
                                tokio::time::sleep(Duration::from_secs(60)).await;
                            }
                        }
                    }
                }

                // --- HEARTBEAT DASHBOARD ---
                _ = tokio::time::sleep(Duration::from_secs(3)) => {
                     if trades_processed == 0 {
                         info!("⏳ WAITING FOR DATA...");
                     } else if microstructure.prices.len() < 20 {
                         info!("⏳ CALIBRATING... ({}/20 ticks)", microstructure.prices.len());
                     } else {
                        // DASHBOARD WITH WALLET BALANCE
                        let ofi = microstructure.calculate_ofi();
                        let status = if ofi > 0.2 { "🐂" } else if ofi < -0.2 { "🐻" } else { "🦀" };
                        let price = microstructure.prices.back().unwrap_or(&0.0);
                        
                        // Win Rate Calc
                        let total_trades = wallet.wins + wallet.losses;
                        let win_rate = if total_trades > 0 {
                            (wallet.wins as f64 / total_trades as f64 * 100.0) as u64
                        } else { 0 };

                        info!("⚡ BTC: {:.2} | 💰 Bal: ${:.0} (WR: {}%) | OFI: {:.3} | {}", 
                            price, wallet.balance, win_rate, ofi, status);
                     }
                }
            }

            // News Check
            if last_news_check.elapsed() > Duration::from_secs(900) {
                if oracle.check_danger().await {
                    warn!("⚠️ MARKET HALT: High Impact News.");
                    continue
                    // tokio::time::sleep(Duration::from_secs(300)).await;
                }
                last_news_check = Instant::now();
            }
        }
    });

    logic_handle.await?;
    Ok(())
}