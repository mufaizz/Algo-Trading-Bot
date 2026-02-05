use futures_util::StreamExt;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use serde_json::Value;
use log::{info, warn, error}; // Now 'warn' is actually used
use url::Url;
use crate::model::TradeData;

pub async fn start_market_stream(tx: tokio::sync::mpsc::Sender<TradeData>) -> anyhow::Result<()> {
    // US EAST FIX: Use Binance.US
    let url = Url::parse("wss://stream.binance.us:9443/ws/btcusdt@aggTrade")?;
    
    info!("🔌 CONNECTING to Binance.US (US East Mode)...");
    
    match connect_async(url).await {
        Ok((mut ws_stream, _)) => {
            info!("✅ SOCKET OPEN. Listening for packets...");

            while let Some(message) = ws_stream.next().await {
                match message {
                    Ok(Message::Text(text)) => {
                        if let Ok(v) = serde_json::from_str::<Value>(&text) {
                            let p = v.get("p").and_then(|x| x.as_str()).and_then(|x| x.parse::<f64>().ok());
                            let q = v.get("q").and_then(|x| x.as_str()).and_then(|x| x.parse::<f64>().ok());
                            let m = v.get("m").and_then(|x| x.as_bool());

                            if let (Some(price), Some(qty), Some(is_maker)) = (p, q, m) {
                                let trade = TradeData {
                                    price,
                                    quantity: qty,
                                    is_buyer_maker: is_maker,
                                };
                                
                                if tx.send(trade).await.is_err() { 
                                    // [FIX] Using warn here
                                    warn!("⚠️ Internal Channel Closed. Stopping Stream.");
                                    break; 
                                }
                            } else {
                                // [FIX] Using warn here for bad data
                                if v.get("e").is_some() {
                                    warn!("⚠️ Data Parse Warning: Missing Fields in {:?}", v);
                                }
                            }
                        }
                    }
                    Ok(Message::Ping(_)) => continue,
                    Ok(Message::Close(_)) => {
                        // [FIX] Using warn here
                        warn!("❌ SERVER CLOSED CONNECTION");
                        break;
                    },
                    Err(e) => error!("❌ SOCKET ERROR: {}", e),
                    _ => {}
                }
            }
        }
        Err(e) => error!("❌ CONNECTION FAILED: {} (Check Internet)", e),
    }
    Ok(())
}