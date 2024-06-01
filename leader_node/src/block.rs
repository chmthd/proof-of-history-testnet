use std::sync::Arc;
use tokio::sync::Mutex;
use validator::poh_handler::PohEntry;
use validator::transaction::Transaction;
use validator::registration::Validator;
use crate::network::Stake;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use tokio::io::AsyncWriteExt;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Block {
    pub poh_entries: Vec<PohEntry>,
    pub block_hash: Vec<u8>,
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
) {
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;

        let poh_entries;
        let block_transactions;
        {
            let poh_guard = poh.lock().await;
            poh_entries = poh_guard.clone();
        }
        {
            let txs_guard = transactions.lock().await;
            block_transactions = txs_guard.clone();
        }

        let block = Block {
            poh_entries,
            block_hash: vec![0; 32],
            transactions: block_transactions,
        };

        println!("Proposing new block: {:?}", block);

        let serialized_block = serde_json::to_string(&Message::BlockProposal(block.clone())).unwrap();
        let message_length = (serialized_block.len() as u32).to_be_bytes();

        // Gossip the block proposal
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
