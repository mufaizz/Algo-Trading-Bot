use rayon::prelude::*;
use rand::prelude::*;

#[derive(Debug, Clone)]
pub struct QuantumSignal {
    pub confidence: f64,
    pub direction: String,
}

impl QuantumSignal {
    /// Runs a parallelized Monte Carlo simulation to determine future price probability.
    /// 
    /// * `current_price` - The latest execution price from Binance.
    /// * `volatility` - Standard deviation of the last 60 ticks.
    pub fn run_monte_carlo(current_price: f64, volatility: f64) -> Self {
        let paths = 10_000;
        let steps = 60; // Predicting 60 seconds into the future (1-min candle)
        
        // Rayon parallel iterator for multi-core processing
        let results: Vec<f64> = (0..paths)
            .into_par_iter()
            .map(|_| {
                let mut rng = rand::rng();
                let mut price = current_price;
                
                
                for _ in 0..steps {
                    let shock = rng.random_range(-1.0..1.0);
                    let change = volatility * shock; 
                    price += change;
                }
                price
            })
            .collect();

        
        let up_moves = results.iter().filter(|&&p| p > current_price).count();
        let prob_up = up_moves as f64 / paths as f64;

        
        if prob_up > 0.70 {
            Self { 
                confidence: prob_up * 100.0, 
                direction: "UP/LONG".to_string() 
            }
        } else if prob_up < 0.30 {
            Self { 
                confidence: (1.0 - prob_up) * 100.0, 
                direction: "DOWN/SHORT".to_string() 
            }
        } else {
            Self { 
                confidence: 50.0, 
                direction: "NEUTRAL".to_string() 
            }
        }
    }
}