use sha2::{Sha256, Digest};
use std::time::{SystemTime, UNIX_EPOCH};
use std::thread;
use std::sync::{Arc, Mutex};

struct PoHGenerator {
    poh: Arc<Mutex<Vec<Vec<u8>>>>,
}

impl PoHGenerator {
    fn new() -> Self {
        let poh = Arc::new(Mutex::new(vec![]));
        PoHGenerator { poh }
    }

    fn start(&self) {
        let poh_clone = Arc::clone(&self.poh);
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
    }

    fn print_poh(&self) {
        let poh = self.poh.lock().unwrap();
        println!("{:?}", poh);
    }
}

fn main() {
    let poh_generator = PoHGenerator::new();
    poh_generator.start();

    loop {
        poh_generator.print_poh();
        thread::sleep(std::time::Duration::from_secs(5)); // Print every 5 seconds
    }
}
