use anyhow::Result;
use hmac::{Hmac, Mac};
use sha2::Sha512;

type HmacSha512 = Hmac<Sha512>;

#[derive(Clone)]
pub struct ChildKey {
    pub secret: [u8; 32],
    pub chain_code: [u8; 32],
}

impl ChildKey {

    pub fn new(
        secret: [u8; 32],
        chain_code: [u8; 32],
    ) -> Self {

        Self {
            secret,
            chain_code,
        }
    }

    pub fn derive(
        &self,
        index: u32,
    ) -> Result<Self> {

        let mut mac = HmacSha512::new_from_slice(&self.chain_code)?;

        mac.update(&self.secret);
        mac.update(&index.to_be_bytes());

        let hash = mac.finalize().into_bytes();

        let mut secret = [0u8; 32];
        let mut chain = [0u8; 32];

        secret.copy_from_slice(&hash[..32]);
        chain.copy_from_slice(&hash[32..]);

        Ok(Self {
            secret,
            chain_code: chain,
        })
    }
}