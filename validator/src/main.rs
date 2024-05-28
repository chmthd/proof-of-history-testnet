use tokio::net::TcpStream;
use tokio::io::{self, AsyncReadExt, AsyncWriteExt};
use serde::{Serialize, Deserialize};
use sha2::{Sha256, Digest};
use rand::seq::IteratorRandom;
use rand::rngs::StdRng;
use rand::SeedableRng;

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

fn validate_poh_entries(poh_entries: &Vec<PohEntry>) -> Result<(), usize> {
    for i in 1..poh_entries.len() {
        let prev_entry = &poh_entries[i - 1];
        let curr_entry = &poh_entries[i];

        let mut hasher = Sha256::new();
        let timestamp_bytes = curr_entry.timestamp.to_be_bytes();

        hasher.update(&prev_entry.hash);
        hasher.update(&timestamp_bytes);
        let expected_hash = hasher.finalize_reset().to_vec();

        if curr_entry.hash != expected_hash {
            return Err(i);
        }
    }
    Ok(())
}

async fn gossip_message(message: &Message, peer_addrs: &Vec<String>) {
    let serialized_message = serde_json::to_string(&message).unwrap();
    let message_length = (serialized_message.len() as u32).to_be_bytes();
    let mut rng = StdRng::from_entropy();

    for addr in peer_addrs.iter().choose_multiple(&mut rng, peer_addrs.len() / 2) {
        if let Ok(mut stream) = TcpStream::connect(addr).await {
            if let Err(_) = stream.write_all(&message_length).await {
                continue;
            }
            if let Err(_) = stream.write_all(serialized_message.as_bytes()).await {
                continue;
            }
        }
    }
}

#[tokio::main]
async fn main() -> io::Result<()> {
    let mut stream = TcpStream::connect("127.0.0.1:8080").await?;
    let peer_addrs = vec!["127.0.0.1:8081".to_string(), "127.0.0.1:8082".to_string()]; // Add peers' addresses here

    let register_message = Message::RegisterValidator(Validator {
        id: "validator_1".to_string(),
        public_key: vec![1, 2, 3, 4, 5],
    });
    let serialized_register = serde_json::to_string(&register_message).unwrap();
    stream.write_all(&(serialized_register.len() as u32).to_be_bytes()).await?;
    stream.write_all(serialized_register.as_bytes()).await?;

    loop {
        let mut length_buffer = [0; 4];
        if stream.read_exact(&mut length_buffer).await.is_err() {
            break;
        }

        let message_length = u32::from_be_bytes(length_buffer) as usize;
        let mut buffer = vec![0; message_length];

        if stream.read_exact(&mut buffer).await.is_err() {
            break;
        }

        match serde_json::from_slice::<Message>(&buffer) {
            Ok(Message::PoHEntries(poh_entries)) => {
                if validate_poh_entries(&poh_entries).is_ok() {
                    gossip_message(&Message::PoHEntries(poh_entries), &peer_addrs).await;
                } else {
                    let request = Message::RetransmissionRequest(poh_entries.len());
                    let serialized_request = serde_json::to_string(&request).unwrap();
                    stream.write_all(&(serialized_request.len() as u32).to_be_bytes()).await?;
                    stream.write_all(serialized_request.as_bytes()).await?;
                }
            },
            Ok(Message::BlockProposal(block)) => {
                let vote = Message::ConsensusVote(block.clone());
                gossip_message(&Message::BlockProposal(block), &peer_addrs).await;
                let serialized_vote = serde_json::to_string(&vote).unwrap();
                stream.write_all(&(serialized_vote.len() as u32).to_be_bytes()).await?;
                stream.write_all(serialized_vote.as_bytes()).await?;
            },
            Ok(Message::ConsensusVote(block)) => {
                gossip_message(&Message::ConsensusVote(block), &peer_addrs).await;
            },
            _ => {},
        }
    }

    Ok(())
}
