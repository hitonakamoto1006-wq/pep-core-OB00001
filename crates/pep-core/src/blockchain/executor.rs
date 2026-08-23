use crate::blockchain::state::{State, StateError};
use crate::blockchain::transaction::{
    Transaction,
    TransactionType,
};

pub struct Executor;

impl Executor {
    pub fn execute(
        state: &mut State,
        tx: &Transaction,
    ) -> Result<(), StateError> {

        match tx.transaction_type {

            TransactionType::Transfer => {

                state.debit(
                    &tx.from,
                    &tx.asset,
                    tx.amount,
                )?;

                state.credit(
                    &tx.to,
                    &tx.asset,
                    tx.amount,
                )?;

                state.increase_nonce(
                    &tx.from,
                );

                Ok(())
            }

            TransactionType::Mint => {

                state.credit(
                &tx.to,
                &tx.asset,
                tx.amount,
            )?;

                Ok(())
            }

            TransactionType::Burn => {
                todo!()
            }

            TransactionType::Stake => {
                todo!()
            }

            TransactionType::Vote => {
                todo!()
            }
        }
    }
}