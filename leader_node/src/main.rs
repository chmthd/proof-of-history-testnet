use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use sha2::{Sha256, Digest};
use serde::{Serialize, Deserialize};
use std::sync::Arc;
use tokio::task;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Serialize, Deserialize, Debug, Clone)]
struct PohEntry {
    timestamp: u64,
    hash: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Block {
    poh_entries: Vec<PohEntry>,
    block_hash: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug)]
enum Message {
    PoHEntries(Vec<PohEntry>),
    Block(Block),
    RetransmissionRequest(usize),
}

struct Leader {
    poh: Arc<Mutex<Vec<PohEntry>>>,
}

impl Leader {
    fn new() -> Self {
        Leader {
            poh: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn start(&self) {
        let poh_clone = Arc::clone(&self.poh);
        task::spawn(async move {
            let mut prev_hash = vec![0; 32]; // Initial previous hash (all zeros)
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
                println!("Generated entry: {:?}", entry); // Print the entry before moving it
                {
                    let mut poh = poh_clone.lock().await;
                    poh.push(entry.clone()); // Clone the entry before moving it
                    prev_hash = result.clone(); // Update prev_hash with the current result
                }
                tokio::time::sleep(tokio::time::Duration::from_millis(400)).await; // 400-millisecond interval
            }
        });

        // Block generation every few seconds
        let poh_clone = Arc::clone(&self.poh);
        task::spawn(async move {
            loop {
                let block_entries;
                {
                    let poh = poh_clone.lock().await;
                    block_entries = poh.clone();
                }
                if !block_entries.is_empty() {
                    let mut hasher = Sha256::new();
                    for entry in &block_entries {
                        hasher.update(&entry.hash);
                    }
                    let block_hash = hasher.finalize_reset().to_vec();
                    let block = Block {
                        poh_entries: block_entries,
                        block_hash: block_hash.clone(),
                    };
                    println!("Generated block: {:?}", block);
                }
                tokio::time::sleep(tokio::time::Duration::from_secs(10)).await; // Block generation interval
            }
        });
    }

    async fn handle_connection(mut stream: TcpStream, poh: Arc<Mutex<Vec<PohEntry>>>) {
        loop {
            {
                let poh = poh.lock().await;
                let serialized = serde_json::to_string(&Message::PoHEntries(poh.clone())).unwrap();
                if let Err(e) = stream.write_all(serialized.as_bytes()).await {
                    eprintln!("Failed to send data: {}", e);
                    break;
                }
            }
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await; // Send every 5 seconds
        }
    }

    async fn start_server(&self) {
        let listener = TcpListener::bind("127.0.0.1:8080").await.unwrap();
        println!("Server running on 127.0.0.1:8080");
        loop {
            let (socket, _) = listener.accept().await.unwrap();
            let poh_clone = Arc::clone(&self.poh);
            tokio::spawn(async move {
                Leader::handle_connection(socket, poh_clone).await;
            });
        }
    }
}

#[tokio::main]
async fn main() {
    let leader = Leader::new();
    leader.start();
    leader.start_server().await;
}
