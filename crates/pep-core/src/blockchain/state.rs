use std::collections::HashMap;

use crate::blockchain::asset::{
    AssetDefinition,
    AssetId,
    AssetRegistry,
};

use crate::wallet::Address;


// ============================================================
// STATE ERROR
// ============================================================

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
)]
pub enum StateError {

    AccountNotFound,

    AssetNotFound,

    AssetAlreadyExists,

    InsufficientBalance,

    Overflow,
}


// ============================================================
// ACCOUNT
// ============================================================

#[derive(
    Clone,
)]
pub struct Account {

    pub address:
        Address,

    /*
     * MULTI-ASSET BALANCES
     *
     * AssetId -> amount
     *
     * Ví dụ:
     *
     * PEP  -> 1000
     * BTCP -> 200000
     * USDP -> 5000000
     */
    pub balances:
        HashMap<
            AssetId,
            u64,
        >,

    pub nonce:
        u64,

    pub stake:
        u64,
}


impl Account {

    pub fn new(
        address: Address,
    ) -> Self {

        Self {

            address,

            balances:
                HashMap::new(),

            nonce: 0,

            stake: 0,
        }
    }


    pub fn balance(
        &self,
        asset: &AssetId,
    ) -> u64 {

        self.balances
            .get(asset)
            .copied()
            .unwrap_or(0)
    }
}


// ============================================================
// STATE
// ============================================================

#[derive(
    Clone,
)]
pub struct State {

    pub height:
        u64,

    pub accounts:
        HashMap<
            Address,
            Account,
        >,

    /*
     * ASSET DEFINITIONS
     *
     * AssetId -> AssetDefinition
     *
     * Đây là canonical asset state.
     *
     * Account balances chỉ lưu:
     *
     *     AssetId -> amount
     *
     * Metadata của asset nằm ở đây.
     */
    pub assets:
        HashMap<
            AssetId,
            AssetDefinition,
        >,
}


impl State {

    pub fn new() -> Self {

        let mut state =
            Self {

                height: 0,

                accounts:
                    HashMap::new(),

                assets:
                    HashMap::new(),
            };


        /*
         * Load core assets mặc định.
         *
         * PEP / BTCP / ETHP / USDP
         */
        for symbol in [
            AssetId::PEP,
            AssetId::BTCP,
            AssetId::ETHP,
            AssetId::USDP,
        ] {

            let asset =
                AssetId::new(
                    symbol
                );


            if let Some(
                definition
            ) =
                AssetRegistry::get(
                    &asset
                )
            {

                state.assets.insert(
                    asset,
                    definition,
                );
            }
        }


        state
    }


    // ========================================================
    // ASSET
    // ========================================================

    pub fn register_asset(
        &mut self,
        asset: AssetDefinition,
    ) -> Result<
        (),
        StateError,
    > {

        let id =
            asset.id.clone();


        /*
         * Canonical state không cho duplicate.
         */
        if self
            .assets
            .contains_key(&id)
        {

            return Err(
                StateError::AssetAlreadyExists
            );
        }


        /*
         * Runtime registry cũng phải biết asset.
         */
        AssetRegistry::register(
            asset.clone()
        )
        .map_err(
            |_| {
                StateError::AssetAlreadyExists
            }
        )?;


        self.assets.insert(
            id,
            asset,
        );


        Ok(())
    }


    pub fn get_asset(
        &self,
        asset: &AssetId,
    ) -> Option<
        &AssetDefinition
    > {

        self.assets.get(
            asset
        )
    }


    pub fn has_asset(
        &self,
        asset: &AssetId,
    ) -> bool {

        self.assets
            .contains_key(asset)
    }


    pub fn assets(
        &self,
    ) -> &HashMap<
        AssetId,
        AssetDefinition,
    > {

        &self.assets
    }


    // ========================================================
    // ACCOUNT
    // ========================================================

    pub fn create_account(
        &mut self,
        address: &Address,
    ) {

        if !self
            .accounts
            .contains_key(address)
        {

            let account =
                Account::new(
                    address.clone()
                );


            self.accounts.insert(
                address.clone(),
                account,
            );
        }
    }


