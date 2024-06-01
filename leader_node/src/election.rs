use rand::Rng;
use std::sync::Arc;
use tokio::sync::Mutex;
use std::collections::HashMap;

#[derive(Debug)]
pub struct LeaderElection {
    stakes: Arc<Mutex<HashMap<String, u64>>>,
}

impl LeaderElection {
    pub fn new(stakes: Arc<Mutex<HashMap<String, u64>>>) -> Self {
        LeaderElection { stakes }
    }

    pub async fn elect_leader(&self) -> Option<String> {
        let stakes = self.stakes.lock().await;
        if stakes.is_empty() {
            return None;
        }

        let mut rng = rand::thread_rng();
        let weighted_choices: Vec<(&String, u64)> = stakes.iter().map(|(k, &v)| (k, v)).collect();
        let weights: Vec<u64> = weighted_choices.iter().map(|(_, weight)| *weight).collect();

        let total_weight: u64 = weights.iter().sum();
        if total_weight == 0 {
            return None;
        }

        let mut choice = rng.gen_range(0..total_weight);
        for (validator, weight) in weighted_choices {
            if choice < weight {
                return Some(validator.clone());
            }
            choice -= weight;
        }

        None
    }
}
