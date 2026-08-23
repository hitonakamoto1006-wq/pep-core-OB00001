use crate::blockchain::{
    block::Block,
    hash::Hash256,
};

pub struct Difficulty;

impl Difficulty {

    /// Bitcoin retarget interval
    pub const INTERVAL: u64 = 2016;

    /// 10 phút
    pub const TARGET_BLOCK_TIME: u64 = 600;

    pub const TARGET_TIMESPAN: u64 =
        Self::INTERVAL
            * Self::TARGET_BLOCK_TIME;

    /// Difficulty mặc định
    pub fn genesis_bits() -> u32 {

        18
    }

    /// Tính bits mới
    pub fn next_bits(
        chain: &[Block],
    ) -> u32 {

        if chain.len() < 2 {

            return Self::genesis_bits();
        }

        if chain.len() as u64
            % Self::INTERVAL
            != 0
        {

            return chain
                .last()
                .unwrap()
                .header
                .bits;
        }

        let last =
            chain.last().unwrap();

        let first =
            &chain[
                chain.len()
                    - Self::INTERVAL
                        as usize
            ];

        let actual_time =
            last.header.timestamp
                - first.header.timestamp;

        let mut bits =
            last.header.bits;

        if actual_time
            < Self::TARGET_TIMESPAN / 2
        {

            bits += 1;

        } else if actual_time
            > Self::TARGET_TIMESPAN * 2
        {

            bits =
                bits.saturating_sub(1);
        }

        bits
    }

    /// Difficulty hiện tại
    pub fn current(
        chain: &[Block],
    ) -> u32 {

        chain
            .last()
            .map(|b| b.header.bits)
            .unwrap_or(
                Self::genesis_bits(),
            )
    }

    pub fn target(
        bits: u32,
    ) -> primitive_types::U256 {

        primitive_types::U256::MAX
            >> bits
    }

    pub fn verify_hash(
        hash: Hash256,
        bits: u32,
    ) -> bool {

        hash.to_u256()
            <= Self::target(bits)
    }
}