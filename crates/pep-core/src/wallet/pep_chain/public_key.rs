use super::PrivateKey;

use k256::{
    elliptic_curve::sec1::ToEncodedPoint,
    PublicKey as K256PublicKey,
    SecretKey,
};

use std::fmt;

#[derive(Clone, Debug)]
pub struct PublicKey {
    compressed: [u8; 33],
}

impl PublicKey {
    pub fn new(bytes: [u8; 33]) -> Self {
        Self {
            compressed: bytes,
        }
    }

    pub fn bytes(&self) -> &[u8; 33] {
        &self.compressed
    }

    pub fn compressed(&self) -> &[u8; 33] {
        &self.compressed
    }

    pub fn from_private(
        private: &PrivateKey,
    ) -> Self {

        let secret =
            SecretKey::from_slice(
                private.bytes()
            )
            .unwrap();

        let public =
            secret.public_key();

        let encoded =
            public.to_encoded_point(true);

        let mut bytes = [0u8; 33];

        bytes.copy_from_slice(
            encoded.as_bytes()
        );

        Self {
            compressed: bytes,
        }
    }

    /// 65-byte uncompressed SEC1 public key
    pub fn uncompressed(
        &self,
    ) -> [u8; 65] {

        let public =
            K256PublicKey::from_sec1_bytes(
                &self.compressed,
            )
            .unwrap();

        let encoded =
            public.to_encoded_point(false);

        let mut bytes = [0u8; 65];

        bytes.copy_from_slice(
            encoded.as_bytes(),
        );

        bytes
    }

    /// x || y (64 bytes)
    /// dùng cho Ethereum
    pub fn ethereum_bytes(
        &self,
    ) -> [u8; 64] {

        let uncompressed =
            self.uncompressed();

        let mut out = [0u8; 64];

        out.copy_from_slice(
            &uncompressed[1..],
        );

        out
    }

    pub fn to_hex(&self) -> String {

        self.compressed
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect()
    }

    pub fn from_hex(
        hex: &str,
    ) -> Option<Self> {

        if hex.len() != 66 {
            return None;
        }

        let mut bytes = [0u8; 33];

        for i in 0..33 {

            bytes[i] =
                u8::from_str_radix(
                    &hex[i * 2..i * 2 + 2],
                    16,
                )
                .ok()?;
        }

        Some(
            Self {
                compressed: bytes,
            }
        )
    }
}

impl fmt::Display for PublicKey {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {

        write!(
            f,
            "{}",
            self.to_hex()
        )
    }
}