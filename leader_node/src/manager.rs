use validator::poh_handler::PohEntry;
use validator::transaction::Transaction; 
use crate::block::Block; 
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Validator {
    pub id: String,
    pub public_key: Vec<u8>,
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
