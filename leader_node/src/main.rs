use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;
use tokio::io::{AsyncWriteExt, AsyncReadExt};
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
                    println!("Generated entry: timestamp={}, prev_hash={:?}, timestamp_bytes={:?}, result={:?}", timestamp, prev_hash, timestamp_bytes, result); // Debug output
                }

                tokio::time::sleep(tokio::time::Duration::from_millis(400)).await; 
            }
        });
    }

    async fn handle_connection(mut stream: TcpStream, poh: Arc<Mutex<Vec<PohEntry>>>, validators: Arc<Mutex<HashMap<String, usize>>>) {
        let peer_addr = stream.peer_addr().unwrap().to_string();
        validators.lock().await.insert(peer_addr.clone(), 0);
        println!("New validator connected: {}", peer_addr);

        loop {
            let mut length_buffer = [0; 4];
            if let Err(e) = stream.read_exact(&mut length_buffer).await {
                eprintln!("Failed to read message length: {}", e);
                break;
            }

            let message_length = u32::from_be_bytes(length_buffer) as usize;
            let mut buffer = vec![0; message_length];

            if let Err(e) = stream.read_exact(&mut buffer).await {
                eprintln!("Failed to read message data: {}", e);
                break;
            }

            match serde_json::from_slice::<Message>(&buffer) {
                Ok(Message::RetransmissionRequest(index)) => {
                    println!("Received retransmission request for index: {}", index);
                    let poh = poh.lock().await;
                    let entries_to_send: Vec<PohEntry> = poh.iter().skip(index).cloned().collect();
                    let serialized = serde_json::to_string(&Message::PoHEntries(entries_to_send)).unwrap();
                    let message_length = (serialized.len() as u32).to_be_bytes();
                    if let Err(e) = stream.write_all(&message_length).await {
                        eprintln!("Failed to send data length: {}", e);
                        continue;
                    }
                    if let Err(e) = stream.write_all(serialized.as_bytes()).await {
                        eprintln!("Failed to send data: {}", e);
                        continue;
                    }
                    println!("Sent retransmitted PoH entries from index {}: {}", index, serialized);
                },
                Ok(Message::ConsensusVote(block)) => {
                    println!("Received consensus vote for block: {:?}", block);
                },
                Ok(Message::RegisterValidator(_)) => {
                    // Ignore RegisterValidator messages
                },
                Ok(_) => {
                    // Ignore other messages for now
                },
                Err(e) => {
                    println!("Failed to deserialize message: {:?}", e);
                    continue;
                },
            }

            {
                let poh = poh.lock().await;
                let serialized = serde_json::to_string(&Message::PoHEntries(poh.clone())).unwrap();
                let message_length = (serialized.len() as u32).to_be_bytes();

                println!("Sending message of length: {} to {}", serialized.len(), peer_addr);

                if let Err(e) = stream.write_all(&message_length).await {
                    eprintln!("Failed to send data length to {}: {}", peer_addr, e);
                    break;
                }
                if let Err(e) = stream.write_all(serialized.as_bytes()).await {
                    eprintln!("Failed to send data to {}: {}", peer_addr, e);
                    break;
                }
                println!("Sent PoH entries to {}: {}", peer_addr, serialized);
            }
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await; 
        }
        println!("Validator disconnected: {}", peer_addr);
    }

    async fn start_server(&self) {
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

        let validators = validators.lock().await;
        for (addr, _) in validators.iter() {
            if let Ok(mut stream) = tokio::net::TcpStream::connect(addr).await {
                let serialized_block = serde_json::to_string(&Message::BlockProposal(block.clone())).unwrap();
                let message_length = (serialized_block.len() as u32).to_be_bytes();
                if let Err(e) = stream.write_all(&message_length).await {
                    eprintln!("Failed to send block proposal length: {}", e);
                    continue;
                }
                if let Err(e) = stream.write_all(&serialized_block.clone().into_bytes()).await {
                    eprintln!("Failed to send block proposal: {}", e);
                }
                println!("Sent block proposal to {}: {}", addr, serialized_block);
            }
        }
    }
}

#[tokio::main]
async fn main() {
    let poh_generator = PoHGenerator::new();
    poh_generator.start();
    tokio::spawn(propose_block(Arc::clone(&poh_generator.poh), Arc::clone(&poh_generator.validators)));
    poh_generator.start_server().await;
}
