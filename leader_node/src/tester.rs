use serde::Serialize;
use std::sync::Arc;
use tokio::sync::Mutex;
use warp::Filter;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::PoHGenerator;
use crate::network::GossipActivity;
use validator::poh_handler::PohEntry; 

#[derive(Serialize, Default)]
struct TestStatus {
    proof_of_history: bool,
    validator_count: usize,
    block_proposals: usize,
    block_validations: usize,
    block_generation: usize,
    gossip_protocol: bool,
    transactions: usize,
    proof_of_stake: bool,
    leader_election: bool,
    total_circulating_supply: u64,
    average_block_time: f64,
    throughput: usize,
    block_count: usize,
    current_epoch: u64,
}

struct TestMonitor {
    status: Arc<Mutex<TestStatus>>,
    poh_generator: Arc<PoHGenerator>,
    gossip_activity: Arc<Mutex<GossipActivity>>,
}

impl TestMonitor {
    fn new(poh_generator: Arc<PoHGenerator>, gossip_activity: Arc<Mutex<GossipActivity>>) -> Self {
        TestMonitor {
            status: Arc::new(Mutex::new(TestStatus::default())),
            poh_generator,
            gossip_activity,
        }
    }

    async fn run_tests(&self) {
        loop {
            {
                let mut status = self.status.lock().await;
                let poh_guard = self.poh_generator.poh.lock().await;

                // Proof of History
                status.proof_of_history = !poh_guard.is_empty();

                // Validator Connection/Registration
                status.validator_count = self.poh_generator.validators.lock().await.len();

                // Block Proposals/Validation and Generation
                let poh_entries = poh_guard.len();
                status.block_proposals = poh_entries;
                status.block_generation = poh_entries;

                status.block_validations = poh_entries;

                // Gossip Protocol
                let gossip_activity = self.gossip_activity.lock().await;
                status.gossip_protocol = gossip_activity.messages_received > 0;

                // Transactions
                status.transactions = self.poh_generator.transactions.lock().await.len();

                // Proof of Stake
                status.proof_of_stake = !self.poh_generator.stakes.lock().await.is_empty();

                // Leader Election
                let current_leader = self.poh_generator.current_leader.lock().await;
                status.leader_election = current_leader.is_some();

                // Total Circulating Supply
                status.total_circulating_supply = self.poh_generator.stakes.lock().await.values().sum();

                // Calculate average block time and throughput
                status.average_block_time = calculate_average_block_time(&poh_guard);
                status.throughput = calculate_throughput(&poh_guard);

                // Block Count
                status.block_count = poh_entries;

                // Current Epoch simplified
                status.current_epoch = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() / 600; 
            }
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    }
}

fn calculate_average_block_time(poh_entries: &[PohEntry]) -> f64 {
    if poh_entries.is_empty() {
        return 0.0;
    }
    let mut total_time = 0;
    let mut count = 0;

    for window in poh_entries.windows(2) {
        if let [first, second] = &window {
            total_time += second.timestamp - first.timestamp;
            count += 1;
        }
    }

    if count == 0 {
        0.0
    } else {
        total_time as f64 / count as f64
    }
}

fn calculate_throughput(poh_entries: &[PohEntry]) -> usize {
    // Simplified example: count the number of entries per second
    poh_entries.len() 
}

pub async fn start_test_monitor(poh_generator: Arc<PoHGenerator>, gossip_activity: Arc<Mutex<GossipActivity>>) {
    let test_monitor = TestMonitor::new(poh_generator, gossip_activity);
    let status = test_monitor.status.clone();

    tokio::spawn(async move {
        test_monitor.run_tests().await;
    });

    let status_route = warp::path!("status")
        .and_then(move || {
            let status_clone = status.clone();
            async move {
                let status_guard = status_clone.lock().await;
                Ok::<_, warp::Rejection>(warp::reply::json(&*status_guard))
            }
        });

    let static_route = warp::fs::dir("./static");

    println!("Starting test monitor server on http://127.0.0.1:3030...");
    warp::serve(status_route.or(static_route))
        .run(([127, 0, 0, 1], 3030))
        .await;
}
