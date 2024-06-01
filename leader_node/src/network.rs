use serde::{Serialize, Deserialize};
use validator::poh_handler::PohEntry;
use validator::transaction::Transaction;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use crate::block::Block;
use tokio::net::TcpStream;
use std::sync::Arc;
use tokio::sync::Mutex;
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug)]
pub enum Message {
    PoHEntries(Vec<PohEntry>),
    RetransmissionRequest(usize),
    BlockProposal(Block),
    ConsensusVote(Block),
    RegisterValidator(crate::manager::Validator),
    Transaction(Transaction),
}

pub async fn handle_transaction(transaction: Transaction, transactions: Arc<Mutex<Vec<Transaction>>>) {
    println!("Handling transaction: {:?}", transaction);
    let mut txs = transactions.lock().await;
    txs.push(transaction);
}

pub async fn handle_connection(
    mut stream: TcpStream,
    poh: Arc<Mutex<Vec<PohEntry>>>,
    validators: Arc<Mutex<HashMap<String, usize>>>,
    votes: Arc<Mutex<HashMap<String, bool>>>,
    transactions: Arc<Mutex<Vec<Transaction>>>
) {
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
            Ok(Message::Transaction(transaction)) => {
                handle_transaction(transaction, Arc::clone(&transactions)).await;
            },
            Ok(_) => {
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
            Err(e) => println!("Failed to parse message: {}", e),
        }
    }
    {
        let mut validators = validators.lock().await;
        validators.remove(&peer_addr);
    }
    println!("Validator disconnected: {}", peer_addr);
}
