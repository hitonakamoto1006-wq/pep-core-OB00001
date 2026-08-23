use anyhow::{anyhow, Result};

use k256::{
    ecdsa::{
        signature::hazmat::PrehashSigner,
        Signature as K256Signature,
        SigningKey,
    },
    SecretKey,
};

use rand::rngs::OsRng;
use zeroize::Zeroize;

use super::{
    PublicKey,
    Signature,
    Transaction,
};

#[derive(Clone)]
pub struct PrivateKey {
    inner: SecretKey,
}

impl PrivateKey {

    /// Create from raw secret.
    pub fn from_bytes(
        bytes: [u8; 32],
    ) -> Result<Self> {

        let inner =
            SecretKey::from_slice(&bytes)
                .map_err(|_| anyhow!("invalid private key"))?;

        Ok(Self {
            inner,
        })

    }

    /// Generate random key.
    pub fn generate() -> Self {

        Self {
            inner: SecretKey::random(&mut OsRng),
        }

    }

    /// Raw bytes.
    pub fn to_bytes(
        &self,
    ) -> [u8; 32] {

        self.inner.to_bytes().into()

    }

    /// Hex.
    pub fn to_hex(
        &self,
    ) -> String {

        hex::encode(
            self.to_bytes(),
        )

    }

    /// Borrow inner key.
    pub fn inner(
        &self,
    ) -> &SecretKey {

        &self.inner

    }

    /// Public key.
    pub fn public_key(
        &self,
    ) -> PublicKey {

        PublicKey::from_private(self)

    }

    /// Sign 32-byte hash.
    pub fn sign_hash(
    &self,
    hash: &[u8; 32],
) -> Result<Signature> {

    let signing =
        SigningKey::from(
            self.inner.clone(),
        );

    let (sig, recid) =
        signing
            .sign_prehash_recoverable(hash)
            .map_err(|e| anyhow!(e.to_string()))?;

    let bytes =
        sig.to_bytes();

    let mut r = [0u8; 32];
    let mut s = [0u8; 32];

    r.copy_from_slice(
        &bytes[..32],
    );

    s.copy_from_slice(
        &bytes[32..],
    );

    Ok(
        Signature::new(
            r,
            s,
            recid.to_byte(),
        )
    )

}

    /// Transaction signing.
    pub fn sign_transaction(
        &self,
        tx: &Transaction,
    ) -> Result<Signature> {

        let hash =
        tx.signing_hash();

        self.sign_hash(
            &hash,
        )

    }

    /// Zeroize key material.
    pub fn zeroize(
        &mut self,
    ) {

        self.inner
            .to_bytes()
            .zeroize();

    }

}