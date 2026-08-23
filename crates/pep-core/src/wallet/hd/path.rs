pub const HARDENED: u32 = 0x8000_0000;

#[derive(Clone)]
pub struct DerivationPath {
    indices: Vec<u32>,
}

impl DerivationPath {

    pub fn new(indices: Vec<u32>) -> Self {
        Self { indices }
    }

    pub fn indices(&self) -> &[u32] {
        &self.indices
    }

    pub fn evm(
        account: u32,
        index: u32,
    ) -> Self {

        Self::new(vec![
            44 | HARDENED,
            60 | HARDENED,
            account | HARDENED,
            0,
            index,
        ])
    }

    pub fn pep(
        account: u32,
        index: u32,
    ) -> Self {

        Self::new(vec![
            999 | HARDENED,
            0 | HARDENED,
            account | HARDENED,
            0,
            index,
        ])
    }
}