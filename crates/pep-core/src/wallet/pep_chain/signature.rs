use std::fmt;

#[derive(Clone, Debug)]
pub struct Signature {
    bytes: [u8; 64],
}

impl Signature {
    pub fn new(bytes: [u8; 64]) -> Self {
        Self { bytes }
    }

    pub fn bytes(&self) -> &[u8; 64] {
        &self.bytes
    }

    pub fn to_hex(&self) -> String {
        self.bytes
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect()
    }

    pub fn from_hex(hex: &str) -> Option<Self> {
        if hex.len() != 128 {
            return None;
        }

        let mut bytes = [0u8; 64];

        for i in 0..64 {
            bytes[i] = u8::from_str_radix(
                &hex[i * 2..i * 2 + 2],
                16,
            )
            .ok()?;
        }

        Some(Self { bytes })
    }
}

impl fmt::Display for Signature {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}