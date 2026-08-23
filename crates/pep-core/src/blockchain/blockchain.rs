use crate::blockchain::{
    block::Block,
    executor::Executor,
    genesis::Genesis,
    state::{
        State,
        StateError,
    },
    transaction::Transaction,
};

use crate::blockchain::consensus::{
    difficulty::Difficulty,
    pow::PoW,
};

use crate::blockchain::storage::{
    block_store::BlockStore,
    chain_store::ChainStore,
    metadata_store::{
        Metadata,
        MetadataStore,
    },
    state_store::StateStore,
    transaction_store::TransactionStore,
};


// ============================================================
// BLOCKCHAIN
// ============================================================

pub struct Blockchain {

    pub blocks:
        Vec<Block>,

    pub state:
        State,
}


impl Blockchain {

    // ========================================================
    // NEW
    // ========================================================

    pub fn new() -> Self {

        /*
         * ====================================================
         * Try loading existing canonical chain.
         * ====================================================
         */

        if let Some(
            blocks
        ) =
            ChainStore::load()
        {

            let state =
                StateStore::load()
                    .unwrap_or_else(
                        State::new
                    );


            return Self {

                blocks,

                state,
            };
        }


        // ====================================================
        // GENESIS
        // ====================================================

        let genesis =
            Genesis::block();


        let mut state =
            State::new();


        /*
         * Execute genesis transactions into the initial
         * canonical state.
         */

        for tx in
            &genesis.transactions
        {

            Executor::execute(
                &mut state,
                tx,
            )
            .expect(
                "Genesis transaction execution failed"
            );
        }


        /*
         * Genesis is height 0.
         */

        state.height =
            0;


        /*
         * Persist genesis.
         */

        ChainStore::append(
            &genesis
        );


        BlockStore::put(
            &genesis
        );


        for tx in
            &genesis.transactions
        {

            TransactionStore::put(
                tx
            );
        }


        StateStore::save(
            &state
        );


        StateStore::save_snapshot(
            &state
        );


        MetadataStore::save(

            &Metadata {

                chain_id:
                    1,

                version:
                    1,

                best_height:
                    0,

                best_hash:
                    genesis.hash(),

                genesis_hash:
                    genesis.hash(),

                difficulty:
                    genesis.header.bits,
            }
        );


        Self {

            blocks:
                vec![
                    genesis
                ],

            state,
        }
    }


    // ========================================================
    // FROM STATE
    // ========================================================

    pub fn from_state(
        state: State,
    ) -> Self {

        Self {

            blocks:
                ChainStore::load()
                    .unwrap_or_else(
                        || {
                            vec![
                                Genesis::block()
                            ]
                        }
                    ),

            state,
        }
    }


    // ========================================================
    // HEIGHT
    // ========================================================

    pub fn height(
        &self,
    ) -> u64 {

        self.blocks
            .len()
            .saturating_sub(1)
            as u64
    }


    // ========================================================
    // TIP
    // ========================================================

    pub fn tip(
        &self,
    ) -> Option<&Block> {

        self.blocks.last()
    }


    // ========================================================
    // EXECUTE ONE TRANSACTION
    // ========================================================

    pub fn execute_transaction(
        &mut self,
        tx: Transaction,
    ) -> Result<
        (),
        StateError,
    > {

        self.execute_transactions(
            vec![
                tx
            ]
        )
    }


    // ========================================================
    // EXECUTE TRANSACTIONS
    // ========================================================
    //
    // Local block production.
    //
    // State transition happens against a temporary clone.
    //
    // If any transaction fails:
    //
    //     canonical state remains unchanged
    //
    // Only after ALL transactions succeed do we replace
    // self.state and create the block.
    // ========================================================

