use tokio::net::TcpStream;
use tokio::io::{self, AsyncReadExt, AsyncWriteExt};
use serde::{Serialize, Deserialize};
use sha2::{Sha256, Digest};
use rand::seq::IteratorRandom;
use rand::rngs::StdRng;
use rand::SeedableRng;
use rand::Rng;
use validator::poh_handler::{PohEntry, validate_poh_entries};
use validator::transaction::{Transaction, create_transaction};

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Block {
    poh_entries: Vec<PohEntry>,
    block_hash: Vec<u8>,
    transactions: Vec<Transaction>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Validator {
    id: String,
    public_key: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug)]
enum Message {
    PoHEntries(Vec<PohEntry>),
    RetransmissionRequest(usize),
    BlockProposal(Block),
    ConsensusVote(Block),
    RegisterValidator(Validator),
    Transaction(Transaction),
    GossipMessage(String), // Example of a simple gossip message
}

async fn gossip_message(message: &Message, peer_addrs: &Vec<String>) {
    let serialized_message = serde_json::to_string(&message).unwrap();
    let message_length = (serialized_message.len() as u32).to_be_bytes();
    let mut rng = StdRng::from_entropy();

    for addr in peer_addrs.iter().choose_multiple(&mut rng, peer_addrs.len() / 2) {
        if let Ok(mut stream) = TcpStream::connect(addr).await {
            if let Err(_) = stream.write_all(&message_length).await {
                continue;
            }
            if let Err(_) = stream.write_all(serialized_message.as_bytes()).await {
                continue;
            }
            println!("Gossiped message to {}", addr);
        }
    }
}

#[tokio::main]
async fn main() -> io::Result<()> {
    let mut rng = rand::thread_rng();
    let validator_id = format!("validator_{}", rng.gen::<u32>());
    let mut stream = TcpStream::connect("127.0.0.1:8080").await?;
    let peer_addrs = vec!["127.0.0.1:8081".to_string(), "127.0.0.1:8082".to_string()]; // Add peers' addresses here

    let register_message = Message::RegisterValidator(Validator {
        id: validator_id.clone(),
        public_key: vec![1, 2, 3, 4, 5],
    });
    let serialized_register = serde_json::to_string(&register_message).unwrap();
    stream.write_all(&(serialized_register.len() as u32).to_be_bytes()).await?;
    stream.write_all(serialized_register.as_bytes()).await?;
    println!("Registered validator with ID {}", validator_id);

    let mut transactions = Vec::new();

    // Create a sample transaction and send it to the leader node
    let sample_transaction = create_transaction(
        validator_id.clone(),
        "recipient_1".to_string(),
        100,
        vec![1, 2, 3, 4]
    );
    let transaction_message = Message::Transaction(sample_transaction.clone());
    let serialized_transaction = serde_json::to_string(&transaction_message).unwrap();
    stream.write_all(&(serialized_transaction.len() as u32).to_be_bytes()).await?;
    stream.write_all(serialized_transaction.as_bytes()).await?;
    println!("Sent transaction to leader");

    loop {
        let mut length_buffer = [0; 4];
        if stream.read_exact(&mut length_buffer).await.is_err() {
            break;
        }

        let message_length = u32::from_be_bytes(length_buffer) as usize;
        let mut buffer = vec![0; message_length];
        if stream.read_exact(&mut buffer).await.is_err() {
            break;
        }

        match serde_json::from_slice::<Message>(&buffer) {
            Ok(Message::PoHEntries(poh_entries)) => {
                match validate_poh_entries(&poh_entries) {
                    Ok(_) => println!("Valid PoH entries received"),
                    Err(index) => {
                        println!("Invalid PoH entry at index {}, requesting retransmission", index);
                        let request = Message::RetransmissionRequest(index);
                        let serialized_request = serde_json::to_string(&request).unwrap();
                        stream.write_all(&(serialized_request.len() as u32).to_be_bytes()).await?;
                        stream.write_all(serialized_request.as_bytes()).await?;
                    }
                }
            },
            Ok(Message::BlockProposal(block)) => {
                println!("Received block proposal");
                gossip_message(&Message::BlockProposal(block.clone()), &peer_addrs).await;
                let vote = Message::ConsensusVote(block.clone());
                let serialized_vote = serde_json::to_string(&vote).unwrap();
                stream.write_all(&(serialized_vote.len() as u32).to_be_bytes()).await?;
                stream.write_all(serialized_vote.as_bytes()).await?;
                println!("Sent consensus vote");
            },
            Ok(Message::ConsensusVote(block)) => {
                println!("Received consensus vote");
                gossip_message(&Message::ConsensusVote(block), &peer_addrs).await;
            },
            Ok(Message::Transaction(transaction)) => {
                println!("Received transaction: {:?}", transaction);
                if transaction.validate() {
                    transactions.push(transaction.clone());
                    gossip_message(&Message::Transaction(transaction), &peer_addrs).await;
                } else {
                    println!("Invalid transaction received");
                }
            },
            _ => {},
        }
    }

    Ok(())
}
