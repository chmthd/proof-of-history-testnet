use std::sync::{Arc, Mutex};
use tokio::time::{self, Duration};
use std::collections::HashSet;
use tokio::sync::RwLock;
use std::time::Instant;

struct MonitorState {
    current_block_number: u64,
    connected_validators: HashSet<String>,
    block_proposals: u64,
    consensus_votes: u64,
    failed_consensus_votes: u64,
    total_proposal_time: u64,
    proposal_start_time: Option<Instant>,
}

impl MonitorState {
    fn new() -> Self {
        MonitorState {
            current_block_number: 0,
            connected_validators: HashSet::new(),
            block_proposals: 0,
            consensus_votes: 0,
            failed_consensus_votes: 0,
            total_proposal_time: 0,
            proposal_start_time: None,
        }
    }

    fn add_validator(&mut self, addr: String) {
        self.connected_validators.insert(addr);
    }

    fn remove_validator(&mut self, addr: &String) {
        self.connected_validators.remove(addr);
    }

    fn increment_block_proposals(&mut self) {
        self.block_proposals += 1;
        self.current_block_number += 1;
        if let Some(start_time) = self.proposal_start_time.take() {
            let duration = start_time.elapsed().as_secs();
            self.total_proposal_time += duration;
        }
        self.proposal_start_time = Some(Instant::now());
    }

    fn increment_consensus_votes(&mut self) {
        self.consensus_votes += 1;
    }

    fn increment_failed_consensus_votes(&mut self) {
        self.failed_consensus_votes += 1;
    }

    fn average_proposal_time(&self) -> u64 {
        if self.block_proposals == 0 {
            0
        } else {
            self.total_proposal_time / self.block_proposals
        }
    }
}

async fn display_state(state: Arc<RwLock<MonitorState>>) {
    loop {
        {
            let state = state.read().await;
            println!("\n---- Monitor State ----");
            println!("Current Block Number: {}", state.current_block_number);
            println!("Connected Validators: {:?}", state.connected_validators);
            println!("Block Proposals: {}", state.block_proposals);
            println!("Consensus Votes: {}", state.consensus_votes);
            println!("Failed Consensus Votes: {}", state.failed_consensus_votes);
            println!("Average Block Proposal Time: {} seconds", state.average_proposal_time());
            println!("------------------------\n");
        }
        time::sleep(Duration::from_secs(5)).await; // Update every 5 seconds
    }
}

async fn simulate_leader(state: Arc<RwLock<MonitorState>>) {
    loop {
        {
            let mut state = state.write().await;
            state.increment_block_proposals();
        }
        time::sleep(Duration::from_secs(10)).await; // Propose a block every 10 seconds
    }
}

async fn simulate_validator(state: Arc<RwLock<MonitorState>>, addr: String) {
    {
        let mut state = state.write().await;
        state.add_validator(addr.clone());
    }

    loop {
        {
            let mut state = state.write().await;
            state.increment_consensus_votes();
        }
        time::sleep(Duration::from_secs(15)).await; // Send a consensus vote every 15 seconds
    }
}

async fn simulate_failed_votes(state: Arc<RwLock<MonitorState>>) {
    loop {
        {
            let mut state = state.write().await;
            state.increment_failed_consensus_votes();
        }
        time::sleep(Duration::from_secs(20)).await; // Fail a vote every 20 seconds
    }
}

#[tokio::main]
async fn main() {
    let state = Arc::new(RwLock::new(MonitorState::new()));

    tokio::spawn(display_state(state.clone()));

    tokio::spawn(simulate_leader(state.clone()));

    tokio::spawn(simulate_validator(state.clone(), "127.0.0.1:3001".to_string()));
    tokio::spawn(simulate_validator(state.clone(), "127.0.0.1:3002".to_string()));

    tokio::spawn(simulate_failed_votes(state.clone()));

    // Keep the main thread alive
    loop {
        time::sleep(Duration::from_secs(60)).await;
    }
}
