use tokio::net::TcpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::sync::Arc;
use tokio::sync::Mutex;
use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use validator::poh_handler::PohEntry;
use validator::transaction::Transaction;
use validator::registration::Validator;
use crate::block::Block;

#[derive(Serialize, Deserialize, Debug)]
pub enum Message {
    ConsensusVote(Block),
    PoHEntries(Vec<PohEntry>),
    StakeTokens(Stake),
    RegisterValidator(Validator),
    Transaction(Transaction),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Stake {
    pub validator_id: String,
    pub amount: u64,
}

pub async fn handle_connection(
    mut stream: TcpStream,
    poh: Arc<Mutex<Vec<PohEntry>>>,
    validators: Arc<Mutex<HashMap<String, usize>>>,
    votes: Arc<Mutex<HashMap<String, bool>>>,
    transactions: Arc<Mutex<Vec<Transaction>>>,
    stakes: Arc<Mutex<HashMap<String, u64>>>,
) {
    let peer_addr = stream.peer_addr().unwrap().to_string();
    {
        let mut validators = validators.lock().await;
        validators.insert(peer_addr.clone(), 0);
        println!("New validator connected: {}", peer_addr);
    }

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
                let mut votes = votes.lock().await;
                votes.insert(peer_addr.clone(), true);
                println!("Received consensus vote for block {:?}", block);
            }
            Ok(Message::PoHEntries(_)) => {
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
            Ok(Message::StakeTokens(stake)) => {
                let mut stakes = stakes.lock().await;
                let entry = stakes.entry(stake.validator_id.clone()).or_insert(0);
                *entry += stake.amount;
                println!("Staked {} tokens for validator {}", stake.amount, stake.validator_id);
            }
            Ok(Message::RegisterValidator(validator)) => {
                println!("Received RegisterValidator message");
                // Handle validator registration
            }
            Ok(Message::Transaction(transaction)) => {
                println!("Received Transaction message: {:?}", transaction);
                let mut txs = transactions.lock().await;
                txs.push(transaction);
            }
            Err(e) => println!("Failed to parse message: {}", e),
        }
    }

    {
        let mut validators = validators.lock().await;
        validators.remove(&peer_addr);
        println!("Validator disconnected: {}", peer_addr);
    }
}

pub async fn start_server(
    poh: Arc<Mutex<Vec<PohEntry>>>,
    validators: Arc<Mutex<HashMap<String, usize>>>,
    votes: Arc<Mutex<HashMap<String, bool>>>,
    transactions: Arc<Mutex<Vec<Transaction>>>,
    stakes: Arc<Mutex<HashMap<String, u64>>>,
) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:8080").await.unwrap();
    println!("Server listening on port 8080");

    loop {
        let (socket, _) = listener.accept().await.unwrap();
        let poh = Arc::clone(&poh);
        let validators = Arc::clone(&validators);
        let votes = Arc::clone(&votes);
        let transactions = Arc::clone(&transactions);
        let stakes = Arc::clone(&stakes);

        tokio::spawn(async move {
            handle_connection(socket, poh, validators, votes, transactions, stakes).await;
        });
    }
}
