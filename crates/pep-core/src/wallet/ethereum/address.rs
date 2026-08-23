use sha3::{Digest, Keccak256};

use super::PublicKey;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Address {
    bytes: [u8; 20],
}

impl Address {

    /// Create from raw bytes.
    pub fn new(
        bytes: [u8; 20],
    ) -> Self {

        Self {
            bytes,
        }

    }
    pub fn from_hex(
    hex: &str,
) -> anyhow::Result<Self> {

    let hex =
        hex.trim_start_matches("0x");

    let bytes =
        hex::decode(hex)?;

    if bytes.len() != 20 {

        anyhow::bail!(
            "invalid address length",
        );

    }

    let mut out =
        [0u8; 20];

    out.copy_from_slice(
        &bytes,
    );

    Ok(
        Self::new(out)
    )

}
    /// Create from public key.
    pub fn from_public(
        public: &PublicKey,
    ) -> Self {

        let uncompressed =
            public.uncompressed();

        // Skip SEC1 prefix (0x04)
        let hash =
            Keccak256::digest(
                &uncompressed[1..],
            );

        let mut bytes = [0u8; 20];

        bytes.copy_from_slice(
            &hash[12..],
        );

        Self {
            bytes,
        }

    }

    /// Borrow raw bytes.
    pub fn bytes(
        &self,
    ) -> &[u8; 20] {

        &self.bytes

    }

    /// Convert to array.
    pub fn to_bytes(
        &self,
    ) -> [u8; 20] {

        self.bytes

    }

    /// Hex string without prefix.
    pub fn to_hex(
        &self,
    ) -> String {

        hex::encode(self.bytes)

    }

    /// Hex string with 0x.
    pub fn to_checksum_string(
        &self,
    ) -> String {

        format!(
            "0x{}",
            self.to_hex(),
        )

    }

    /// Zero address.
    pub fn zero() -> Self {

        Self {
            bytes: [0u8; 20],
        }

    }

    /// Is zero address.
    pub fn is_zero(
        &self,
    ) -> bool {

        self.bytes
            .iter()
            .all(|v| *v == 0)

    }

}

impl std::fmt::Display for Address {

    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {

        write!(
            f,
            "{}",
            self.to_checksum_string(),
        )

    }

}

impl std::fmt::Debug for Address {

    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {

        write!(
            f,
            "{}",
            self.to_checksum_string(),
        )

    }

}