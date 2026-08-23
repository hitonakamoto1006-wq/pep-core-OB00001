use crate::blockchain::{
    hash::Hash256,
    transaction::Transaction,
};

use crate::wallet::crypto;

pub struct MerkleTree;

impl MerkleTree {

    /// Merkle Root
    pub fn root(
        transactions: &[Transaction],
    ) -> Hash256 {

        if transactions.is_empty() {
            return Hash256::zero();
        }

        let mut hashes: Vec<Hash256> =
            transactions
                .iter()
                .map(|tx| tx.hash())
                .collect();

        while hashes.len() > 1 {

            if hashes.len() % 2 == 1 {

                let last =
                    *hashes.last().unwrap();

                hashes.push(last);
            }

            let mut next =
                Vec::new();

            for pair in hashes.chunks(2) {

                next.push(
                    Self::hash_pair(
                        pair[0],
                        pair[1],
                    ),
                );
            }

            hashes = next;
        }

        hashes[0]
    }

    /// Hash 2 node
    fn hash_pair(
        left: Hash256,
        right: Hash256,
    ) -> Hash256 {

        let mut data =
            Vec::with_capacity(64);

        data.extend_from_slice(
            left.bytes(),
        );

        data.extend_from_slice(
            right.bytes(),
        );

        Hash256::new(
            crypto::sha256_bytes(
                &data,
            ),
        )
    }

    /// Verify Root
    pub fn verify(
        transactions: &[Transaction],
        root: &Hash256,
    ) -> bool {

        Self::root(
            transactions,
        ) == *root
    }

    /// Merkle Proof
    pub fn proof(
        transactions: &[Transaction],
        index: usize,
    ) -> Option<Vec<Hash256>> {

        if transactions.is_empty()
            || index >= transactions.len()
        {
            return None;
        }

        let mut proof =
            Vec::new();

        let mut idx =
            index;

        let mut hashes: Vec<Hash256> =
            transactions
                .iter()
                .map(|tx| tx.hash())
                .collect();

        while hashes.len() > 1 {

            if hashes.len() % 2 == 1 {

                let last =
                    *hashes.last().unwrap();

                hashes.push(last);
            }

            if idx % 2 == 0 {

                proof.push(
                    hashes[idx + 1],
                );

            } else {

                proof.push(
                    hashes[idx - 1],
                );
            }

            let mut next =
                Vec::new();

            for pair in hashes.chunks(2) {

                next.push(
                    Self::hash_pair(
                        pair[0],
                        pair[1],
                    ),
                );
            }

            idx /= 2;

            hashes = next;
        }

        Some(proof)
    }

    /// Verify Proof
    pub fn verify_proof(
        tx_hash: Hash256,
        proof: &[Hash256],
        mut index: usize,
        expected_root: Hash256,
    ) -> bool {

        let mut current =
            tx_hash;

        for hash in proof {

            current =
                if index % 2 == 0 {

                    Self::hash_pair(
                        current,
                        *hash,
                    )

                } else {

                    Self::hash_pair(
                        *hash,
                        current,
                    )
                };

            index /= 2;
        }

        current == expected_root
    }
}