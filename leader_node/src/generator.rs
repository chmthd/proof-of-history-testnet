use validator::poh_handler::PohEntry;
use validator::transaction::Transaction;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::net::TcpListener;
use sha2::{Sha256, Digest};
use std::time::{SystemTime, UNIX_EPOCH};
use std::collections::HashMap;

#[derive(Debug)]
pub struct PoHGenerator {
    pub poh: Arc<Mutex<Vec<PohEntry>>>,
    pub validators: Arc<Mutex<HashMap<String, usize>>>,
    pub votes: Arc<Mutex<HashMap<String, bool>>>,
    pub transactions: Arc<Mutex<Vec<Transaction>>>,
}

impl PoHGenerator {
    pub fn new() -> Self {
        PoHGenerator {
            poh: Arc::new(Mutex::new(Vec::new())),
            validators: Arc::new(Mutex::new(HashMap::new())),
            votes: Arc::new(Mutex::new(HashMap::new())),
            transactions: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn start(self: Arc<Self>) {
        let poh_clone = Arc::clone(&self.poh);
        tokio::spawn(async move {
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
                    let mut poh = poh_clone.lock().await;
                    poh.push(entry);
                    prev_hash = result.clone();
                    println!("Generated entry at timestamp {}", timestamp);
                }

                tokio::time::sleep(tokio::time::Duration::from_millis(400)).await;
            }
        });
    }

    pub async fn start_server(self: Arc<Self>) {
        let listener = TcpListener::bind("127.0.0.1:8080").await.unwrap();
        println!("Server running on 127.0.0.1:8080");

        loop {
            let (socket, _) = listener.accept().await.unwrap();
            let poh_clone = Arc::clone(&self.poh);
            let validators_clone = Arc::clone(&self.validators);
            let votes_clone = Arc::clone(&self.votes);
            let transactions_clone = Arc::clone(&self.transactions);
            tokio::spawn(async move {
                crate::network::handle_connection(socket, poh_clone, validators_clone, votes_clone, transactions_clone).await;
            });
        }
    }
}
