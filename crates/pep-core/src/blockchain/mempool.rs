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

    pub fn add_transaction(
        &mut self,
        tx: Transaction,
    ) {
        self.transactions.push(tx);
    }

    pub fn len(&self) -> usize {
        self.transactions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.transactions.is_empty()
    }

    pub fn take_all(
        &mut self,
    ) -> Vec<Transaction> {
        std::mem::take(&mut self.transactions)
    }
}