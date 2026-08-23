use rand::{thread_rng, Rng};

#[derive(Clone)]
pub struct MasterEntropy {
    bytes: [u8; 32],
}

impl MasterEntropy {

    /// Generate 256-bit random entropy.
    pub fn new() -> Self {

        let mut rng = thread_rng();

        let mut bytes = [0u8; 32];

        rng.fill(&mut bytes);

        Self {
            bytes,
        }

    }

    /// Create from existing bytes.
    pub fn from_bytes(
        bytes: [u8; 32],
    ) -> Self {

        Self {
            bytes,
        }

    }

    /// Borrow entropy bytes.
    pub fn bytes(
        &self,
    ) -> &[u8; 32] {

        &self.bytes

    }

    /// Copy entropy bytes.
    pub fn clone_bytes(
        &self,
    ) -> [u8; 32] {

        self.bytes

    }

    /// Convert into owned bytes.
    pub fn into_bytes(
        self,
    ) -> [u8; 32] {

        self.bytes

    }

    /// Hex string.
    pub fn to_hex(
        &self,
    ) -> String {

        hex::encode(self.bytes)

    }

    /// Build from hex.
    pub fn from_hex(
        hex_str: &str,
    ) -> anyhow::Result<Self> {

        let data = hex::decode(hex_str)?;

        if data.len() != 32 {
            anyhow::bail!("entropy must be exactly 32 bytes");
        }

        let mut bytes = [0u8; 32];
        bytes.copy_from_slice(&data);

        Ok(Self { bytes })

    }

}