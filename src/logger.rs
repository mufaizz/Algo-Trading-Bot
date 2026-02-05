use std::fs::OpenOptions;
use std::io::Write;
use chrono::Local;
use log::error;

pub struct TradeLogger;

impl TradeLogger {
    /// Appends a new signal to the trade journal.
    pub fn log_signal(direction: &str, confidence: f64, price: f64, whale_confirmed: bool) {
        let file_path = "trade_journal.csv";
        
        // Open file in Append mode
        let mut file = match OpenOptions::new()
            .create(true)
            .write(true)
            .append(true)
            .open(file_path) {
                Ok(f) => f,
                Err(e) => {
                    error!("FAILED TO OPEN LOG FILE: {}", e);
                    return;
                }
            };

        let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
        let whale_status = if whale_confirmed { "CONFIRMED" } else { "UNCONFIRMED" };

        // Format: Time, Direction, Confidence, Price, Whale_Status
        let record = format!("{},{},{:.2}%,{:.5},{}\n", 
            timestamp, direction, confidence, price, whale_status);

        if let Err(e) = file.write_all(record.as_bytes()) {
            error!("FAILED TO WRITE LOG: {}", e);
        }
    }
}