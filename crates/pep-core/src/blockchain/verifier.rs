use crate::blockchain::{
    state::State,
    transaction::{
        Transaction,
        TransactionType,
    },
};

use crate::wallet::{
    Address,
    Signer,
};

#[derive(Debug)]
pub enum VerifyError {
    MissingSignature,
    InvalidAddress,
    InvalidSignature,
    InvalidNonce,
    InsufficientBalance,
}

pub struct Verifier;

impl Verifier {

    pub fn verify(
        state: &State,
        tx: &Transaction,
    ) -> Result<(), VerifyError> {

        // Có chữ ký
        if tx.signature().is_none() {
            return Err(
                VerifyError::MissingSignature,
            );
        }

        // Address phải khớp PublicKey
        let address =
            Address::from_public_key(
                &tx.public_key,
            );

        if address != tx.from {
            return Err(
                VerifyError::InvalidAddress,
            );
        }

        // Verify chữ ký
        if !Signer::verify(
            &tx.public_key,
            tx,
        ) {
            return Err(
                VerifyError::InvalidSignature,
            );
        }

        match tx.transaction_type {

            TransactionType::Transfer => {

                let account =
                    match state.get_account(&tx.from) {

                        Some(account) => account,

                        None => {
                            return Err(
                                VerifyError::InsufficientBalance,
                            );
                        }
                    };

                if account.nonce != tx.nonce {
                    return Err(
                        VerifyError::InvalidNonce,
                    );
                }

                if account.balance(&tx.asset) < tx.amount {
                    return Err(
                        VerifyError::InsufficientBalance,
                    );
                }
            }

            TransactionType::Mint => {

                // Testnet:
                // Chưa kiểm Founder Wallet
                // Chưa kiểm Treasury

                return Ok(());
            }

            TransactionType::Burn => {

                let account =
                    match state.get_account(&tx.from) {

                        Some(account) => account,

                        None => {
                            return Err(
                                VerifyError::InsufficientBalance,
                            );
                        }
                    };

                if account.nonce != tx.nonce {
                    return Err(
                        VerifyError::InvalidNonce,
                    );
                }

                if account.balance(&tx.asset) < tx.amount {
                    return Err(
                        VerifyError::InsufficientBalance,
                    );
                }
            }

            TransactionType::Stake => {}

            TransactionType::Vote => {}
        }

        Ok(())
    }
}