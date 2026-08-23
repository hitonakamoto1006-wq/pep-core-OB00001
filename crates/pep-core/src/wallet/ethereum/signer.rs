use anyhow::Result;
use sha3::{Digest, Keccak256};

use super::{
    PrivateKey,
    Signature,
    Transaction,
};

pub struct Signer;

impl Signer {

    /* ===========================
            Sign
    =========================== */

    /// Ethereum personal_sign
    pub fn sign_message(
        key: &PrivateKey,
        message: &[u8],
    ) -> Result<Signature> {

        let hash =
            Self::message_hash(
                message,
            );

        Self::sign_hash(
            key,
            &hash,
        )

    }

    /// Sign raw 32-byte hash.
    pub fn sign_hash(
        key: &PrivateKey,
        hash: &[u8; 32],
    ) -> Result<Signature> {

        key.sign_hash(
            hash,
        )

    }

    /// Sign transaction.
    pub fn sign_transaction(
        key: &PrivateKey,
        tx: &Transaction,
    ) -> Result<Signature> {

        let hash =
            tx.signing_hash();

        Self::sign_hash(
            key,
            &hash,
        )

    }

    /* ===========================
            Verify
    =========================== */

    /// Verify message.
    ///
    /// TODO:
    /// Recover public key.
    pub fn verify_message(
        _message: &[u8],
        _signature: &Signature,
    ) -> bool {

        false

    }

    /// Verify transaction.
    pub fn verify_transaction(
        tx: &Transaction,
    ) -> bool {

        tx.is_signed()

    }

    /* ===========================
            Hash
    =========================== */

    /// Ethereum personal message hash.
    pub fn message_hash(
        message: &[u8],
    ) -> [u8; 32] {

        let prefix =
            format!(
                "\x19Ethereum Signed Message:\n{}",
                message.len(),
            );

        let mut hasher =
            Keccak256::new();

        hasher.update(
            prefix.as_bytes(),
        );

        hasher.update(
            message,
        );

        hasher.finalize().into()

    }

    /// Transaction signing hash.
    pub fn transaction_hash(
        tx: &Transaction,
    ) -> [u8; 32] {

        tx.signing_hash()

    }

}