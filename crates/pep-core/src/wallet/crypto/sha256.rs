use sha2::{Digest, Sha256};

pub fn sha256_bytes(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(data);

    hasher.finalize().into()
}