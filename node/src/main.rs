use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;
use tokio::io::AsyncWriteExt;
use sha2::{Sha256, Digest};
use serde::{Serialize, Deserialize};
use std::sync::Arc;
use tokio::task;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Serialize, Deserialize, Debug)]
struct PohEntry {
    timestamp: u64,
    hash: Vec<u8>,
}

struct PoHGenerator {
    poh: Arc<Mutex<Vec<PohEntry>>>,
}

impl PoHGenerator {
    fn new() -> Self {
        PoHGenerator {
            poh: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn start(&self) {
        let poh_clone = Arc::clone(&self.poh);
        task::spawn(async move {
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
                }

                tokio::time::sleep(tokio::time::Duration::from_millis(400)).await; // 400-millisecond interval
            }
        });
    }

    async fn handle_connection(mut stream: TcpStream, poh: Arc<Mutex<Vec<PohEntry>>>) {
        loop {
            let poh = poh.lock().await;
            let serialized = serde_json::to_string(&*poh).unwrap();
            if let Err(e) = stream.write_all(serialized.as_bytes()).await {
                eprintln!("Failed to send data: {}", e);
                break;
            }
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await; // Send every 5 seconds
        }
    }

    async fn start_server(&self) {
        let listener = TcpListener::bind("127.0.0.1:8080").await.unwrap();

        loop {
            let (socket, _) = listener.accept().await.unwrap();
            let poh_clone = Arc::clone(&self.poh);
            tokio::spawn(async move {
                PoHGenerator::handle_connection(socket, poh_clone).await;
            });
        }
    }
}

#[tokio::main]
async fn main() {
    let poh_generator = PoHGenerator::new();
    poh_generator.start();
    poh_generator.start_server().await;
}
