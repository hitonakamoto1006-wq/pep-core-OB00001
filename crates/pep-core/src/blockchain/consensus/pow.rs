use std::time::{Duration, Instant};

use primitive_types::U256;

use crate::blockchain::{
    block::Block,
    consensus::difficulty::Difficulty,
    hash::Hash256,
};

pub struct PoW {
    bits: u32,
}

impl PoW {

    pub fn new(
        bits: u32,
    ) -> Self {

        Self {
            bits,
        }
    }

    pub fn bits(
        &self,
    ) -> u32 {

        self.bits
    }

    pub fn target(
        &self,
    ) -> U256 {

        Difficulty::target(
            self.bits,
        )
    }

    pub fn verify_hash(
        &self,
        hash: &Hash256,
    ) -> bool {

        Difficulty::verify_hash(
            *hash,
            self.bits,
        )
    }

    pub fn verify_block(
        &self,
        block: &Block,
    ) -> bool {

        let hash =
            block.header.calculate_hash();

        hash == block.hash()
            && self.verify_hash(
                &hash,
            )
    }

    // ===========================
    // Benchmark
    // ===========================

    pub fn benchmark(
        &self,
        block: &mut Block,
    ) -> u64 {

        let start =
            Instant::now();

        let mut nonce = 0u64;

        while start.elapsed()
            < Duration::from_secs(1)
        {

            block.header.set_nonce(
                nonce,
            );

            let _ =
                block.header.calculate_hash();

            nonce += 1;
        }

        println!(
            "Hashrate: {} H/s",
            nonce,
        );

        nonce
    }

    // ===========================
    // Mining
    // ===========================

    pub fn mine(
        &self,
        block: &mut Block,
    ) {

        let start =
            Instant::now();

        let mut last_report =
            Instant::now();

        let mut nonce = 0u64;

        let mut hashes = 0u64;

        loop {

            block.header.set_nonce(
                nonce,
            );

            let hash =
                block.header.calculate_hash();

            hashes += 1;

            if self.verify_hash(
                &hash,
            ) {

                let elapsed =
                    start
                        .elapsed()
                        .as_secs_f64();

                let hps =
                    hashes as f64
                        / elapsed;

                println!();
                println!("==================================");
                println!("BLOCK FOUND");
                println!("==================================");
                println!("Bits       : {}", self.bits);
                println!("Nonce      : {}", nonce);
                println!("Hash       : {}", hash);
                println!("Elapsed    : {:.3} s", elapsed);

                if hps >= 1_000_000.0 {

                    println!(
                        "Hashrate   : {:.2} MH/s",
                        hps / 1_000_000.0,
                    );

                } else if hps >= 1_000.0 {

                    println!(
                        "Hashrate   : {:.2} KH/s",
                        hps / 1_000.0,
                    );

                } else {

                    println!(
                        "Hashrate   : {:.2} H/s",
                        hps,
                    );
                }

                println!("==================================");

                break;
            }

            if last_report.elapsed()
                >= Duration::from_secs(1)
            {

                let elapsed =
                    start
                        .elapsed()
                        .as_secs_f64();

                let hps =
                    hashes as f64
                        / elapsed;

                if hps >= 1_000_000.0 {

                    println!(
                        "Mining... {:.2} MH/s | Bits {} | Nonce {}",
                        hps / 1_000_000.0,
                        self.bits,
                        nonce,
                    );

                } else if hps >= 1_000.0 {

                    println!(
                        "Mining... {:.2} KH/s | Bits {} | Nonce {}",
                        hps / 1_000.0,
                        self.bits,
                        nonce,
                    );

                } else {

                    println!(
                        "Mining... {:.2} H/s | Bits {} | Nonce {}",
                        hps,
                        self.bits,
                        nonce,
                    );
                }

                last_report =
                    Instant::now();
            }

            nonce += 1;
        }
    }
}