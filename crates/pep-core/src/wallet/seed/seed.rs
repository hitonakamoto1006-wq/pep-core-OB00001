use crate::wallet::crypto;
use crate::wallet::mnemonic::Mnemonic;

use argon2::{
    password_hash::SaltString,
    Algorithm,
    Argon2,
    Params,
    Version,
};

pub struct Seed {
    bytes: [u8; 64],
}

impl Seed {
    pub fn from_mnemonic(
        mnemonic: &Mnemonic,
        passphrase: &str,
    ) -> Self {
        // Mnemonic phrase
        let phrase = mnemonic.phrase();

        // Salt = SHA256("PEP39" + passphrase)
        let salt_input = format!("PEP39{}", passphrase);
        let salt_hash = crypto::sha256_bytes(salt_input.as_bytes());

        let salt = SaltString::encode_b64(&salt_hash)
            .unwrap();

        // Argon2id parameters
        let params = Params::new(
            64 * 1024, // 64 MB
            3,         // iterations
            1,         // parallelism
            Some(64),  // output length
        )
        .unwrap();

        let argon2 = Argon2::new(
            Algorithm::Argon2id,
            Version::V0x13,
            params,
        );

        // Generate seed
        let mut seed = [0u8; 64];

        argon2
            .hash_password_into(
                phrase.as_bytes(),
                salt.as_salt().as_str().as_bytes(),
                &mut seed,
            )
            .unwrap();

        Self::new(seed)
    }

    pub fn new(bytes: [u8; 64]) -> Self {
        Self { bytes }
    }

    pub fn bytes(&self) -> &[u8; 64] {
        &self.bytes
    }
}