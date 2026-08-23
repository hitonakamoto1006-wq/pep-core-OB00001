use std::collections::HashSet;
use std::sync::{Arc, RwLock};

use crate::PEPDEX::balance::BalanceLedger;
use crate::PEPDEX::market_data::MarketDataEngine;
use crate::PEPDEX::order_store::OrderStore;
use crate::PEPDEX::reconciliation::ReconciliationEngine;
use crate::PEPDEX::risk::RiskEngine;


// ============================================================
// PEPDEX ACCOUNT REGISTRY
// ============================================================

#[derive(Clone)]
pub struct AccountRegistry {

    connected:
        Arc<RwLock<HashSet<String>>>,
}

impl AccountRegistry {

    pub fn new() -> Self {

        Self {
            connected:
                Arc::new(
                    RwLock::new(
                        HashSet::new()
                    )
                ),
        }
    }


    // --------------------------------------------------------
    // CONNECT WALLET
    // --------------------------------------------------------

    pub fn connect(
        &self,
        address: &str,
    ) {

        let mut accounts =
            self.connected
                .write()
                .expect(
                    "account registry poisoned"
                );

        accounts.insert(
            address.to_string()
        );
    }


    // --------------------------------------------------------
    // CHECK CONNECTION
    // --------------------------------------------------------

    pub fn is_connected(
        &self,
        address: &str,
    ) -> bool {

        let accounts =
            self.connected
                .read()
                .expect(
                    "account registry poisoned"
                );

        accounts.contains(address)
    }
}


impl Default for AccountRegistry {

    fn default() -> Self {
        Self::new()
    }
}


// ============================================================
// PEPDEX STATE
// ============================================================

pub struct PEPDEXState {

    pub accounts:
        AccountRegistry,

    pub balances:
        BalanceLedger,

    pub market_data:
        MarketDataEngine,

    pub risk:
        RiskEngine,

    pub orders:
        OrderStore,

    pub reconciliation:
        ReconciliationEngine,
}


impl PEPDEXState {

    pub fn memory() -> Self {

        let balances =
            BalanceLedger::new();

        Self {

            accounts:
                AccountRegistry::new(),

            balances,

            market_data:
                MarketDataEngine::new(),

            risk:
                RiskEngine::new(),

            orders:
                OrderStore::memory(),

            reconciliation:
                ReconciliationEngine::new(),
        }
    }


    pub fn persistent(
        order_store_path:
            impl Into<std::path::PathBuf>,
    ) -> Result<
        Self,
        crate::PEPDEX::order_store::OrderStoreError,
    > {

        let balances =
            BalanceLedger::new();

        let orders =
            OrderStore::file(
                order_store_path
            )?;

        Ok(Self {

            accounts:
                AccountRegistry::new(),

            balances,

            market_data:
                MarketDataEngine::new(),

            risk:
                RiskEngine::new(),

            orders,

            reconciliation:
                ReconciliationEngine::new(),
        })
    }
}


#[cfg(test)]
mod tests {

    use super::*;


    #[test]
    fn creates_memory_state() {

        let state =
            PEPDEXState::memory();

        assert_eq!(
            state
                .orders
                .all()
                .unwrap()
                .len(),
            0
        );
    }


    #[test]
    fn connects_wallet() {

        let state =
            PEPDEXState::memory();

        let address =
            "pep42testwallet";

        assert!(
            !state
                .accounts
                .is_connected(address)
        );

        state
            .accounts
            .connect(address);

        assert!(
            state
                .accounts
                .is_connected(address)
        );
    }
}