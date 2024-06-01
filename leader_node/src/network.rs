use tokio::net::TcpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::block::Message;
use validator::poh_handler::PohEntry; // Correct import

pub async fn handle_connection(mut stream: TcpStream, poh: Arc<Mutex<Vec<PohEntry>>>, validators: Arc<Mutex<std::collections::HashMap<String, usize>>>, votes: Arc<Mutex<std::collections::HashMap<String, bool>>>) {
    let peer_addr = stream.peer_addr().unwrap().to_string();
    {
        let mut validators = validators.lock().await;
        validators.insert(peer_addr.clone(), 0);
    }
    println!("New validator connected: {}", peer_addr);

    loop {
        let mut length_buffer = [0; 4];
        if let Err(_e) = stream.read_exact(&mut length_buffer).await {
            break;
        }
        let message_length = u32::from_be_bytes(length_buffer) as usize;
        let mut buffer = vec![0; message_length];
        if let Err(_e) = stream.read_exact(&mut buffer).await {
            break;
        }

        match serde_json::from_slice::<Message>(&buffer) {
            Ok(Message::ConsensusVote(block)) => {
                println!("Received consensus vote for block {:?}", block);
                let mut votes = votes.lock().await;
                votes.insert(peer_addr.clone(), true);  // Simplified: assuming all votes are true for this example
            },
            Ok(Message::PoHEntries(poh_entries)) => {
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
            },
            Ok(Message::BlockProposal(block)) => {
                println!("Received block proposal");
                // Handle block proposal here
            },
            Ok(Message::RegisterValidator(validator)) => {
                println!("Validator registered: {:?}", validator);
                // Handle validator registration here
            },
            Ok(Message::RetransmissionRequest(index)) => {
                println!("Retransmission request for index: {}", index);
                // Handle retransmission request here
            },
            Ok(Message::Transaction(transaction)) => {
                println!("Received transaction: {:?}", transaction);
                // Handle transaction here
            },
            Err(e) => println!("Failed to parse message: {}", e),
        }
    }
    {
        let mut validators = validators.lock().await;
        validators.remove(&peer_addr);
    }
    println!("Validator disconnected: {}", peer_addr);
}
