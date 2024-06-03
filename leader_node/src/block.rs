use std::sync::Arc;
use tokio::sync::Mutex;
use validator::poh_handler::PohEntry;
use validator::transaction::Transaction;
use validator::registration::Validator;
use crate::network::Stake;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use tokio::io::AsyncWriteExt;
use std::time::{SystemTime, UNIX_EPOCH};
use sha2::{Sha256, Digest}; 

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Block {
    pub parent_hash: String,
    pub block_hash: String,
    pub block_height: u64,
    pub timestamp: u64,
    pub transactions: Vec<Transaction>,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Message {
    PoHEntries(Vec<PohEntry>),
    BlockProposal(Block),
    ConsensusVote(Block),
    RegisterValidator(Validator),
    StakeTokens(Stake),
    Transaction(Transaction),
}

pub async fn propose_block(
    poh: Arc<Mutex<Vec<PohEntry>>>,
    validators: Arc<Mutex<HashMap<String, usize>>>,
    _votes: Arc<Mutex<HashMap<String, bool>>>,
    transactions: Arc<Mutex<Vec<Transaction>>>,
    parent_hash: Arc<Mutex<[u8; 32]>>,
    block_height: Arc<Mutex<u64>>,
) {
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;

        let block_transactions;
        {
            let txs_guard = transactions.lock().await;
            block_transactions = txs_guard.clone();
        }

        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();

        let current_parent_hash = *parent_hash.lock().await;
        let current_block_height = *block_height.lock().await;

        let block_hash = generate_block_hash(&current_parent_hash, current_block_height, timestamp, &block_transactions);

        let block = Block {
            parent_hash: hex::encode(current_parent_hash),
            block_hash: hex::encode(block_hash),
            block_height: current_block_height + 1,
            timestamp,
            transactions: block_transactions,
        };

        {
            let mut parent_hash_lock = parent_hash.lock().await;
            *parent_hash_lock = block_hash;
            let mut block_height_lock = block_height.lock().await;
            *block_height_lock += 1;
        }

        println!("Proposing new block: {:?}", block);

        let serialized_block = serde_json::to_string(&Message::BlockProposal(block.clone())).unwrap();
        let message_length = (serialized_block.len() as u32).to_be_bytes();

        for validator in validators.lock().await.keys() {
            if let Ok(mut stream) = tokio::net::TcpStream::connect(validator).await {
                if stream.write_all(&message_length).await.is_ok() {
                    if stream.write_all(serialized_block.as_bytes()).await.is_ok() {
                        println!("Sent block proposal to {}", validator);
                    }
                }
            }
        }
    }
}

fn generate_block_hash(
    parent_hash: &[u8],
    block_height: u64,
    timestamp: u64,
    transactions: &[Transaction],
) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(parent_hash);
    hasher.update(&block_height.to_be_bytes());
    hasher.update(&timestamp.to_be_bytes());
    for tx in transactions {
        hasher.update(&serde_json::to_vec(tx).unwrap());
    }
    let result = hasher.finalize();
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&result);
    hash
}
