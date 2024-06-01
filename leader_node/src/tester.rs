use serde::Serialize;
use std::sync::Arc;
use tokio::sync::Mutex;
use warp::Filter;
use std::time::Duration;

use crate::{PoHGenerator};

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
}

struct TestMonitor {
    status: Arc<Mutex<TestStatus>>,
    poh_generator: Arc<PoHGenerator>,
}

impl TestMonitor {
    fn new(poh_generator: Arc<PoHGenerator>) -> Self {
        TestMonitor {
            status: Arc::new(Mutex::new(TestStatus::default())),
            poh_generator,
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
                status.block_validations = poh_entries; // Placeholder for actual validation count

                // Gossip Protocol
                status.gossip_protocol = true; // Placeholder for actual gossip status

                // Transactions
                status.transactions = self.poh_generator.transactions.lock().await.len();

                // Proof of Stake
                status.proof_of_stake = !self.poh_generator.stakes.lock().await.is_empty();

                // Leader Election
                status.leader_election = true; // Placeholder for actual leader election status
            }
            tokio::time::sleep(Duration::from_secs(5)).await; // Adjust as needed.
        }
    }
}

pub async fn start_test_monitor(poh_generator: Arc<PoHGenerator>) {
    let test_monitor = TestMonitor::new(poh_generator);
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