    pub fn execute_transactions(
        &mut self,
        txs: Vec<Transaction>,
    ) -> Result<
        (),
        StateError,
    > {

        /*
         * Empty blocks are not produced by the transaction
         * execution path.
         */

        if txs.is_empty() {

            return Ok(());
        }


        /*
         * Candidate state.
         *
         * This is the important atomicity boundary.
         */

        let mut candidate =
            self.state.clone();


        /*
         * Execute every transaction against candidate state.
         */

        for tx in
            &txs
        {

            Executor::execute(
                &mut candidate,
                tx,
            )?;
        }


        /*
         * All transactions succeeded.
         *
         * Advance state height exactly once for this block.
         */

        candidate.height =
            candidate
                .height
                .saturating_add(1);


        /*
         * Commit the candidate state to memory.
         *
         * create_block() only constructs/mines/persists the block;
         * it does not execute transactions again.
         */

        self.state =
            candidate;


        self.create_block(
            txs
        );


        /*
         * Persist canonical state after the block has been
         * accepted into the local chain.
         */

        StateStore::save(
            &self.state
        );


        StateStore::save_snapshot(
            &self.state
        );


        Ok(())
    }


    // ========================================================
    // CREATE BLOCK
    // ========================================================
    //
    // Used by the local block-production path.
    //
    // Transactions have ALREADY been executed against state
    // before this function is called.
    // ========================================================

    pub fn create_block(
        &mut self,
        transactions: Vec<Transaction>,
    ) {

        let previous =
            self.blocks
                .last()
                .expect(
                    "Genesis missing"
                );


        let bits =
            Difficulty::next_bits(
                &self.blocks
            );


        let timestamp =
            std::time::SystemTime::now()
                .duration_since(
                    std::time::UNIX_EPOCH
                )
                .expect(
                    "System clock is before UNIX epoch"
                )
                .as_secs();


        let mut block =
            Block::new(

                previous.hash(),

                timestamp,

                bits,

                transactions,
            );


        let pow =
            PoW::new(
                bits
            );


        pow.mine(
            &mut block
        );


        self.commit_block(
            block
        );
    }


    // ========================================================
    // COMMIT BLOCK
    // ========================================================
    //
    // Canonical local storage path.
    //
    // The in-memory block list is updated BEFORE metadata is
    // written so best_height always describes the newly
    // committed tip.
    // ========================================================

    pub fn commit_block(
        &mut self,
        block: Block,
    ) {

        /*
         * Persist complete block.
         */

        BlockStore::put(
            &block
        );


        /*
         * Persist transactions belonging to block.
         */

        for tx in
            &block.transactions
        {

            TransactionStore::put(
                tx
            );
        }


        /*
         * Append canonical block to chain storage.
         */

        ChainStore::append(
            &block
        );


        /*
         * Update in-memory canonical chain FIRST.
         */

        self.blocks.push(
            block
        );


        /*
         * The newly pushed block is now the canonical tip.
         */

        let best_block =
            self.blocks
                .last()
                .expect(
                    "Committed blockchain has no tip"
                );


        let best_height =
            self.blocks
                .len()
                .saturating_sub(1)
                as u64;


        let genesis_hash =
            self.blocks[0]
                .hash();


        /*
         * Persist metadata describing the new tip.
         */

        MetadataStore::save(

            &Metadata {

                chain_id:
                    1,

                version:
                    1,

                best_height,

                best_hash:
                    best_block.hash(),

                genesis_hash,

                difficulty:
                    best_block
                        .header
                        .bits,
            }
        );
    }


    // ========================================================
    // VALIDATE + ADD BLOCK
    // ========================================================
    //
    // THIS IS THE ONLY CANONICAL ENTRY POINT FOR P2P BLOCKS.
    //
    // Network:
    //
    //     deserialize
    //          ↓
    //     block integrity
    //          ↓
    //     previous hash
    //          ↓
    //     PoW
    //          ↓
    //     candidate state
    //          ↓
    //     execute ALL tx
    //          ↓
    //     commit block
    //          ↓
    //     persist state
    //
    // If transaction execution fails, canonical state is NOT
    // modified.
    // ========================================================

