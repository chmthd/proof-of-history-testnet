use sha2::{Sha256, Digest};
use std::time::{SystemTime, UNIX_EPOCH};
use std::thread;
use std::sync::{Arc, Mutex};

fn main() {
    let poh = Arc::new(Mutex::new(vec![]));
    let poh_clone = Arc::clone(&poh);

    thread::spawn(move || {
        let mut hasher = Sha256::new();
        loop {
            let now = SystemTime::now().duration_since(UNIX_EPOCH).expect("Time went backwards");
            hasher.update(now.as_secs().to_string());
            hasher.update(now.subsec_nanos().to_string());
            let result = hasher.finalize_reset();
            let mut poh = poh_clone.lock().unwrap();
            poh.push(result.to_vec());
            thread::sleep(std::time::Duration::from_millis(400)); // 400-millisecond interval
        }
    });

    loop {
        {
            let poh = poh.lock().unwrap();
            println!("{:?}", poh);
        }
        thread::sleep(std::time::Duration::from_secs(5)); // Print every 5 seconds
    }
}
