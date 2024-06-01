use sha2::{Sha256, Digest};
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PohEntry {
    pub timestamp: u64,
    pub hash: Vec<u8>,
}

pub fn validate_poh_entries(poh_entries: &Vec<PohEntry>) -> Result<(), usize> {
    for i in 1..poh_entries.len() {
        let prev_entry = &poh_entries[i - 1];
        let curr_entry = &poh_entries[i];

        let mut hasher = Sha256::new();
        let timestamp_bytes = curr_entry.timestamp.to_be_bytes();

        hasher.update(&prev_entry.hash);
        hasher.update(&timestamp_bytes);
        let expected_hash = hasher.finalize_reset().to_vec();

        if curr_entry.hash != expected_hash {
            println!(
                "Validation failed at index {}: expected={:?}, got={:?}",
                i, expected_hash, curr_entry.hash
            );
            return Err(i);
        }
    }
    Ok(())
}
