use futures_util::StreamExt;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use serde_json::Value;
use log::{info, warn};
use url::Url;
use crate::model::TradeData;

pub async fn start_market_stream(tx: tokio::sync::mpsc::Sender<TradeData>) -> anyhow::Result<()> {
    // "aggTrade" stream gives us volume (q) and buyer_maker (m)
    let url = Url::parse("wss://fstream.binance.com/ws/eurusdt@aggTrade")?;
    
    match connect_async(url).await {
        Ok((mut ws_stream, _)) => {
            info!("Connected to Binance Institutional Stream (Price + Volume).");

            while let Some(message) = ws_stream.next().await {
                match message {
                    Ok(Message::Text(text)) => {
                        if let Ok(v) = serde_json::from_str::<Value>(&text) {
                            // Extract Price (p), Quantity (q), and Maker Side (m)
                            let p = v.get("p").and_then(|x| x.as_str()).and_then(|x| x.parse::<f64>().ok());
                            let q = v.get("q").and_then(|x| x.as_str()).and_then(|x| x.parse::<f64>().ok());
                            let m = v.get("m").and_then(|x| x.as_bool());

                            if let (Some(price), Some(qty), Some(is_maker)) = (p, q, m) {
                                let trade = TradeData {
                                    price,
                                    quantity: qty,
                                    is_buyer_maker: is_maker,
                                };
                                
                                // Send data to main loop
                                if tx.send(trade).await.is_err() {
                                    break; 
                                }
                            }
                        }
                    }
                    Ok(Message::Ping(_)) => continue, 
                    Ok(Message::Close(_)) => break,
                    Err(e) => warn!("WebSocket Error: {}", e),
                    _ => {}
                }
            }
        }
        Err(e) => warn!("Connection Failed: {}", e),
    }
    Ok(())
}