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
use rand::Rng;
use rand::rngs::StdRng;
use rand::SeedableRng;
use bs58;

#[derive(Serialize, Deserialize, Debug)]
pub enum Message {
    ConsensusVote(Block),
    PoHEntries(Vec<PohEntry>),
    StakeTokens(Stake),
    RegisterValidator(Validator),
    Transaction(Transaction),
    GossipMessage(String), 
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
    gossip_activity: Arc<Mutex<GossipActivity>>, 
) {
    let mut length_buffer = [0; 4];
    if let Err(_e) = stream.read_exact(&mut length_buffer).await {
        println!("Failed to read length buffer");
        return;
    }
    let message_length = u32::from_be_bytes(length_buffer) as usize;
    let mut buffer = vec![0; message_length];
    if let Err(_e) = stream.read_exact(&mut buffer).await {
        println!("Failed to read buffer");
        return;
    }

    let validator_id = match serde_json::from_slice::<Message>(&buffer) {
        Ok(Message::RegisterValidator(validator)) => {
            let mut validators = validators.lock().await;
            let validator_id_str = bs58::encode(validator.public_key.clone()).into_string();
            validators.insert(validator_id_str.clone(), 0);
            println!("Registered validator: {}", validator_id_str);
            validator_id_str
        }
        _ => {
            println!("First message must be RegisterValidator");
            return;
        }
    };

    // Mint tokens for the new validator
    let mut rng = StdRng::from_entropy();
    let random_tokens: u64 = rng.gen_range(1000..10000); 
    {
        let mut stakes = stakes.lock().await;
        stakes.insert(validator_id.clone(), random_tokens);
        println!("Minted {} tokens for validator {}", random_tokens, validator_id);
    }

    loop {
        let mut length_buffer = [0; 4];
        if let Err(_e) = stream.read_exact(&mut length_buffer).await {
            println!("Failed to read length buffer in loop");
            break;
        }
        let message_length = u32::from_be_bytes(length_buffer) as usize;
        let mut buffer = vec![0; message_length];
        if let Err(_e) = stream.read_exact(&mut buffer).await {
            println!("Failed to read buffer in loop");
            break;
        }

        match serde_json::from_slice::<Message>(&buffer) {
            Ok(Message::ConsensusVote(block)) => {
                let mut votes = votes.lock().await;
                votes.insert(validator_id.clone(), true);
                println!("Received consensus vote for block {:?}", block);
            }
            Ok(Message::PoHEntries(_)) => {
                let poh = poh.lock().await;
                let serialized = serde_json::to_string(&Message::PoHEntries(poh.clone())).unwrap();
                let message_length = (serialized.len() as u32).to_be_bytes();
                if let Err(_e) = stream.write_all(&message_length).await {
                    println!("Failed to send length buffer in loop");
                    break;
                }
                if let Err(_e) = stream.write_all(serialized.as_bytes()).await {
                    println!("Failed to send buffer in loop");
                    break;
                }
                println!("Sent PoH entries to {}", validator_id);
            }
            Ok(Message::StakeTokens(stake)) => {
                let mut stakes = stakes.lock().await;
                let entry = stakes.entry(stake.validator_id.clone()).or_insert(0);
                *entry += stake.amount;
                println!("Staked {} tokens for validator {}", stake.amount, stake.validator_id);
            }
            Ok(Message::Transaction(transaction)) => {
                println!("Received Transaction message: {:?}", transaction);
                let mut txs = transactions.lock().await;
                txs.push(transaction);
            }
            Ok(Message::GossipMessage(_msg)) => {
                // Update gossip activity
                let mut gossip_activity = gossip_activity.lock().await;
                gossip_activity.messages_received += 1;
                println!("Received gossip message from {}", validator_id);
            }
            Ok(Message::RegisterValidator(_validator)) => {
                println!("Validator {} is already registered", validator_id);
            }
            Err(e) => println!("Failed to parse message: {}", e),
        }
    }

    {
        let mut validators = validators.lock().await;
        validators.remove(&validator_id);
        println!("Validator disconnected: {}", validator_id);
    }
}

pub async fn start_server(
    poh: Arc<Mutex<Vec<PohEntry>>>,
    validators: Arc<Mutex<HashMap<String, usize>>>,
    votes: Arc<Mutex<HashMap<String, bool>>>,
    transactions: Arc<Mutex<Vec<Transaction>>>,
    stakes: Arc<Mutex<HashMap<String, u64>>>,
    gossip_activity: Arc<Mutex<GossipActivity>>,
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
        let gossip_activity = Arc::clone(&gossip_activity);

        tokio::spawn(async move {
            handle_connection(socket, poh, validators, votes, transactions, stakes, gossip_activity).await;
        });
    }
}

#[derive(Default)]
pub struct GossipActivity {
    pub messages_sent: usize,
    pub messages_received: usize,
}
