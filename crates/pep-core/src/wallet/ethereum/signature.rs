use anyhow::{anyhow, Result};

#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    PartialEq,
    Eq,
)]
pub struct Signature {

    r: [u8; 32],

    s: [u8; 32],

    v: u8,

}

impl Signature {

    /// Create signature.
    pub fn new(
        r: [u8; 32],
        s: [u8; 32],
        v: u8,
    ) -> Self {

        Self {
            r,
            s,
            v,
        }

    }

    /// Empty signature.
    pub fn empty() -> Self {

        Self::default()

    }

    /// Parse from 65-byte signature.
    pub fn from_bytes(
        bytes: &[u8; 65],
    ) -> Result<Self> {

        let mut r =
            [0u8; 32];

        let mut s =
            [0u8; 32];

        r.copy_from_slice(
            &bytes[..32],
        );

        s.copy_from_slice(
            &bytes[32..64],
        );

        Ok(

            Self {

                r,

                s,

                v: bytes[64],

            }

        )

    }

    /// Parse from hex string.
    pub fn from_hex(
        hex: &str,
    ) -> Result<Self> {

        let hex =
            hex.trim_start_matches(
                "0x",
            );

        let bytes =
            hex::decode(
                hex,
            )?;

        if bytes.len() != 65 {

            return Err(anyhow!(
                "Invalid signature length",
            ));

        }

        let mut raw =
            [0u8; 65];

        raw.copy_from_slice(
            &bytes,
        );

        Self::from_bytes(
            &raw,
        )

    }

    /// Serialize.
    pub fn to_bytes(
        &self,
    ) -> [u8; 65] {

        let mut out =
            [0u8; 65];

        out[..32]
            .copy_from_slice(
                &self.r,
            );

        out[32..64]
            .copy_from_slice(
                &self.s,
            );

        out[64] =
            self.v;

        out

    }

    /// Hex.
    pub fn to_hex(
        &self,
    ) -> String {

        hex::encode(
            self.to_bytes(),
        )

    }

    pub fn r(
        &self,
    ) -> &[u8; 32] {

        &self.r

    }

    pub fn s(
        &self,
    ) -> &[u8; 32] {

        &self.s

    }

    pub fn v(
        &self,
    ) -> u8 {

        self.v

    }

    pub fn set_v(
        &mut self,
        v: u8,
    ) {

        self.v = v;

    }

    /// Recovery id (0/1).
    pub fn recovery_id(
        &self,
    ) -> u8 {

        if self.v >= 27 {

            self.v - 27

        } else {

            self.v

        }

    }

    /// Convert 0/1 -> 27/28.
    pub fn normalize_v(
        &mut self,
    ) {

        if self.v < 27 {

            self.v += 27;

        }

    }

    pub fn is_empty(
        &self,
    ) -> bool {

        self.r
            .iter()
            .all(
                |v| *v == 0,
            )

        &&

        self.s
            .iter()
            .all(
                |v| *v == 0,
            )

    }

}

impl std::fmt::Display
for Signature {

    fn fmt(

        &self,

        f: &mut std::fmt::Formatter<'_>,

    ) -> std::fmt::Result {

        write!(
            f,
            "0x{}",
            self.to_hex(),
        )

    }

}