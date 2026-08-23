use crate::blockchain::hash::Hash256;
use crate::wallet::crypto;
#[derive(Clone, Debug)]
pub struct BlockHeader {

    /// Block Version
    pub version: u32,

    /// Previous Block Hash
    pub previous_hash: Hash256,

    /// Merkle Root
    pub merkle_root: Hash256,

    /// Unix Timestamp
    pub timestamp: u64,

    /// Compact Target (Bitcoin gọi là bits)
    pub bits: u32,

    /// Mining Nonce
    pub nonce: u64,
}

impl BlockHeader {

    pub fn new(
        previous_hash: Hash256,
        merkle_root: Hash256,
        timestamp: u64,
        bits: u32,
    ) -> Self {

        Self {

            version: 1,

            previous_hash,

            merkle_root,

            timestamp,

            bits,

            nonce: 0,
        }
    }

    // ==========================================
    // Serialize Header
    // ==========================================

    pub fn serialize(
        &self,
    ) -> String {

        format!(
            "{}#{}#{}#{}#{}#{}",
            self.version,
            self.previous_hash,
            self.merkle_root,
            self.timestamp,
            self.bits,
            self.nonce,
        )
    }

    pub fn deserialize(
        data: &str,
    ) -> Result<Self, String> {

        let parts: Vec<&str> =
            data.split('#').collect();

        if parts.len() != 6 {

            return Err(
                "Invalid BlockHeader".into(),
            );
        }

        Ok(

            Self {

                version:
                    parts[0]
                        .parse()
                        .map_err(|_| "Invalid version")?,

                previous_hash:
                    Hash256::from_hex(parts[1])?,

                merkle_root:
                    Hash256::from_hex(parts[2])?,

                timestamp:
                    parts[3]
                        .parse()
                        .map_err(|_| "Invalid timestamp")?,

                bits:
                    parts[4]
                        .parse()
                        .map_err(|_| "Invalid bits")?,

                nonce:
                    parts[5]
                        .parse()
                        .map_err(|_| "Invalid nonce")?,
            }
        )
    }

    // ==========================================
    // Hash
    // ==========================================

    pub fn calculate_hash(
        &self,
    ) -> Hash256 {

        Hash256::new(

            crypto::sha256_bytes(

                self.serialize()
                    .as_bytes()

            )
        )
    }

    pub fn hash(
        &self,
    ) -> Hash256 {

        self.calculate_hash()
    }

    // ==========================================
    // Getter
    // ==========================================

    pub fn nonce(
        &self,
    ) -> u64 {

        self.nonce
    }

    pub fn bits(
        &self,
    ) -> u32 {

        self.bits
    }

    pub fn set_nonce(
        &mut self,
        nonce: u64,
    ) {

        self.nonce = nonce;
    }

    pub fn set_bits(
        &mut self,
        bits: u32,
    ) {

        self.bits = bits;
    }

    pub fn set_timestamp(
        &mut self,
        timestamp: u64,
    ) {

        self.timestamp = timestamp;
    }
}