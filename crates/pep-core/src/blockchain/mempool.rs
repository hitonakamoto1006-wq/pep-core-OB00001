use crate::blockchain::transaction::Transaction;

pub struct Mempool {
    transactions: Vec<Transaction>,
}

impl Mempool {

    pub fn new() -> Self {
        Self {
            transactions: Vec::new(),
        }
    }

    // ========================================================
    // ADD
    // ========================================================

    pub fn add_transaction(
        &mut self,
        tx: Transaction,
    ) {

        let tx_id =
            tx.id();

        if self.contains_id(
            &tx_id
        ) {
            return;
        }

        self.transactions.push(
            tx
        );
    }

    // ========================================================
    // CONTAINS
    // ========================================================

    pub fn contains_id(
        &self,
        tx_id: &str,
    ) -> bool {

        self.transactions
            .iter()
            .any(
                |tx|
                    tx.id() == tx_id
            )
    }

    // ========================================================
    // LENGTH
    // ========================================================

    pub fn len(
        &self,
    ) -> usize {

        self.transactions.len()
    }

    // ========================================================
    // EMPTY
    // ========================================================

    pub fn is_empty(
        &self,
    ) -> bool {

        self.transactions.is_empty()
    }

    // ========================================================
    // TAKE ALL
    // ========================================================

    pub fn take_all(
        &mut self,
    ) -> Vec<Transaction> {

        std::mem::take(
            &mut self.transactions
        )
    }
}