use tokio::net::TcpListener;
use tokio::io::{AsyncReadExt};
use serde::{Serialize, Deserialize};
use std::collections::{HashSet, HashMap};
use tokio::sync::Mutex;
use std::sync::Arc;

#[derive(Serialize, Deserialize, Debug)]
enum MonitorMessage {
    CurrentBlock(u64),
    BlockProposal,
    ConsensusVote,
    ValidatorUpdate(Vec<String>),
}

struct Monitor {
    current_block: u64,
    block_proposals: u64,
    consensus_votes: u64,
    connected_validators: HashSet<String>,
}

impl Monitor {
    fn new() -> Self {
        Monitor {
            current_block: 0,
            block_proposals: 0,
            consensus_votes: 0,
            connected_validators: HashSet::new(),
        }
    }

    fn display(&self) {
        print!("\x1B[2J\x1B[1;1H"); // Clear terminal
        println!("Current Block: {}, Connected Validators: {}, Block Proposals: {}, Consensus Votes: {}",
                 self.current_block, self.connected_validators.len(), self.block_proposals, self.consensus_votes);
    }

    async fn update(&mut self, message: MonitorMessage) {
        match message {
            MonitorMessage::CurrentBlock(block) => self.current_block = block,
            MonitorMessage::BlockProposal => self.block_proposals += 1,
            MonitorMessage::ConsensusVote => self.consensus_votes += 1,
            MonitorMessage::ValidatorUpdate(validators) => {
                self.connected_validators = validators.into_iter().collect();
            },
        }
        self.display();
    }
}

#[tokio::main]
async fn main() {
    let monitor = Arc::new(Mutex::new(Monitor::new()));
    let listener = TcpListener::bind("127.0.0.1:9000").await.unwrap();
    println!("Monitor running on 127.0.0.1:9000");

    loop {
        let (mut socket, _) = listener.accept().await.unwrap();
        let monitor = Arc::clone(&monitor);
        
        tokio::spawn(async move {
            let mut length_buffer = [0; 4];
            loop {
                if let Err(_) = socket.read_exact(&mut length_buffer).await {
                    break;
                }
                let message_length = u32::from_be_bytes(length_buffer) as usize;
                let mut buffer = vec![0; message_length];
                if let Err(_) = socket.read_exact(&mut buffer).await {
                    break;
                }
                let message: MonitorMessage = serde_json::from_slice(&buffer).unwrap();
                let mut monitor = monitor.lock().await;
                monitor.update(message).await;
            }
        });
    }
}
