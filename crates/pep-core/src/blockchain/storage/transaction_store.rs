use std::{
    collections::hash_map::DefaultHasher,
    fs::{self, File},
    hash::{Hash, Hasher},
    io::{Read, Write},
    path::Path,
};

use crate::blockchain::transaction::Transaction;

pub struct TransactionStore;

impl TransactionStore {

    const DIR: &'static str = "data/transactions";

    /// Lưu transaction
    pub fn put(
        tx: &Transaction,
    ) {

        fs::create_dir_all(Self::DIR)
            .unwrap();

        let hash =
            Self::hash(tx);

        let path = format!(
            "{}/{}.tx",
            Self::DIR,
            hash,
        );

        let mut file =
            File::create(path)
                .unwrap();

        file.write_all(
            tx.serialize().as_bytes(),
        )
        .unwrap();
    }

    /// Đọc transaction
    pub fn get(
        hash: &str,
    ) -> Option<Transaction> {

        let path = format!(
            "{}/{}.tx",
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
            .read_to_string(&mut text)
            .ok()?;

        Transaction::deserialize(
            text.trim(),
        )
    }

    /// Có tồn tại không
    pub fn contains(
        hash: &str,
    ) -> bool {

        let path = format!(
            "{}/{}.tx",
            Self::DIR,
            hash,
        );

        Path::new(&path).exists()
    }

    /// Xóa transaction
    pub fn delete(
        hash: &str,
    ) {

        let path = format!(
            "{}/{}.tx",
            Self::DIR,
            hash,
        );

        if Path::new(&path).exists() {

            let _ =
                fs::remove_file(path);
        }
    }

    /// Xóa toàn bộ transaction
    pub fn clear() {

        if Path::new(Self::DIR).exists() {

            let _ =
                fs::remove_dir_all(
                    Self::DIR,
                );
        }
    }

    /// Số transaction
    pub fn count() -> usize {

        fs::read_dir(Self::DIR)
            .map(|d| d.count())
            .unwrap_or(0)
    }

    /// Transaction Hash
    pub fn hash(
        tx: &Transaction,
    ) -> String {

        let mut hasher =
            DefaultHasher::new();

        tx.serialize()
            .hash(&mut hasher);

        format!(
            "{:016x}",
            hasher.finish()
        )
    }
}