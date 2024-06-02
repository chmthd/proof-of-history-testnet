use serde::Serialize;
use std::sync::Arc;
use tokio::sync::Mutex;
use warp::Filter;
use std::time::Duration;

use crate::PoHGenerator;
use crate::network::GossipActivity;

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
}

struct TestMonitor {
    status: Arc<Mutex<TestStatus>>,
    poh_generator: Arc<PoHGenerator>,
    gossip_activity: Arc<Mutex<GossipActivity>>, // Track gossip activity
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

                // Proof of History
                status.proof_of_history = !self.poh_generator.poh.lock().await.is_empty();

                // Validator Connection/Registration
                status.validator_count = self.poh_generator.validators.lock().await.len();

                // Block Proposals/Validation and Generation
                let poh_entries = self.poh_generator.poh.lock().await.len();
                status.block_proposals = poh_entries; // Assuming each entry corresponds to a proposal
                status.block_generation = poh_entries; // Assuming each entry corresponds to a block generation

                // To get the actual number of block validations, you need to keep track of them separately
                // Here, we assume that block validations are counted somewhere in your logic
                status.block_validations = poh_entries; // Placeholder for actual validation count

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
            }
            tokio::time::sleep(Duration::from_secs(5)).await; // Adjust as needed.
        }
    }
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
