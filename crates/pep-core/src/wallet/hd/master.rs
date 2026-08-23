use anyhow::Result;
use hmac::{Hmac, Mac};
use sha2::Sha512;

use crate::wallet::seed::Seed;

use super::{
    child::ChildKey,
    path::DerivationPath,
};

type HmacSha512 = Hmac<Sha512>;

#[derive(Clone)]
pub struct MasterKey {
    secret: [u8; 32],
    chain_code: [u8; 32],
}

impl MasterKey {

    /// Create master key from PEP seed.
    pub fn new(seed: &Seed) -> Result<Self> {

        let mut mac =
            HmacSha512::new_from_slice(
                b"Bitcoin seed",
            )?;

        // Seed của project chỉ có bytes()
        mac.update(seed.bytes());

        let hash =
            mac.finalize().into_bytes();

        let mut secret = [0u8; 32];
        let mut chain_code = [0u8; 32];

        secret.copy_from_slice(&hash[..32]);
        chain_code.copy_from_slice(&hash[32..]);

        Ok(Self {
            secret,
            chain_code,
        })
    }

    pub fn secret(
        &self,
    ) -> &[u8; 32] {
        &self.secret
    }

    pub fn chain_code(
        &self,
    ) -> &[u8; 32] {
        &self.chain_code
    }

    pub fn derive(
        &self,
        path: &DerivationPath,
    ) -> Result<[u8; 32]> {

        let mut child =
            ChildKey::new(
                self.secret,
                self.chain_code,
            );

        for index in path.indices() {

            child =
                child.derive(*index)?;

        }

        Ok(child.secret)
    }

    /// m/44'/60'/account'/0/index
    pub fn derive_evm(
        &self,
        account: u32,
        index: u32,
    ) -> Result<[u8; 32]> {

        self.derive(
            &DerivationPath::evm(
                account,
                index,
            )
        )
    }

    /// Native PEP path
    pub fn derive_pep(
        &self,
        account: u32,
        index: u32,
    ) -> Result<[u8; 32]> {

        self.derive(
            &DerivationPath::pep(
                account,
                index,
            )
        )
    }
}