use crate::blockchain::{
    block_header::BlockHeader,
    hash::Hash256,
    transaction::{Transaction, TransactionType},
};

use crate::wallet::crypto::MerkleTree;


#[derive(Clone)]
pub struct Block {

    pub header: BlockHeader,

    pub transactions: Vec<Transaction>,
}


impl Block {

    // ========================================================
    // CREATE BLOCK
    // ========================================================

    pub fn new(
        previous_hash: Hash256,
        timestamp: u64,
        bits: u32,
        transactions: Vec<Transaction>,
    ) -> Self {

        let merkle_root =
            MerkleTree::root(&transactions);

        let header = BlockHeader::new(
            previous_hash,
            merkle_root,
            timestamp,
            bits,
        );

        Self {
            header,
            transactions,
        }
    }


    // ========================================================
    // ADD TRANSACTION
    // ========================================================

    pub fn add_transaction(
        &mut self,
        tx: Transaction,
    ) {

        self.transactions.push(tx);

        self.header.merkle_root =
            MerkleTree::root(
                &self.transactions,
            );
    }


    // ========================================================
    // HASH
    // ========================================================

    pub fn hash(
        &self,
    ) -> Hash256 {

        self.header.hash()
    }


    pub fn calculate_hash(
        &self,
    ) -> Hash256 {

        self.header.calculate_hash()
    }


    // ========================================================
    // NONCE
    // ========================================================

    pub fn nonce(
        &self,
    ) -> u64 {

        self.header.nonce()
    }


    pub fn set_nonce(
        &mut self,
        nonce: u64,
    ) {

        self.header.set_nonce(
            nonce,
        );
    }


    // ========================================================
    // VERIFY INTEGRITY
    // ========================================================
    //
    // Kiểm tra những thứ có thể xác định hoàn toàn
    // từ chính Block.
    //
    // Consensus-level checks như:
    //
    //     previous_hash
    //     PoW difficulty
    //
    // được Blockchain xử lý ở tầng cao hơn.
    //
    // ========================================================

    pub fn verify_integrity(
        &self,
    ) -> Result<(), String> {

        /*
         * ====================================================
         * 1. Merkle Root
         *
         * Không tin merkle_root được gửi từ network.
         *
         * Tự tính lại từ transactions.
         * ====================================================
         */

        let calculated_merkle_root =
            MerkleTree::root(
                &self.transactions,
            );


        if self.header.merkle_root
            != calculated_merkle_root
        {

            return Err(
                "Invalid Merkle root."
                    .to_string()
            );
        }


        /*
         * ====================================================
         * 2. Block Hash
         *
         * Header hash phải đúng với dữ liệu header hiện tại.
         * ====================================================
         */

        let calculated_hash =
            self.calculate_hash();

        let actual_hash =
            self.hash();


        if actual_hash
            != calculated_hash
        {

            return Err(
                "Invalid block hash."
                    .to_string()
            );
        }


        Ok(())
    }


    // ========================================================
    // SERIALIZE
    // ========================================================

    pub fn serialize(
        &self,
    ) -> String {

        let txs = self
            .transactions
            .iter()
            .map(
                |tx| tx.serialize()
            )
            .collect::<Vec<_>>()
            .join(";");


        format!(
            "{}@{}",
            self.header.serialize(),
            txs,
        )
    }


    // ========================================================
    // DESERIALIZE
    // ========================================================

    pub fn deserialize(
        data: &str,
    ) -> Result<Self, String> {

        let parts:
            Vec<&str> =
            data
                .splitn(2, '@')
                .collect();


        if parts.len() != 2 {

            return Err(
                "Invalid Block".into()
            );
        }


        let header =
            BlockHeader::deserialize(
                parts[0],
            )?;


        let mut txs =
            Vec::new();


        if !parts[1].is_empty() {

            for tx in
                parts[1].split(';')
            {

                txs.push(

                    Transaction::deserialize(
                        tx
                    )
                    .ok_or(
                        "Invalid transaction"
                    )?

                );
            }
        }


        let block =
            Self {

                header,

                transactions:
                    txs,
            };


        /*
         * ====================================================
         * IMPORTANT
         *
         * Deserialize xong phải kiểm tra integrity ngay.
         *
         * Như vậy block nhận từ P2P có merkle root/hash giả
         * sẽ bị loại ngay tại boundary.
         * ====================================================
         */

        block.verify_integrity()?;


        Ok(
            block
        )
    }


    // ========================================================
    // PRINT BLOCK INFO
    // ========================================================

    pub fn print_block_info(
        &self,
    ) {

        println!(
            "=============================="
        );

        println!(
            "Block"
        );

        println!(
            "=============================="
        );


        println!(
            "Version        : {}",
            self.header.version
        );

        println!(
            "Previous Hash  : {}",
            self.header.previous_hash
        );

        println!(
            "Merkle Root    : {}",
            self.header.merkle_root
        );

        println!(
            "Timestamp      : {}",
            self.header.timestamp
        );

        println!(
            "Bits           : {}",
            self.header.bits
        );

        println!(
            "Nonce          : {}",
            self.header.nonce()
        );

        println!(
            "Hash           : {}",
            self.hash()
        );


        println!(
            "Transactions   : {}",
            self.transactions.len()
        );


        for (
            i,
            tx
        ) in
            self.transactions
                .iter()
                .enumerate()
        {

            println!(
                "------------ TX {} ------------",
                i + 1
            );


            match tx.transaction_type {

                TransactionType::Transfer =>
                    println!(
                        "Type : Transfer"
                    ),

                TransactionType::Mint =>
                    println!(
                        "Type : Mint"
                    ),

                TransactionType::Burn =>
                    println!(
                        "Type : Burn"
                    ),

                TransactionType::Stake =>
                    println!(
                        "Type : Stake"
                    ),

                TransactionType::Vote =>
                    println!(
                        "Type : Vote"
                    ),
            }


            println!(
                "From   : {}",
                tx.from
            );

            println!(
                "To     : {}",
                tx.to
            );

            println!(
                "Amount : {}",
                tx.amount
            );
        }
    }
}