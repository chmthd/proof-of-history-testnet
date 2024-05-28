use tokio::net::TcpStream;
use tokio::io::{self, AsyncReadExt, AsyncWriteExt};
use serde::{Serialize, Deserialize};
use sha2::{Sha256, Digest};

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

#[derive(Serialize, Deserialize, Debug)]
enum Message {
    PoHEntries(Vec<PohEntry>),
    RetransmissionRequest(usize),
    BlockProposal(Block),
    ConsensusVote(Block),
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

        println!(
            "Client Validation: prev_hash={:?}, timestamp_bytes={:?}, expected={:?}, got={:?}", 
            prev_entry.hash, timestamp_bytes, expected_hash, curr_entry.hash
        ); 

        if curr_entry.hash != expected_hash {
            println!(
                "Validation failed at index {}: prev_hash={:?}, timestamp_bytes={:?}, expected={:?}, got={:?}", 
                i, prev_entry.hash, timestamp_bytes, expected_hash, curr_entry.hash
            ); 
            return Err(i);
        } else {
            println!("Validation succeeded at index {}: hash={:?}", i, curr_entry.hash); 
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() -> io::Result<()> {
    let mut stream = TcpStream::connect("127.0.0.1:8080").await?;
    println!("Connected to the server at 127.0.0.1:8080");

    loop {
        let mut length_buffer = [0; 4];
        if stream.read_exact(&mut length_buffer).await.is_err() {
            eprintln!("Failed to read message length");
            break;
        }

        let message_length = u32::from_be_bytes(length_buffer) as usize;
        let mut buffer = vec![0; message_length];

        if stream.read_exact(&mut buffer).await.is_err() {
            eprintln!("Failed to read message data");
            break;
        }

        match serde_json::from_slice::<Message>(&buffer) {
            Ok(Message::PoHEntries(poh_entries)) => {
                println!("Received PoH entries: {:?}", poh_entries); 
                match validate_poh_entries(&poh_entries) {
                    Ok(_) => {
                        println!("Valid PoH entries: {:?}", poh_entries);
                    },
                    Err(index) => {
                        println!("Invalid PoH entry at index {}! Requesting retransmission.", index);
                        let request = Message::RetransmissionRequest(index);
                        let serialized_request = serde_json::to_string(&request).unwrap();
                        stream.write_all(&(serialized_request.len() as u32).to_be_bytes()).await?;
                        stream.write_all(serialized_request.as_bytes()).await?;
                    }
                }
            },
            Ok(Message::BlockProposal(block)) => {
                println!("Received Block Proposal: {:?}", block); 
                let vote = Message::ConsensusVote(block.clone());
                let serialized_vote = serde_json::to_string(&vote).unwrap();
                stream.write_all(&(serialized_vote.len() as u32).to_be_bytes()).await?;
                stream.write_all(serialized_vote.as_bytes()).await?;
            },
            Ok(Message::ConsensusVote(block)) => {
                println!("Received Consensus Vote for Block: {:?}", block);
            },
            Ok(Message::RetransmissionRequest(_)) => {
                continue;
            },
            Err(e) => {
                println!("Failed to deserialize message: {:?}", e);
                continue;
            },
        }
    }

    Ok(())
}
