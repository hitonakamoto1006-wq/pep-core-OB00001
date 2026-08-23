use crate::wallet::seed::Seed;
use sha2::{Digest, Sha256};

#[derive(Clone, Debug)]
pub struct PrivateKey {
    bytes: [u8; 32],
}

impl PrivateKey {

    pub fn from_seed(
        seed: &Seed,
    ) -> Self {

        let hash = Sha256::digest(
            seed.bytes(),
        );

        let mut bytes = [0u8; 32];

        bytes.copy_from_slice(
            &hash[..32],
        );

        Self {
            bytes,
        }
    }

    pub fn new(
        bytes: [u8; 32],
    ) -> Self {

        Self {
            bytes,
        }
    }

    pub fn bytes(
        &self,
    ) -> &[u8; 32] {

        &self.bytes
    }
}