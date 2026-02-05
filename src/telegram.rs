use reqwest::Client;
use serde::Serialize;
use log::{info, error};

#[derive(Serialize)]
struct TelegramMessage {
    chat_id: String,
    text: String,
    parse_mode: String,
}

#[derive(Clone)]
pub struct TelegramBot {
    client: Client,
    token: String,
    chat_id: String,
}

impl TelegramBot {
    pub fn new(token: String, chat_id: String) -> Self {
        Self {
            client: Client::new(),
            token,
            chat_id,
        }
    }

    pub async fn send_signal(&self, msg: &str) {
        let url = format!("https://api.telegram.org/bot{}/sendMessage", self.token);
        let payload = TelegramMessage {
            chat_id: self.chat_id.clone(),
            text: msg.to_string(),
            parse_mode: "Markdown".to_string(),
        };

        match self.client.post(&url).json(&payload).send().await {
            Ok(resp) => {
                if !resp.status().is_success() {
                    error!("Telegram API Error: {:?}", resp.text().await);
                } else {
                    info!("Alert dispatched.");
                }
            },
            Err(e) => error!("Network Error sending alert: {}", e),
        }
    }
}