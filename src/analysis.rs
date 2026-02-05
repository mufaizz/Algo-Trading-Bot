use std::collections::VecDeque;

pub struct MarketMicrostructure {
    pub bids: VecDeque<f64>, // Best Bid Volumes
    pub asks: VecDeque<f64>, // Best Ask Volumes
    pub prices: VecDeque<f64>,
}

impl MarketMicrostructure {
    pub fn new() -> Self {
        Self {
            bids: VecDeque::new(),
            asks: VecDeque::new(),
            prices: VecDeque::new(),
        }
    }

    /// Updates the internal state with new tick data
    pub fn update(&mut self, price: f64, volume: f64, is_buyer_maker: bool) {
        // In a full L2 stream, we'd have exact bid/ask depth.
        // For AggTrades, we infer flow:
        // is_buyer_maker = TRUE -> Seller hit the bid (Bearish Flow)
        // is_buyer_maker = FALSE -> Buyer lifted the ask (Bullish Flow)
        
        if is_buyer_maker {
            self.asks.push_back(volume);
            self.bids.push_back(0.0);
        } else {
            self.bids.push_back(volume);
            self.asks.push_back(0.0);
        }
        self.prices.push_back(price);

        // Keep rolling window short (e.g., 50 ticks) for HFT
        if self.bids.len() > 50 {
            self.bids.pop_front();
            self.asks.pop_front();
            self.prices.pop_front();
        }
    }

    /// Calculates Order Flow Imbalance (OFI)
    /// Returns: Positive = Buying Pressure, Negative = Selling Pressure
    pub fn calculate_ofi(&self) -> f64 {
        let buy_vol: f64 = self.bids.iter().sum();
        let sell_vol: f64 = self.asks.iter().sum();
        
        // Normalize
        let total = buy_vol + sell_vol;
        if total == 0.0 { return 0.0; }
        
        (buy_vol - sell_vol) / total
    }

    /// Detects "Whale" activity
    pub fn whale_alert(&self) -> Option<String> {
        let ofi = self.calculate_ofi();
        
        // Threshold: If > 80% of volume is one-sided, it's a Whale
        if ofi > 0.8 {
            Some("WHALE BUYING".to_string())
        } else if ofi < -0.8 {
            Some("WHALE SELLING".to_string())
        } else {
            None
        }
    }
}