use crate::blockchain::{
    asset::AssetId,
    block::Block,
    hash::Hash256,
    transaction::{
        Transaction,
        TransactionType,
    },
};

use crate::wallet::{
    Address,
    PublicKey,
};

pub struct Genesis;

impl Genesis {

    pub fn block() -> Block {

        let transactions = vec![
            Self::mint(
                "pep42foundation",
                1_000_000_000,
            ),
            Self::mint(
                "pep42treasury",
                1_000_000_000,
            ),
            Self::mint(
                "pep42validator",
                1_000_000_000,
            ),
            Self::mint(
                "pep42faucet",
                1_000_000,
            ),
        ];

        Block::new(

            Hash256::zero(),

            0,

            18,

            transactions,
        )
    }

    fn mint(
        address: &str,
        amount: u64,
    ) -> Transaction {

        Transaction {
            asset: AssetId::new(AssetId::PEP),
            transaction_type:
                TransactionType::Mint,

            from:
                Address::new(
                    "GENESIS".to_string(),
                ),

            to:
                Address::new(
                    address.to_string(),
                ),

            amount,

            nonce: 0,

            public_key:
                PublicKey::new(
                    [0u8; 33],
                ),

            signature: None,
        }
    }
}