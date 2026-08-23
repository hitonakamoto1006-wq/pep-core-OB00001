use crate::blockchain::{
    blockchain::Blockchain,
    block::Block,
    mempool::Mempool,
    state::StateError,
    transaction::Transaction,
    verifier::Verifier,
};

use crate::wallet::Address;


// ============================================================
// NODE
// ============================================================
//
// Node là lớp consensus/state-facing.
//
// Network layer KHÔNG được tự:
//
//     - push block
//     - sửa state
//     - sửa blockchain
//
// Mọi block nhận từ P2P:
//
//     Core
//       ↓
//     Node::accept_block()
//       ↓
//     Blockchain::validate_and_add_block()
//
// Transaction:
//
//     Node::on_transaction()
//       ↓
//     Verifier
//       ↓
//     Mempool
//       ↓
//     execute_transactions()
// ============================================================

const BLOCK_SIZE: usize = 1;


pub struct Node {

    blockchain:
        Blockchain,

    mempool:
        Mempool,
}


impl Node {

    // ========================================================
    // NEW
    // ========================================================

    pub fn new() -> Self {

        println!(
            "Loading blockchain..."
        );


        let blockchain =
            Blockchain::new();


        println!(
            "Blockchain loaded. Height: {}",
            blockchain
                .blocks
                .len()
                .saturating_sub(1)
        );


        Self {

            blockchain,

            mempool:
                Mempool::new(),
        }
    }


    // ========================================================
    // BLOCKCHAIN ACCESS
    // ========================================================

    pub fn blockchain(
        &self,
    ) -> &Blockchain {

        &self.blockchain
    }


    pub fn blockchain_mut(
        &mut self,
    ) -> &mut Blockchain {

        &mut self.blockchain
    }


    // ========================================================
    // CURRENT HEIGHT
    // ========================================================

    pub fn height(
        &self,
    ) -> usize {

        self.blockchain
            .blocks
            .len()
            .saturating_sub(1)
    }


    // ========================================================
    // TIP HASH
    // ========================================================

    pub fn tip_hash(
        &self,
    ) -> String {

        self.blockchain
            .blocks
            .last()
            .map(
                |block|
                    block
                        .hash()
                        .to_string()
            )
            .unwrap_or_else(
                || "0".to_string()
            )
    }


    // ========================================================
    // BALANCE
    // ========================================================

    pub fn get_balance(
        &self,
        address: &Address,
    ) -> (
        Vec<(String, u64)>,
        u64,
        u64,
    ) {

        match self
            .blockchain
            .state
            .get_account(address)
        {

            Some(account) => {

                let mut balances:
                    Vec<(String, u64)> =
                    account
                        .balances
                        .iter()
                        .filter_map(
                            |(asset, amount)| {

                                if *amount == 0 {
                                    return None;
                                }


                                Some(
                                    (
                                        asset.to_string(),
                                        *amount,
                                    )
                                )
                            }
                        )
                        .collect();


                /*
                 * HashMap order is not deterministic.
                 *
                 * Sort so network responses are stable.
                 */

                balances.sort_by(
                    |a, b|
                        a.0.cmp(
                            &b.0
                        )
                );


                (
                    balances,
                    account.nonce,
                    account.stake,
                )
            }


            None => {

                (
                    Vec::new(),
                    0,
                    0,
                )
            }
        }
    }


    // ========================================================
    // TRANSACTION
    // ========================================================
    //
    // Local wallet / P2P transaction.
    //
    // Invalid transaction:
    //
    //     rejected
    //
    // Valid transaction:
    //
    //     mempool
    //     ↓
    //     block
    // ========================================================

    pub fn on_transaction(
        &mut self,
        tx: Transaction,
    ) -> Result<(), StateError> {

        println!(
            "Transaction received."
        );


        /*
         * Verify against current canonical state.
         */

        match Verifier::verify(
            &self.blockchain.state,
            &tx,
        ) {

            Ok(_) => {}


            Err(error) => {

                println!(
                    "Transaction rejected: {:?}",
                    error
                );


                /*
                 * Preserve existing API:
                 *
                 * invalid transaction does not crash
                 * the node.
                 */

                return Ok(());
            }
        }


        /*
         * Add verified transaction.
         */

        self.mempool
            .add_transaction(
                tx
            );


        println!(
            "Mempool: {} transaction(s)",
            self.mempool.len()
        );


        /*
         * Current chain:
         *
         * one transaction per block.
         */

        if self.mempool.len()
            >= BLOCK_SIZE
        {

            self.mine_pending()?;
        }


        Ok(())
    }


    // ========================================================
    // ACCEPT BLOCK
    // ========================================================
    //
    // IMPORTANT:
    //
    // P2P block MUST enter here.
    //
    // Không được:
    //
    //     blockchain.blocks.push(...)
    //
    // Blockchain tự chịu trách nhiệm validate.
    //
    // Return type là String vì:
    //
    // Blockchain::validate_and_add_block()
    // hiện trả Result<(), String>.
    //
    // ========================================================

    pub fn accept_block(
        &mut self,
        block: Block,
    ) -> Result<(), String> {

        let current_height =
            self.height();


        println!(
            "Block received at network height {}.",
            current_height
                .saturating_add(1)
        );


        /*
         * Canonical blockchain validation.
         */

        self.blockchain
            .validate_and_add_block(
                block
            )?;


        /*
         * Block accepted.
         */

        println!(
            "Block accepted. New height: {}",
            self.height()
        );


        Ok(())
    }


    // ========================================================
    // MINE PENDING
    // ========================================================

    pub fn mine_pending(
        &mut self,
    ) -> Result<(), StateError> {

        if self.mempool.len() == 0 {

            return Ok(());
        }


        let txs =
            self.mempool
                .take_all();


        /*
         * Local block production.
         */

        self.blockchain
            .execute_transactions(
                txs
            )?;


        println!(
            "Block mined."
        );


        self.print_status();


        Ok(())
    }


    // ========================================================
    // MEMPOOL SIZE
    // ========================================================

    pub fn mempool_len(
        &self,
    ) -> usize {

        self.mempool.len()
    }


    // ========================================================
    // STATUS
    // ========================================================

    pub fn print_status(
        &self,
    ) {

        println!();

        println!(
            "========== NODE =========="
        );


        println!(
            "Height : {}",
            self.height()
        );


        println!(
            "Blocks : {}",
            self.blockchain
                .blocks
                .len()
        );


        println!(
            "Mempool: {}",
            self.mempool.len()
        );


        self.blockchain
            .state
            .print_accounts();


        self.blockchain
            .print_last_block();


        println!(
            "=========================="
        );
    }
}