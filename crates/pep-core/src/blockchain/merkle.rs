use crate::blockchain::{
    hash::Hash256,
    transaction::Transaction,
};

use sha2::{
    Digest,
    Sha256,
};

pub struct MerkleTree;

impl MerkleTree {

    // =========================================================
    // ROOT
    // =========================================================

    pub fn root(
        transactions: &[Transaction],
    ) -> Hash256 {

        if transactions.is_empty() {

            return Hash256::zero();
        }

        let mut hashes: Vec<Hash256> =
            transactions
                .iter()
                .map(
                    |tx| tx.hash()
                )
                .collect();

        while hashes.len() > 1 {

            if hashes.len() % 2 != 0 {

                let last =
                    *hashes
                        .last()
                        .unwrap();

                hashes.push(
                    last,
                );
            }

            let mut next =
                Vec::with_capacity(
                    hashes.len() / 2,
                );

            for pair in
                hashes.chunks(2)
            {

                next.push(
                    Self::hash_pair(
                        pair[0],
                        pair[1],
                    )
                );
            }

            hashes =
                next;
        }

        hashes[0]
    }

    // =========================================================
    // HASH PAIR
    // =========================================================

    fn hash_pair(
        left: Hash256,
        right: Hash256,
    ) -> Hash256 {

        let mut data =
            Vec::with_capacity(
                64,
            );

        data.extend_from_slice(
            left.bytes(),
        );

        data.extend_from_slice(
            right.bytes(),
        );

        let digest =
            Sha256::digest(
                &data,
            );

        Hash256::new(
            digest.into(),
        )
    }

    // =========================================================
    // VERIFY ROOT
    // =========================================================

    pub fn verify(
        transactions: &[Transaction],
        root: &Hash256,
    ) -> bool {

        Self::root(
            transactions,
        ) == *root
    }

    // =========================================================
    // MERKLE PROOF
    // =========================================================

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
                .map(
                    |tx| tx.hash()
                )
                .collect();

        while hashes.len() > 1 {

            if hashes.len() % 2 != 0 {

                let last =
                    *hashes
                        .last()
                        .unwrap();

                hashes.push(
                    last,
                );
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
                Vec::with_capacity(
                    hashes.len() / 2,
                );

            for pair in
                hashes.chunks(2)
            {

                next.push(
                    Self::hash_pair(
                        pair[0],
                        pair[1],
                    )
                );
            }

            idx /= 2;

            hashes =
                next;
        }

        Some(
            proof,
        )
    }

    // =========================================================
    // VERIFY PROOF
    // =========================================================

    pub fn verify_proof(
        tx_hash: Hash256,
        proof: &[Hash256],
        mut index: usize,
        expected_root: Hash256,
    ) -> bool {

        let mut current =
            tx_hash;

        for sibling in
            proof
        {

            current =
                if index % 2 == 0 {

                    Self::hash_pair(
                        current,
                        *sibling,
                    )

                } else {

                    Self::hash_pair(
                        *sibling,
                        current,
                    )
                };

            index /= 2;
        }

        current ==
            expected_root
    }
}