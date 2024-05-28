use tokio::net::TcpStream;
use tokio::io::{self, AsyncReadExt};
use serde::{Serialize, Deserialize};
use std::str;

#[derive(Serialize, Deserialize, Debug)]
struct PohEntry {
    timestamp: u64,
    hash: Vec<u8>,
}

#[tokio::main]
async fn main() -> io::Result<()> {
    // Connect to the server node
    let mut stream = TcpStream::connect("127.0.0.1:8080").await?;
    println!("Connected to the server at 127.0.0.1:8080");

    let mut buffer = Vec::new();
    let mut temp_buffer = vec![0; 1024];

    loop {
        let n = stream.read(&mut temp_buffer).await?;

        if n == 0 {
            break;
        }

        buffer.extend_from_slice(&temp_buffer[..n]);
        match serde_json::from_slice::<Vec<PohEntry>>(&buffer) {
            Ok(poh_entries) => {
                println!("{:?}", poh_entries);
                buffer.clear(); 
            }
            Err(e) => {
                if e.is_eof() {
                    continue; 
                } else {
                    eprintln!("Failed to parse JSON: {:?}", e);
                    break;
                }
            }
        }
    }

    Ok(())
}
