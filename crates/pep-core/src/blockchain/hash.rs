use primitive_types::U256;
use std::cmp::Ordering;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub struct Hash256 {
    bytes: [u8; 32],
}

impl Hash256 {

    pub fn new(
        bytes: [u8; 32],
    ) -> Self {

        Self {
            bytes,
        }
    }

    pub fn zero() -> Self {

        Self {
            bytes: [0; 32],
        }
    }

    pub fn bytes(
        &self,
    ) -> &[u8; 32] {

        &self.bytes
    }

    pub fn as_bytes(
        &self,
    ) -> &[u8] {

        &self.bytes
    }

    pub fn to_vec(
        &self,
    ) -> Vec<u8> {

        self.bytes.to_vec()
    }

    pub fn is_zero(
        &self,
    ) -> bool {

        self.bytes == [0u8; 32]
    }

    pub fn to_hex(
        &self,
    ) -> String {

        hex::encode(self.bytes)
    }

    pub fn to_u256(
        &self,
    ) -> U256 {

        U256::from_big_endian(
            &self.bytes,
        )
    }

    pub fn from_hex(
        hex_str: &str,
    ) -> Result<Self, String> {

        let bytes =
            hex::decode(hex_str)
                .map_err(|_| "Invalid hash")?;

        if bytes.len() != 32 {

            return Err(
                "Invalid hash length".into(),
            );
        }

        let mut array =
            [0u8; 32];

        array.copy_from_slice(
            &bytes,
        );

        Ok(
            Self::new(array)
        )
    }
}

impl From<[u8; 32]> for Hash256 {

    fn from(
        bytes: [u8; 32],
    ) -> Self {

        Self::new(bytes)
    }
}

impl AsRef<[u8]> for Hash256 {

    fn as_ref(
        &self,
    ) -> &[u8] {

        &self.bytes
    }
}

impl PartialOrd for Hash256 {

    fn partial_cmp(
        &self,
        other: &Self,
    ) -> Option<Ordering> {

        Some(
            self.cmp(other)
        )
    }
}

impl Ord for Hash256 {

    fn cmp(
        &self,
        other: &Self,
    ) -> Ordering {

        self.bytes.cmp(
            &other.bytes,
        )
    }
}

impl std::fmt::Display for Hash256 {

    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {

        write!(
            f,
            "{}",
            self.to_hex(),
        )
    }
}

impl std::str::FromStr for Hash256 {

    type Err = String;

    fn from_str(
        s: &str,
    ) -> Result<Self, Self::Err> {

        Self::from_hex(s)
    }
}