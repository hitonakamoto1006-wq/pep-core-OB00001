use crate::wallet::PublicKey;
use ripemd::Ripemd160;
use sha2::{Digest, Sha256};
use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Address {
    text: String,
}

impl Address {
    pub fn new(text: String) -> Self {
        Self { text }
    }

    pub fn as_str(&self) -> &str {
        &self.text
    }

    pub fn from_public_key(
        public_key: &PublicKey,
    ) -> Self {
        let sha = Sha256::digest(public_key.bytes());

        let ripe = Ripemd160::digest(sha);

        let mut payload = Vec::new();

        payload.push(0x04);

        payload.extend_from_slice(&ripe);

        let checksum1 = Sha256::digest(&payload);
        let checksum2 = Sha256::digest(checksum1);

        payload.extend_from_slice(&checksum2[..4]);

        let encoded =
            bs58::encode(payload).into_string();

        Self {
            text: format!("pep4{}", encoded),
        }
    }
}

impl fmt::Display for Address {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(f, "{}", self.text)
    }
}