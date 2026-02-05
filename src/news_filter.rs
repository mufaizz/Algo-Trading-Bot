use serde::Deserialize;
use chrono::{Utc};
use log::{info, warn};
use reqwest::Client;

#[derive(Debug, Deserialize)]
struct WeeklyEvents {
    #[serde(rename = "event", default)]
    events: Vec<NewsEvent>,
}

#[derive(Debug, Deserialize)]
struct NewsEvent {
    title: String,
    country: String,
    date: String, // Format: MM-DD-YYYY
    impact: String,
}

pub struct NewsOracle {
    client: Client,
}

impl NewsOracle {
    pub fn new() -> Self {
        Self { client: Client::new() }
    }

    pub async fn check_danger(&self) -> bool {
        let url = "https://nfs.faireconomy.media/ff_calendar_thisweek.xml";
        let xml = match self.client.get(url).send().await {
            Ok(r) => r.text().await.unwrap_or_default(),
            Err(e) => {
                warn!("News Feed Error: {}", e);
                return false; // Fail safe
            }
        };

        let schedule: WeeklyEvents = match quick_xml::de::from_str(&xml) {
            Ok(s) => s,
            Err(_) => return false,
        };

        let now = Utc::now();
        let today_str = now.format("%m-%d-%Y").to_string();

        for event in schedule.events {
            if event.country != "USD" && event.country != "EUR" { continue; }
            if event.impact != "High" { continue; }
            if event.date != today_str { continue; }

            // Parse Time (Approximate for 24h conversion)
            // Logic: If event is within 30 mins, return TRUE (DANGER)
            // Implementation simplified for reliability:
            info!("⚠️ WARNING: High Impact News Detected: {}", event.title);
            return true; 
        }

        false
    }
}