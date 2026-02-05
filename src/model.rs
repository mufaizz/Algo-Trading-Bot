use rayon::prelude::*;
use rand::prelude::*;
use std::collections::VecDeque;

#[derive(Debug, Clone, Copy)]
pub struct TradeData {
    pub price: f64,
    pub quantity: f64,
    pub is_buyer_maker: bool,
}

#[derive(Debug, Clone)]
pub struct QuantumSignal {
    pub confidence: f64,
    pub direction: String,
    pub is_whale_confirmed: bool,
}

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

    pub fn update(&mut self, trade: &TradeData) {
        if trade.is_buyer_maker {
            self.asks.push_back(trade.quantity);
            self.bids.push_back(0.0);
        } else {
            self.bids.push_back(trade.quantity);
            self.asks.push_back(0.0);
        }
        self.prices.push_back(trade.price);

        if self.bids.len() > 100 {
            self.bids.pop_front();
            self.asks.pop_front();
            self.prices.pop_front();
        }
    }

    pub fn calculate_ofi(&self) -> f64 {
        let buy_vol: f64 = self.bids.iter().sum();
        let sell_vol: f64 = self.asks.iter().sum();
        let total = buy_vol + sell_vol;
        if total == 0.0 { return 0.0; }
        (buy_vol - sell_vol) / total
    }
}

impl QuantumSignal {
    pub fn analyze(market: &MarketMicrostructure, current_price: f64, volatility: f64) -> Self {
        let ofi = market.calculate_ofi();
        
        // Monte Carlo
        let paths = 10_000;
        let results: Vec<f64> = (0..paths)
            .into_par_iter()
            .map(|_| {
                let mut rng = rand::rng();
                let mut price = current_price;
                for _ in 0..60 {
                    price += volatility * rng.random_range(-1.0..1.0); 
                }
                price
            })
            .collect();

        let up_moves = results.iter().filter(|&&p| p > current_price).count();
        let prob_up = up_moves as f64 / paths as f64;
        
        let (direction, raw_confidence) = if prob_up > 0.70 {
            ("UP", prob_up * 100.0)
        } else if prob_up < 0.30 {
            ("DOWN", (1.0 - prob_up) * 100.0)
        } else {
            ("NEUTRAL", 50.0)
        };

        // Whale Confirmation
        let is_whale_confirmed = match direction {
            "UP" => ofi > 0.2,   
            "DOWN" => ofi < -0.2, 
            _ => false,
        };

        Self {
            confidence: raw_confidence,
            direction: direction.to_string(),
            is_whale_confirmed,
        }
    }

    // --- NEW: PHASE 5 RISK ENGINE ---
    #[allow(dead_code)]
    pub fn calculate_stake(&self) -> String {
        // Standard Binary Options Payout (85%)
        let b = 0.85; 
        let p = self.confidence / 100.0;
        let q = 1.0 - p;
        
        // Kelly Formula: (bp - q) / b
        let raw_kelly = ((b * p) - q) / b;
        
        // Safety: Use "Tenth Kelly" to minimize Drawdown
        // Max Bet Cap: 5% of account
        let safe_stake = (raw_kelly * 0.10).max(0.01).min(0.05);
        
        if self.confidence < 80.0 {
            return "1.0% (Min)".to_string(); // Flat risk for lower confidence
        }

        format!("{:.1}%", safe_stake * 100.0)
    }
} 