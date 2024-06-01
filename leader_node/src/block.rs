use tokio::io::AsyncWriteExt;
use validator::poh_handler::PohEntry; // Correct import
use crate::manager::Validator;
use validator::transaction::Transaction; // Correct import from validator crate
use std::sync::Arc;
use tokio::sync::Mutex;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Block {
    pub poh_entries: Vec<PohEntry>,
    pub block_hash: Vec<u8>,
    pub transactions: Vec<Transaction>,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Message {
    PoHEntries(Vec<PohEntry>),
    RetransmissionRequest(usize),
    BlockProposal(Block),
    ConsensusVote(Block),
    RegisterValidator(Validator),
    Transaction(Transaction),
}

pub async fn propose_block(
    poh: Arc<Mutex<Vec<PohEntry>>>,
    validators: Arc<Mutex<std::collections::HashMap<String, usize>>>,
    votes: Arc<Mutex<std::collections::HashMap<String, bool>>>,
    transactions: Arc<Mutex<Vec<Transaction>>>
) {
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;

        let poh_entries;
        {
            let poh = poh.lock().await;
            poh_entries = poh.clone();
        }

        let transactions_vec;
        {
            let txs = transactions.lock().await;
            transactions_vec = txs.clone();
        }

        let block = Block {
            poh_entries: poh_entries.clone(),
            block_hash: vec![0; 32],
            transactions: transactions_vec.clone(),
        };

        println!("Proposing new block");

        let validators = validators.lock().await;
        {
            let mut votes = votes.lock().await;
            votes.clear();  // Clear previous votes
        }
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
    }
}
