use std::fs::{create_dir_all, File, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;

use crate::blockchain::{
    block::Block,
    hash::Hash256,
};

pub struct ChainStore;

impl ChainStore {

    const PATH: &'static str = "data/chain.dat";

    /// Append block
    pub fn append(
        block: &Block,
    ) {

        create_dir_all("data").unwrap();

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(Self::PATH)
            .expect("Cannot open chain.dat");

        file.write_all(
            block.serialize().as_bytes(),
        )
        .unwrap();

        file.write_all(b"\n")
            .unwrap();
    }

    /// Save entire chain
    pub fn save(
        blocks: &[Block],
    ) {

        create_dir_all("data").unwrap();

        let mut file = File::create(Self::PATH)
            .expect("Cannot create chain.dat");

        for block in blocks {

            file.write_all(
                block.serialize().as_bytes(),
            )
            .unwrap();

            file.write_all(b"\n")
                .unwrap();
        }
    }

    /// Load chain
    pub fn load(
    ) -> Option<Vec<Block>> {

        if !Path::new(Self::PATH).exists() {
            return None;
        }

        let mut file =
            File::open(Self::PATH)
                .ok()?;

        let mut data =
            String::new();

        file.read_to_string(&mut data)
            .ok()?;

        let mut blocks =
            Vec::new();

        for line in data.lines() {

            if line.trim().is_empty() {
                continue;
            }

            let block =
                Block::deserialize(line)
                    .ok()?;

            blocks.push(block);
        }

        Some(blocks)
    }

    /// Clear chain
    pub fn clear() {

        if Path::new(Self::PATH).exists() {

            std::fs::remove_file(Self::PATH)
                .unwrap();
        }
    }

    /// Current height
    pub fn height(
    ) -> usize {

        Self::load()
            .map(|v| v.len())
            .unwrap_or(0)
    }

    /// Last block
    pub fn last_block(
    ) -> Option<Block> {

        Self::load()?
            .last()
            .cloned()
    }

    /// Block by height
    pub fn get(
        height: usize,
    ) -> Option<Block> {

        Self::load()?
            .get(height)
            .cloned()
    }

    /// Exists?
    pub fn exists(
        height: usize,
    ) -> bool {

        Self::get(height)
            .is_some()
    }

    /// Rewrite chain
    pub fn rewrite(
        blocks: &[Block],
    ) {

        Self::save(blocks);
    }

    /// Last block hash
    pub fn last_hash(
    ) -> Option<Hash256> {

        Some(

            Self::last_block()?
                .hash()

        )
    }
}