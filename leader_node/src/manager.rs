use serde::{Serialize, Deserialize};
use validator::poh_handler::PohEntry; 
use validator::transaction::Transaction;

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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Validator {
    pub id: String,
    pub public_key: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Stake {
    pub validator: Validator,
    pub amount: u64,
}
