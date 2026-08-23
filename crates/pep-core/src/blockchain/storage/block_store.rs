use std::{
    fs::{self, File},
    io::{Read, Write},
    path::Path,
};

use crate::blockchain::{
    block::Block,
    hash::Hash256,
};

pub struct BlockStore;

impl BlockStore {

    const DIR: &'static str = "data/blocks";

    /// Lưu block theo hash
    pub fn put(
        block: &Block,
    ) {

        fs::create_dir_all(
            Self::DIR,
        )
        .unwrap();

        let path = format!(
            "{}/{}.blk",
            Self::DIR,
            block.hash(),
        );

        let mut file =
            File::create(path)
                .unwrap();

        file.write_all(
            block.serialize().as_bytes(),
        )
        .unwrap();
    }

    /// Đọc block theo hash
    pub fn get(
        hash: &Hash256,
    ) -> Option<Block> {

        let path = format!(
            "{}/{}.blk",
            Self::DIR,
            hash,
        );

        if !Path::new(&path).exists() {
            return None;
        }

        let mut text =
            String::new();

        File::open(path)
            .ok()?
            .read_to_string(
                &mut text,
            )
            .ok()?;

        Block::deserialize(
            text.trim(),
        )
        .ok()
    }

    /// Block có tồn tại không
    pub fn contains(
        hash: &Hash256,
    ) -> bool {

        let path = format!(
            "{}/{}.blk",
            Self::DIR,
            hash,
        );

        Path::new(&path).exists()
    }

    /// Xóa block
    pub fn delete(
        hash: &Hash256,
    ) {

        let path = format!(
            "{}/{}.blk",
            Self::DIR,
            hash,
        );

        if Path::new(&path).exists() {

            let _ =
                fs::remove_file(path);
        }
    }

    /// Xóa toàn bộ block
    pub fn clear() {

        if Path::new(Self::DIR).exists() {

            let _ =
                fs::remove_dir_all(
                    Self::DIR,
                );
        }
    }

    /// Đếm số block đang lưu
    pub fn count() -> usize {

        fs::read_dir(
            Self::DIR,
        )
        .map(|dir| dir.count())
        .unwrap_or(0)
    }
}