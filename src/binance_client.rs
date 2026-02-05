use futures_util::StreamExt;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use serde_json::Value;
use log::{info, warn};
use url::Url;

pub async fn start_market_stream(tx: tokio::sync::mpsc::Sender<f64>) -> anyhow::Result<()> {
    let url = Url::parse("wss://fstream.binance.com/ws/eurusdt@aggTrade")?;
    
    match connect_async(url).await {
        Ok((mut ws_stream, _)) => {
            info!("Connected to Binance Futures (EUR/USD).");

            while let Some(message) = ws_stream.next().await {
                match message {
                    Ok(Message::Text(text)) => {
                        if let Ok(v) = serde_json::from_str::<Value>(&text) {
                            if let Some(price_str) = v.get("p") {
                                if let Ok(price) = price_str.as_str().unwrap_or("0").parse::<f64>() {
                                    if let Err(_) = tx.send(price).await {
                                        break; 
                                    }
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
        Err(e) => warn!("Failed to connect to Binance: {}", e),
    }
    
    Ok(())
}