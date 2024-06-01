use tokio::net::TcpStream;
use tokio::io::AsyncWriteExt;
use rand::seq::IteratorRandom;
use rand::rngs::StdRng;
use rand::SeedableRng;
use crate::block::Message;

pub async fn gossip_message(message: &Message, peer_addrs: &Vec<String>) {
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
