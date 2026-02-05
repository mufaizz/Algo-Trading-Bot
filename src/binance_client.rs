use futures_util::StreamExt;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use serde_json::Value;
use log::{info, warn, error};
use url::Url;
use crate::model::TradeData;

pub async fn start_market_stream(tx: tokio::sync::mpsc::Sender<TradeData>) -> anyhow::Result<()> {
    // SWITCH TO FUTURES SERVER (fstream)
    // This server is separate from Spot and often works when Spot is blocked.
    let url = Url::parse("wss://fstream.binance.com/ws/btcusdt@aggTrade")?;
    
    info!("🔌 CONNECTING to Binance FUTURES (Backup Line)...");
    
    let connect_future = connect_async(url);
    
    match tokio::time::timeout(std::time::Duration::from_secs(10), connect_future).await {
        Ok(Ok((mut ws_stream, _))) => {
            info!("✅ FUTURES LINK ESTABLISHED. Waiting for ticks...");
            
            let mut first_packet = true;

            while let Some(message) = ws_stream.next().await {
                match message {
                    Ok(Message::Text(text)) => {
                        // [DEBUG] Print ONLY the first packet to prove data is flowing
                        if first_packet {
                            info!("📩 DATA FLOWING! First packet received.");
                            first_packet = false;
                        }

                        if let Ok(v) = serde_json::from_str::<Value>(&text) {
                            // Futures JSON parsing
                            let p = v.get("p").and_then(|x| x.as_str()).and_then(|x| x.parse::<f64>().ok());
                            let q = v.get("q").and_then(|x| x.as_str()).and_then(|x| x.parse::<f64>().ok());
                            // Futures often skips 'm', default to false
                            let is_maker = v.get("m").and_then(|x| x.as_bool()).unwrap_or(false);

                            if let (Some(price), Some(qty)) = (p, q) {
                                let trade = TradeData {
                                    price,
                                    quantity: qty,
                                    is_buyer_maker: is_maker,
                                };
                                if tx.send(trade).await.is_err() { break; }
                            }
                        }
                    }
                    Ok(Message::Ping(_)) => continue,
                    Ok(Message::Close(_)) => {
                        warn!("❌ SERVER CLOSED CONNECTION");
                        break;
                    },
                    Err(e) => error!("❌ SOCKET ERROR: {}", e),
                    _ => {}
                }
            }
        },
        Ok(Err(e)) => error!("❌ CONNECTION REFUSED: {}", e),
        Err(_) => error!("❌ TIMEOUT: Futures server silent. Firewall is blocking."),
    }
    Ok(())
}