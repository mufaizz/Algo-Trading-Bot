use rayon::prelude::*;
use rand::{prelude::*};
use std::collections::VecDeque;

/// The atomic unit of market data.
#[derive(Debug, Clone, Copy)]
pub struct TradeData {
    pub price: f64,
    pub quantity: f64,
    pub is_buyer_maker: bool, // True = Sell, False = Buy
}

/// The output signal from the prediction engine.
#[derive(Debug, Clone)]
pub struct QuantumSignal {
    pub confidence: f64,
    pub direction: String,
    pub is_whale_confirmed: bool,
}

/// Stores recent market history to detect Whales and Order Flow Imbalance (OFI).
pub struct MarketMicrostructure {
    pub bids: VecDeque<f64>,
    pub asks: VecDeque<f64>,
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

    /// Updates internal state with new trade data.
    pub fn update(&mut self, trade: &TradeData) {
        if trade.is_buyer_maker {
            // Seller initiated (Bearish flow)
            self.asks.push_back(trade.quantity);
            self.bids.push_back(0.0);
        } else {
            // Buyer initiated (Bullish flow)
            self.bids.push_back(trade.quantity);
            self.asks.push_back(0.0);
        }
        self.prices.push_back(trade.price);

        // Keep last 100 ticks for flow analysis
        if self.bids.len() > 100 {
            self.bids.pop_front();
            self.asks.pop_front();
            self.prices.pop_front();
        }
    }

    /// Calculates Order Flow Imbalance (OFI).
    /// Range: -1.0 (Pure Selling) to +1.0 (Pure Buying)
    pub fn calculate_ofi(&self) -> f64 {
        let buy_vol: f64 = self.bids.iter().sum();
        let sell_vol: f64 = self.asks.iter().sum();
        let total = buy_vol + sell_vol;
        
        if total == 0.0 { return 0.0; }
        (buy_vol - sell_vol) / total
    }
}

impl QuantumSignal {
    /// Combines Monte Carlo probability with Whale Order Flow.
    pub fn analyze(market: &MarketMicrostructure, current_price: f64, volatility: f64) -> Self {
        let ofi = market.calculate_ofi();
        
        // 1. Run Monte Carlo (The Price Prediction)
        let paths = 10_000;
        let steps = 60;
        
        let results: Vec<f64> = (0..paths)
            .into_par_iter()
            .map(|_| {
                let mut rng = rand::rng();
                let mut price = current_price;
                for _ in 0..steps {
                    let shock = rng.gen_range(-1.0..1.0);
                    price += volatility * shock; 
                }
                price
            })
            .collect();

        let up_moves = results.iter().filter(|&&p| p > current_price).count();
        let prob_up = up_moves as f64 / paths as f64;
        
        // 2. Determine Raw Direction
        let (direction, raw_confidence) = if prob_up > 0.70 {
            ("UP", prob_up * 100.0)
        } else if prob_up < 0.30 {
            ("DOWN", (1.0 - prob_up) * 100.0)
        } else {
            ("NEUTRAL", 50.0)
        };

        // 3. Whale Confirmation (The Filter)
        // If Price says UP but Whales are Selling (OFI Negative), invalidate the signal.
        let is_whale_confirmed = match direction {
            "UP" => ofi > 0.2,   // Must have buying pressure
            "DOWN" => ofi < -0.2, // Must have selling pressure
            _ => false,
        };

        Self {
            confidence: raw_confidence,
            direction: direction.to_string(),
            is_whale_confirmed,
        }
    }
}