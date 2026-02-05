use std::collections::VecDeque;
use std::time::{Instant, Duration};
use log::info;

pub struct PaperWallet {
    pub balance: f64,
    pub active_trades: VecDeque<VirtualTrade>,
    pub wins: u32,
    pub losses: u32,
}

pub struct VirtualTrade {
    pub entry_price: f64,
    pub direction: String,
    pub stake: f64,
    pub open_time: Instant,
}

impl PaperWallet {
    pub fn new() -> Self {
        Self {
            balance: 10_000.0, // Starting Balance
            active_trades: VecDeque::new(),
            wins: 0,
            losses: 0,
        }
    }

    pub fn open_trade(&mut self, direction: String, price: f64, stake_pct: f64) {
        let stake_amount = self.balance * (stake_pct / 100.0);
        
        self.active_trades.push_back(VirtualTrade {
            entry_price: price,
            direction,
            stake: stake_amount,
            open_time: Instant::now(),
        });

        info!("🎰 TRADE OPENED | Stake: ${:.2} | Entry: {:.2}", stake_amount, price);
    }

    pub fn update(&mut self, current_price: f64) {
        let mut new_wins = 0;
        let mut new_losses = 0;

        // Extract indices of expired trades
        let mut finished_indices = Vec::new();
        for (i, trade) in self.active_trades.iter().enumerate() {
            if trade.open_time.elapsed() >= Duration::from_secs(60) {
                finished_indices.push(i);
            }
        }

        // Process expired trades
        for i in finished_indices.into_iter().rev() {
            let trade = self.active_trades.remove(i).unwrap();
            let is_win = match trade.direction.as_str() {
                "UP" => current_price > trade.entry_price,
                "DOWN" => current_price < trade.entry_price,
                _ => false,
            };

            if is_win {
                let profit = trade.stake * 0.85; // 85% Payout
                self.balance += profit;
                new_wins += 1;
                info!("🏆 WINNER | +${:.2} | Price: {:.2} vs Entry: {:.2}", profit, current_price, trade.entry_price);
            } else {
                self.balance -= trade.stake;
                new_losses += 1;
                info!("💀 LOSS | -${:.2} | Price: {:.2} vs Entry: {:.2}", trade.stake, current_price, trade.entry_price);
            }
        }

        if new_wins > 0 || new_losses > 0 {
            self.wins += new_wins;
            self.losses += new_losses;
            let total = self.wins + self.losses;
            let win_rate = (self.wins as f64 / total as f64) * 100.0;
            
            info!("💰 WALLET UPDATE | Balance: ${:.2} | Win Rate: {:.1}% ({}/{})", 
                self.balance, win_rate, self.wins, total);
        }
    }
}