    pub fn validate_and_add_block(
        &mut self,
        block: Block,
    ) -> Result<
        (),
        String,
    > {

        // ====================================================
        // 1. Chain must have genesis.
        // ====================================================

        let previous =
            self.blocks
                .last()
                .ok_or_else(
                    || {
                        "Blockchain has no genesis block."
                            .to_string()
                    }
                )?;


        // ====================================================
        // 2. Previous hash
        // ====================================================

        if block
            .header
            .previous_hash
            != previous.hash()
        {

            return Err(
                "Invalid previous block hash."
                    .to_string()
            );
        }


        // ====================================================
        // 3. Block hash
        // ====================================================

        let calculated_hash =
            block.calculate_hash();


        let actual_hash =
            block.hash();


        if actual_hash
            != calculated_hash
        {

            return Err(
                "Invalid block hash."
                    .to_string()
            );
        }


        // ====================================================
        // 4. Proof of Work
        // ====================================================

        let pow =
            PoW::new(
                block.header.bits
            );


        if !pow.verify_hash(
            &actual_hash
        ) {

            return Err(
                "Invalid proof of work."
                    .to_string()
            );
        }


        // ====================================================
        // 5. Candidate state
        // ====================================================
        //
        // NEVER execute network transactions directly against
        // canonical self.state.
        // ====================================================

        let mut candidate =
            self.state.clone();


        // ====================================================
        // 6. Execute every transaction
        // ====================================================

        for tx in
            &block.transactions
        {

            Executor::execute(
                &mut candidate,
                tx,
            )
            .map_err(
                |error| {
                    format!(
                        "Block transaction execution failed: {:?}",
                        error
                    )
                }
            )?;
        }


        // ====================================================
        // 7. State height
        // ====================================================

        candidate.height =
            candidate
                .height
                .saturating_add(1);


        // ====================================================
        // 8. Commit state transition
        // ====================================================

        self.state =
            candidate;


        // ====================================================
        // 9. Commit canonical block
        // ====================================================

        self.commit_block(
            block
        );


        // ====================================================
        // 10. Persist canonical state
        // ====================================================

        StateStore::save(
            &self.state
        );


        StateStore::save_snapshot(
            &self.state
        );


        Ok(())
    }


    // ========================================================
    // ADD BLOCK
    // ========================================================
    //
    // Legacy/internal API.
    //
    // P2P MUST NOT use this.
    //
    // P2P uses:
    //
    //     validate_and_add_block()
    // ========================================================

    pub fn add_block(
        &mut self,
        block: Block,
    ) {

        self.commit_block(
            block
        );


        StateStore::save(
            &self.state
        );


        StateStore::save_snapshot(
            &self.state
        );
    }


    // ========================================================
    // VALIDATE CHAIN
    // ========================================================

    pub fn is_valid(
        &self,
    ) -> bool {

        if self.blocks.is_empty() {
            return false;
        }


        for i in
            1..self.blocks.len()
        {

            let previous =
                &self.blocks[
                    i - 1
                ];


            let current =
                &self.blocks[
                    i
                ];


            /*
             * Previous hash.
             */

            if current
                .header
                .previous_hash
                != previous.hash()
            {
                return false;
            }


            /*
             * Block hash.
             */

            if current.hash()
                != current.calculate_hash()
            {
                return false;
            }


            /*
             * Proof of Work.
             */

            let pow =
                PoW::new(
                    current.header.bits
                );


            if !pow.verify_hash(
                &current.hash()
            ) {
                return false;
            }
        }


        true
    }


    // ========================================================
    // PRINT LAST BLOCK
    // ========================================================

    pub fn print_last_block(
        &self,
    ) {

        if let Some(
            block
        ) =
            self.blocks.last()
        {

            block.print_block_info();
        }
    }
}