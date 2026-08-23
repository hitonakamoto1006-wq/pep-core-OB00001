use std::{
    fs::{self, File},
    io::{Read, Write},
    path::Path,
};

use crate::blockchain::hash::Hash256;

pub struct Metadata {
    pub chain_id: u32,
    pub version: u32,

    pub best_height: u64,

    pub best_hash: Hash256,

    pub genesis_hash: Hash256,

    pub difficulty: u32,
}

pub struct MetadataStore;

impl MetadataStore {

    const PATH: &'static str = "data/metadata.dat";

    pub fn save(
        metadata: &Metadata,
    ) {

        fs::create_dir_all("data")
            .unwrap();

        let mut file =
            File::create(Self::PATH)
                .unwrap();

        writeln!(
            file,
            "{}",
            metadata.chain_id,
        ).unwrap();

        writeln!(
            file,
            "{}",
            metadata.version,
        ).unwrap();

        writeln!(
            file,
            "{}",
            metadata.best_height,
        ).unwrap();

        writeln!(
            file,
            "{}",
            metadata.best_hash,
        ).unwrap();

        writeln!(
            file,
            "{}",
            metadata.genesis_hash,
        ).unwrap();

        writeln!(
            file,
            "{}",
            metadata.difficulty,
        ).unwrap();
    }

    pub fn load(
    ) -> Option<Metadata> {

        if !Path::new(Self::PATH).exists() {
            return None;
        }

        let mut text =
            String::new();

        File::open(Self::PATH)
            .ok()?
            .read_to_string(&mut text)
            .ok()?;

        let mut lines =
            text.lines();

        Some(
            Metadata {

                chain_id:
                    lines.next()?
                        .parse()
                        .ok()?,

                version:
                    lines.next()?
                        .parse()
                        .ok()?,

                best_height:
                    lines.next()?
                        .parse()
                        .ok()?,

                best_hash:
                    Hash256::from_hex(
                        lines.next()?,
                    )
                    .ok()?,

                genesis_hash:
                    Hash256::from_hex(
                        lines.next()?,
                    )
                    .ok()?,

                difficulty:
                    lines.next()?
                        .parse()
                        .ok()?,
            }
        )
    }

    pub fn clear() {

        if Path::new(Self::PATH).exists() {

            std::fs::remove_file(
                Self::PATH,
            )
            .unwrap();
        }
    }
}