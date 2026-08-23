use sha2::{Digest, Sha256};

pub fn checksum8(data: &[u8]) -> u8 {
    let hash = Sha256::digest(data);

    hash[0]
}