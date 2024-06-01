use serde::{Serialize, Deserialize};

impl Transaction {
    pub fn validate(&self) -> bool {
        !self.signature.is_empty()
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Transaction {
    pub sender: String,
    pub receiver: String,
    pub amount: u64,
    pub signature: Vec<u8>,
}

pub fn create_transaction(sender: String, receiver: String, amount: u64, signature: Vec<u8>) -> Transaction {
    Transaction {
        sender,
        receiver,
        amount,
        signature,
    }
}



