use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use validator::poh_handler::PohEntry;
use validator::transaction::Transaction;
use sha2::{Sha256, Digest};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::net::TcpListener;
use crate::election::LeaderElection;
use crate::network::GossipActivity;

mod election;
mod network;
mod block;
mod tester;

struct PoHGenerator {
    poh: Arc<Mutex<Vec<PohEntry>>>,
    validators: Arc<Mutex<HashMap<String, usize>>>,
    votes: Arc<Mutex<HashMap<String, bool>>>,
    transactions: Arc<Mutex<Vec<Transaction>>>,
    stakes: Arc<Mutex<HashMap<String, u64>>>,
    leader_election: LeaderElection,
    current_leader: Arc<Mutex<Option<String>>>,
    parent_hash: Arc<Mutex<[u8; 32]>>,
    block_height: Arc<Mutex<u64>>,
}

impl PoHGenerator {
    fn new() -> Self {
        let stakes = Arc::new(Mutex::new(HashMap::new()));
        PoHGenerator {
            poh: Arc::new(Mutex::new(Vec::new())),
            validators: Arc::new(Mutex::new(HashMap::new())),
            votes: Arc::new(Mutex::new(HashMap::new())),
            transactions: Arc::new(Mutex::new(Vec::new())),
            stakes: Arc::clone(&stakes),
            leader_election: LeaderElection::new(Arc::clone(&stakes)),
            current_leader: Arc::new(Mutex::new(None)),
            parent_hash: Arc::new(Mutex::new([0u8; 32])),
            block_height: Arc::new(Mutex::new(0)),
        }
    }

    pub async fn generate_poh_entry(self: Arc<Self>) {
        let mut prev_hash = vec![0; 32];

        loop {
            let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();

            let mut hasher = Sha256::new();
            let timestamp_bytes = timestamp.to_be_bytes();

            hasher.update(&prev_hash);
            hasher.update(&timestamp_bytes);
            let result = hasher.finalize_reset().to_vec();

            let entry = PohEntry {
                timestamp,
                hash: result.clone(),
            };

            {
                let mut poh = self.poh.lock().await;
                poh.push(entry);
                prev_hash = result.clone();
                println!("Generated entry at timestamp {}", timestamp);
            }

            tokio::time::sleep(tokio::time::Duration::from_millis(400)).await;
        }
    }

    async fn start_leader_election(&self) {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
            if let Some(leader) = self.leader_election.elect_leader().await {
                let mut current_leader = self.current_leader.lock().await;
                *current_leader = Some(leader.clone());
                println!("New leader elected: {}", leader);
            } else {
                println!("No leader elected.");
            }
        }
    }

    pub async fn start_server(self: Arc<Self>, gossip_activity: Arc<Mutex<GossipActivity>>) {
        let listener = TcpListener::bind("127.0.0.1:8080").await.unwrap();
        println!("Server running on 127.0.0.1:8080");

        loop {
            let (socket, _) = listener.accept().await.unwrap();
            let poh_clone = Arc::clone(&self.poh);
            let validators_clone = Arc::clone(&self.validators);
            let votes_clone = Arc::clone(&self.votes);
            let transactions_clone = Arc::clone(&self.transactions);
            let stakes_clone = Arc::clone(&self.stakes);
            let gossip_activity_clone = Arc::clone(&gossip_activity);
            tokio::spawn(async move {
                crate::network::handle_connection(socket, poh_clone, validators_clone, votes_clone, transactions_clone, stakes_clone, gossip_activity_clone).await;
            });

            tokio::time::sleep(tokio::time::Duration::from_secs(3600)).await;
        }
    }
}

#[tokio::main]
async fn main() {
    let poh_generator = Arc::new(PoHGenerator::new());
    let gossip_activity = Arc::new(Mutex::new(GossipActivity::default()));

    tokio::spawn({
        let poh_generator = Arc::clone(&poh_generator);
        async move {
            let parent_hash = *poh_generator.parent_hash.lock().await;
            let block_height = *poh_generator.block_height.lock().await;
            block::propose_block(
                Arc::clone(&poh_generator.poh),
                Arc::clone(&poh_generator.validators),
                Arc::clone(&poh_generator.votes),
                Arc::clone(&poh_generator.transactions),
                parent_hash,
                block_height,
            ).await;
        }
    });

    tokio::spawn({
        let poh_generator = Arc::clone(&poh_generator);
        async move {
            poh_generator.start_leader_election().await;
        }
    });

    tokio::spawn({
        let poh_generator = Arc::clone(&poh_generator);
        let gossip_activity = Arc::clone(&gossip_activity);
        async move {
            tester::start_test_monitor(Arc::clone(&poh_generator), Arc::clone(&gossip_activity)).await;
        }
    });

    tokio::spawn({
        let poh_generator = Arc::clone(&poh_generator);
        async move {
            poh_generator.generate_poh_entry().await;
        }
    });

    poh_generator.start_server(Arc::clone(&gossip_activity)).await;
}
