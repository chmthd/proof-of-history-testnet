use std::sync::Arc;
use tokio::sync::Mutex;
use validator::transaction::Transaction;

mod generator;
mod manager;
mod block;
mod network;

#[tokio::main]
async fn main() {
    let poh_generator = Arc::new(generator::PoHGenerator::new());
    let poh_generator_clone = Arc::clone(&poh_generator);

    let transactions = Arc::new(Mutex::new(Vec::<Transaction>::new())); // Initialize transactions

    poh_generator.start();
    tokio::spawn(block::propose_block(
        Arc::clone(&poh_generator_clone.poh),
        Arc::clone(&poh_generator_clone.validators),
        Arc::clone(&poh_generator_clone.votes),
        Arc::clone(&transactions),
    ));
    poh_generator_clone.start_server().await;
}
