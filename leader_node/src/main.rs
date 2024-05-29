use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;
use tokio::io::AsyncWriteExt;
use sha2::{Sha256, Digest};
use serde::{Serialize, Deserialize};
use std::sync::Arc;
use tokio::task;
use std::time::{SystemTime, UNIX_EPOCH};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug, Clone)]
struct PohEntry {
    timestamp: u64,
    hash: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Block {
    poh_entries: Vec<PohEntry>,
    block_hash: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Validator {
    id: String,
    public_key: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug)]
enum Message {
    PoHEntries(Vec<PohEntry>),
    RetransmissionRequest(usize),
    BlockProposal(Block),
    ConsensusVote(Block),
    RegisterValidator(Validator),
}

#[derive(Serialize, Deserialize, Debug)]
enum MonitorMessage {
    CurrentBlock(u64),
    BlockProposal,
    ConsensusVote,
    ValidatorUpdate(Vec<String>),
}

struct PoHGenerator {
    poh: Arc<Mutex<Vec<PohEntry>>>,
    validators: Arc<Mutex<HashMap<String, usize>>>,
}

impl PoHGenerator {
    fn new() -> Self {
        PoHGenerator {
            poh: Arc::new(Mutex::new(Vec::new())),
            validators: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn start(self: Arc<Self>) {
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
                    println!("Generated entry at timestamp {}", timestamp);
                }

                send_monitor_message(MonitorMessage::CurrentBlock(timestamp)).await;

                tokio::time::sleep(tokio::time::Duration::from_millis(400)).await;
            }
        });
    }

    async fn handle_connection(mut stream: TcpStream, poh: Arc<Mutex<Vec<PohEntry>>>, validators: Arc<Mutex<HashMap<String, usize>>>) {
        let peer_addr = stream.peer_addr().unwrap().to_string();
        {
            let mut validators = validators.lock().await;
            validators.insert(peer_addr.clone(), 0);
            send_monitor_message(MonitorMessage::ValidatorUpdate(validators.keys().cloned().collect())).await;
        }
        println!("New validator connected: {}", peer_addr);

        loop {
            {
                let poh = poh.lock().await;
                let serialized = serde_json::to_string(&Message::PoHEntries(poh.clone())).unwrap();
                let message_length = (serialized.len() as u32).to_be_bytes();

                if let Err(_e) = stream.write_all(&message_length).await {
                    break;
                }
                if let Err(_e) = stream.write_all(serialized.as_bytes()).await {
                    break;
                }
                println!("Sent PoH entries to {}", peer_addr);
            }
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        }
        {
            let mut validators = validators.lock().await;
            validators.remove(&peer_addr);
            send_monitor_message(MonitorMessage::ValidatorUpdate(validators.keys().cloned().collect())).await;
        }
        println!("Validator disconnected: {}", peer_addr);
    }

    async fn start_server(self: Arc<Self>) {
        let listener = TcpListener::bind("127.0.0.1:8080").await.unwrap();
        println!("Server running on 127.0.0.1:8080");

        loop {
            let (socket, _) = listener.accept().await.unwrap();
            let poh_clone = Arc::clone(&self.poh);
            let validators_clone = Arc::clone(&self.validators);
            tokio::spawn(async move {
                PoHGenerator::handle_connection(socket, poh_clone, validators_clone).await;
            });
        }
    }
}

async fn propose_block(poh: Arc<Mutex<Vec<PohEntry>>>, validators: Arc<Mutex<HashMap<String, usize>>>) {
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;

        let poh_entries;
        {
            let poh = poh.lock().await;
            poh_entries = poh.clone();
        }

        let block = Block {
            poh_entries: poh_entries.clone(),
            block_hash: vec![0; 32],
        };

        println!("Proposing new block");

        let validators = validators.lock().await;
        for (addr, _) in validators.iter() {
            if let Ok(mut stream) = tokio::net::TcpStream::connect(addr).await {
                let serialized_block = serde_json::to_string(&Message::BlockProposal(block.clone())).unwrap();
                let message_length = (serialized_block.len() as u32).to_be_bytes();
                if let Err(_e) = stream.write_all(&message_length).await {
                    continue;
                }
                if let Err(_e) = stream.write_all(&serialized_block.clone().into_bytes()).await {
                    continue;
                }
                println!("Proposed block to {}", addr);
            }
        }

        send_monitor_message(MonitorMessage::BlockProposal).await;
    }
}

async fn send_monitor_message(message: MonitorMessage) {
    if let Ok(mut stream) = TcpStream::connect("127.0.0.1:9000").await {
        let serialized_message = serde_json::to_string(&message).unwrap();
        let message_length = (serialized_message.len() as u32).to_be_bytes();
        if stream.write_all(&message_length).await.is_ok() {
            stream.write_all(&serialized_message.as_bytes()).await.unwrap();
        }
    }
}

#[tokio::main]
async fn main() {
    let poh_generator = Arc::new(PoHGenerator::new());
    let poh_generator_clone = Arc::clone(&poh_generator);
    poh_generator.start();
    tokio::spawn(propose_block(Arc::clone(&poh_generator_clone.poh), Arc::clone(&poh_generator_clone.validators)));
    poh_generator_clone.start_server().await;
}
