use sha2::{Sha256, Digest};
use std::sync::Arc;
use tokio::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::net::{TcpListener, TcpStream};
use tokio::io::AsyncWriteExt;
use serde::{Serialize, Deserialize};

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
        let poh = Arc::new(Mutex::new(vec![]));
        PoHGenerator { poh }
    }

    fn start(&self) {
        let poh_clone = Arc::clone(&self.poh);
        tokio::spawn(async move {
            let mut hasher = Sha256::new();
            loop {
                let now = SystemTime::now().duration_since(UNIX_EPOCH).expect("Time went backwards");
                hasher.update(now.as_secs().to_string());
                hasher.update(now.subsec_nanos().to_string());
                let result = hasher.finalize_reset();
                let entry = PohEntry {
                    timestamp: now.as_secs(),
                    hash: result.to_vec(),
                };
                {
                    let mut poh = poh_clone.lock().await;
                    poh.push(entry);
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
        println!("Server running on 127.0.0.1:8080");

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