    pub fn get_account(
        &self,
        address: &Address,
    ) -> Option<
        &Account
    > {

        self.accounts.get(
            address
        )
    }


    pub fn get_account_mut(
        &mut self,
        address: &Address,
    ) -> Option<
        &mut Account
    > {

        self.accounts.get_mut(
            address
        )
    }


    // ========================================================
    // BALANCE
    // ========================================================

    pub fn balance(
        &self,
        address: &Address,
        asset: &AssetId,
    ) -> u64 {

        self.get_account(
            address
        )
        .map(
            |account|
                account.balance(
                    asset
                )
        )
        .unwrap_or(0)
    }


    // ========================================================
    // CREDIT
    // ========================================================

    pub fn credit(
        &mut self,
        address: &Address,
        asset: &AssetId,
        amount: u64,
    ) -> Result<
        (),
        StateError,
    > {

        /*
         * Không credit asset không tồn tại.
         */
        if !self.has_asset(
            asset
        ) {

            return Err(
                StateError::AssetNotFound
            );
        }


        self.create_account(
            address
        );


        let account =
            self.get_account_mut(
                address
            )
            .ok_or(
                StateError::AccountNotFound
            )?;


        let current =
            account
                .balances
                .get(asset)
                .copied()
                .unwrap_or(0);


        let new_balance =
            current
                .checked_add(
                    amount
                )
                .ok_or(
                    StateError::Overflow
                )?;


        account
            .balances
            .insert(
                asset.clone(),
                new_balance,
            );


        Ok(())
    }


    // ========================================================
    // DEBIT
    // ========================================================

    pub fn debit(
        &mut self,
        address: &Address,
        asset: &AssetId,
        amount: u64,
    ) -> Result<
        (),
        StateError,
    > {

        /*
         * Asset phải tồn tại.
         */
        if !self.has_asset(
            asset
        ) {

            return Err(
                StateError::AssetNotFound
            );
        }


        let account =
            self.get_account_mut(
                address
            )
            .ok_or(
                StateError::AccountNotFound
            )?;


        let current =
            account
                .balances
                .get(asset)
                .copied()
                .unwrap_or(0);


        if current < amount {

            return Err(
                StateError::InsufficientBalance
            );
        }


        let new_balance =
            current - amount;


        if new_balance == 0 {

            account
                .balances
                .remove(
                    asset
                );

        } else {

            account
                .balances
                .insert(
                    asset.clone(),
                    new_balance,
                );
        }


        Ok(())
    }


    // ========================================================
    // NONCE
    // ========================================================

    pub fn increase_nonce(
        &mut self,
        address: &Address,
    ) {

        if let Some(
            account
        ) =
            self.get_account_mut(
                address
            )
        {

            account.nonce =
                account
                    .nonce
                    .saturating_add(1);
        }
    }


    // ========================================================
    // DEBUG
    // ========================================================

    pub fn print_accounts(
        &self,
    ) {

        println!(
            "========== Accounts =========="
        );


        for account
            in self.accounts.values()
        {

            println!(
                "{} | Nonce: {} | Stake: {}",
                account.address,
                account.nonce,
                account.stake,
            );


            for (
                asset,
                balance,
            ) in &account.balances
            {

                println!(
                    "    {} = {}",
                    asset,
                    balance,
                );
            }
        }


        println!(
            "=============================="
        );
    }


    // ========================================================
    // DEBUG ASSETS
    // ========================================================

    pub fn print_assets(
        &self,
    ) {

        println!(
            "========== Assets =========="
        );


        for (
            id,
            asset,
        ) in &self.assets
        {

            println!(
                "{} | Type: {:?} | Decimals: {} | Supply: {} | Deploy: {}",
                id,
                asset.asset_type,
                asset.decimals,
                asset.supply,
                asset.deploy_address
                    .as_deref()
                    .unwrap_or("-"),
            );
        }


        println!(
            "============================"
        );
    }
}