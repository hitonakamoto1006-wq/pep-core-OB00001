use anyhow::Result;
use k256::{
    elliptic_curve::sec1::ToEncodedPoint,
    PublicKey as K256PublicKey,
};

use super::{
    Address,
    PrivateKey,
};

#[derive(Clone)]
pub struct PublicKey {
    inner: K256PublicKey,
}

impl PublicKey {

    /// Create from k256 public key.
    pub fn new(
        inner: K256PublicKey,
    ) -> Self {

        Self {
            inner,
        }

    }

    /// Derive from private key.
    pub fn from_private(
        private: &PrivateKey,
    ) -> Self {

        Self {
            inner: private.inner().public_key(),
        }

    }

    /// Get inner key.
    pub fn inner(
        &self,
    ) -> &K256PublicKey {

        &self.inner

    }

    /// SEC1 compressed (33 bytes)
    pub fn compressed(
        &self,
    ) -> [u8; 33] {

        self.inner
            .to_encoded_point(true)
            .as_bytes()
            .try_into()
            .unwrap()

    }

    /// SEC1 uncompressed (65 bytes)
    pub fn uncompressed(
        &self,
    ) -> [u8; 65] {

        self.inner
            .to_encoded_point(false)
            .as_bytes()
            .try_into()
            .unwrap()

    }

    /// Raw bytes (compressed)
    pub fn to_bytes(
        &self,
    ) -> [u8; 33] {

        self.compressed()

    }

    /// Hex (compressed)
    pub fn to_hex(
        &self,
    ) -> String {

        hex::encode(
            self.compressed(),
        )

    }

    /// Hex (uncompressed)
    pub fn to_hex_uncompressed(
        &self,
    ) -> String {

        hex::encode(
            self.uncompressed(),
        )

    }

    /// Ethereum address.
    pub fn address(
        &self,
    ) -> Address {

        Address::from_public(
            self,
        )

    }

    /// Compare.
    pub fn equals(
        &self,
        other: &Self,
    ) -> bool {

        self.compressed()
            ==
        other.compressed()

    }

}