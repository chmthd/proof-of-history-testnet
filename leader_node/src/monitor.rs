use tokio::net::TcpListener;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use tokio::sync::Mutex;
use std::sync::Arc;

#[derive(Serialize, Deserialize, Debug)]
enum MonitorMessage {
    NewEntry,
    ValidatorConnected,
    ValidatorDisconnected,
    BlockProposal,
    ConsensusVote,
}

#[derive(Debug)]
struct MonitorState {
    current_block_number: u64,
    connected_validators: usize,
    block_proposals: usize,
    consensus_votes: usize,
}

#[tokio::main]
async fn main() {
    let state = Arc::new(Mutex::new(MonitorState {
        current_block_number: 0,
        connected_validators: 0,
        block_proposals: 0,
        consensus_votes: 0,
    }));

    let listener = TcpListener::bind("127.0.0.1:9000").await.unwrap();
    println!("Monitor server running on 127.0.0.1:9000");

    loop {
        let (mut socket, _) = listener.accept().await.unwrap();
        let state = Arc::clone(&state);

        tokio::spawn(async move {
            loop {
                let mut length_buffer = [0; 4];
                if let Err(_) = socket.read_exact(&mut length_buffer).await {
                    break;
                }

                let message_length = u32::from_be_bytes(length_buffer) as usize;
                let mut buffer = vec![0; message_length];

                if let Err(_) = socket.read_exact(&mut buffer).await {
                    break;
                }

                if let Ok(message) = serde_json::from_slice::<MonitorMessage>(&buffer) {
                    let mut state = state.lock().await;
                    match message {
                        MonitorMessage::NewEntry => {
                            state.current_block_number += 1;
                        }
                        MonitorMessage::ValidatorConnected => {
                            state.connected_validators += 1;
                        }
                        MonitorMessage::ValidatorDisconnected => {
                            if state.connected_validators > 0 {
                                state.connected_validators -= 1;
                            }
                        }
                        MonitorMessage::BlockProposal => {
                            state.block_proposals += 1;
                        }
                        MonitorMessage::ConsensusVote => {
                            state.consensus_votes += 1;
                        }
                    }
                    println!(
                        "Current Block: {}, Connected Validators: {}, Block Proposals: {}, Consensus Votes: {}",
                        state.current_block_number,
                        state.connected_validators,
                        state.block_proposals,
                        state.consensus_votes
                    );
                }
            }
        });
    }
}
