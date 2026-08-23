use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::PEPDEX::settlement::{
    SettlementResult,
    SettlementStatus,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BalanceKey {
    pub user: String,
    pub asset: String,
}

impl BalanceKey {
    pub fn new(
        user: impl Into<String>,
        asset: impl Into<String>,
    ) -> Self {
        Self {
            user: user.into(),
            asset: asset.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Balance {
    /// Available balance that can be used for new orders.
    pub available: i128,

    /// Balance locked by pending orders.
    pub locked: i128,
}

impl Balance {
    pub fn new() -> Self {
        Self {
            available: 0,
            locked: 0,
        }
    }

    pub fn total(&self) -> i128 {
        self.available + self.locked
    }
}

impl Default for Balance {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct BalanceLedger {
    balances: Arc<RwLock<HashMap<BalanceKey, Balance>>>,
}

impl BalanceLedger {
    pub fn new() -> Self {
        Self {
            balances: Arc::new(
                RwLock::new(HashMap::new())
            ),
        }
    }

    pub fn get(
        &self,
        user: &str,
        asset: &str,
    ) -> Balance {
        let balances = self
            .balances
            .read()
            .expect("balance ledger poisoned");

        balances
            .get(&BalanceKey::new(user, asset))
            .copied()
            .unwrap_or_default()
    }

    pub fn available(
        &self,
        user: &str,
        asset: &str,
    ) -> i128 {
        self.get(user, asset).available
    }

    pub fn total(
        &self,
        user: &str,
        asset: &str,
    ) -> i128 {
        self.get(user, asset).total()
    }

    pub fn deposit(
        &self,
        user: &str,
        asset: &str,
        amount: i128,
    ) -> Result<(), BalanceError> {
        if amount <= 0 {
            return Err(
                BalanceError::InvalidAmount
            );
        }

        self.apply_delta(
            user,
            asset,
            amount,
        )
    }

    pub fn withdraw(
        &self,
        user: &str,
        asset: &str,
        amount: i128,
    ) -> Result<(), BalanceError> {
        if amount <= 0 {
            return Err(
                BalanceError::InvalidAmount
            );
        }

        let current =
            self.get(user, asset);

        if current.available < amount {
            return Err(
                BalanceError::InsufficientAvailable {
                    asset: asset.to_string(),
                    available:
                        current.available,
                    requested:
                        amount as u128,
                }
            );
        }

        self.apply_delta(
            user,
            asset,
            -amount,
        )
    }

    pub fn lock(
        &self,
        user: &str,
        asset: &str,
        amount: i128,
    ) -> Result<(), BalanceError> {
        if amount <= 0 {
            return Err(
                BalanceError::InvalidAmount
            );
        }

        let key =
            BalanceKey::new(user, asset);

        let mut balances = self
            .balances
            .write()
            .expect("balance ledger poisoned");

        let balance =
            balances
                .entry(key)
                .or_default();

        if balance.available < amount {
            return Err(
                BalanceError::InsufficientAvailable {
                    asset: asset.to_string(),
                    available:
                        balance.available,
                    requested:
                        amount as u128,
                }
            );
        }

        balance.available -= amount;
        balance.locked += amount;

        Ok(())
    }

    pub fn unlock(
        &self,
        user: &str,
        asset: &str,
        amount: i128,
    ) -> Result<(), BalanceError> {
        if amount <= 0 {
            return Err(
                BalanceError::InvalidAmount
            );
        }

        let key =
            BalanceKey::new(user, asset);

        let mut balances = self
            .balances
            .write()
            .expect("balance ledger poisoned");

        let balance =
            balances
                .entry(key)
                .or_default();

        if balance.locked < amount {
            return Err(
                BalanceError::InsufficientLocked {
                    asset: asset.to_string(),
                    locked: balance.locked,
                    requested: amount,
                }
            );
        }

        balance.locked -= amount;
        balance.available += amount;

        Ok(())
    }

    pub fn apply_delta(
        &self,
        user: &str,
        asset: &str,
        delta: i128,
    ) -> Result<(), BalanceError> {
        if delta == 0 {
            return Ok(());
        }

        let key =
            BalanceKey::new(user, asset);

        let mut balances = self
            .balances
            .write()
            .expect("balance ledger poisoned");

        let balance =
            balances
                .entry(key)
                .or_default();

        let new_available =
            balance.available
                .checked_add(delta)
                .ok_or(
                    BalanceError::Overflow
                )?;

        if new_available < 0 {
            return Err(
                BalanceError::InsufficientAvailable {
                    asset: asset.to_string(),
                    available:
                        balance.available,
                    requested:
                        delta.unsigned_abs(),
                }
            );
        }

        balance.available =
            new_available;

        Ok(())
    }

    /// Apply the final result of a PEPDEX execution.
    ///
    /// BUY:
    ///     +base
    ///     -quote
    ///
    /// SELL:
    ///     -base
    ///     +quote
        // =========================================================
    // LIST USER BALANCES
    // =========================================================

    pub fn user_balances(
        &self,
        user: &str,
    ) -> Vec<(String, Balance)> {

        let balances =
            self.balances
                .read()
                .expect(
                    "balance ledger poisoned"
                );

        balances
            .iter()
            .filter_map(
                |(key, balance)| {

                    if key.user == user {

                        Some((
                            key.asset.clone(),
                            *balance,
                        ))

                    } else {

                        None

                    }
                }
            )
            .collect()
    }
    pub fn apply_settlement(
        &self,
        settlement: &SettlementResult,
    ) -> Result<(), BalanceError> {
        if !matches!(
            settlement.status,
            SettlementStatus::Settled
                | SettlementStatus::PartiallySettled
        ) {
            return Err(
                BalanceError::SettlementNotFinal
            );
        }

        /*
         * Important:
         *
         * Apply both legs atomically under ONE write lock.
         * We don't want:
         *
         * +100 PEP
         * then crash
         * before -USDT
         *
         * leaving the ledger inconsistent.
         */
        let asset_key =
            BalanceKey::new(
                &settlement.user,
                &settlement.asset,
            );

        let quote_key =
            BalanceKey::new(
                &settlement.user,
                &settlement.quote_asset,
            );

        let mut balances = self
            .balances
            .write()
            .expect("balance ledger poisoned");

        /*
         * Take immutable snapshots first.
         *
         * We intentionally do NOT keep mutable references
         * to entries here. Otherwise borrowing the asset
         * entry and then the quote entry from the same
         * HashMap would trigger E0499.
         */
        let current_asset =
            balances
                .get(&asset_key)
                .copied()
                .unwrap_or_default();

        let current_quote =
            balances
                .get(&quote_key)
                .copied()
                .unwrap_or_default();

        /*
         * Calculate both resulting balances before
         * modifying either one.
         */
        let new_asset =
            current_asset
                .available
                .checked_add(
                    settlement.asset_delta
                )
                .ok_or(
                    BalanceError::Overflow
                )?;

        let new_quote =
            current_quote
                .available
                .checked_add(
                    settlement.quote_delta
                )
                .ok_or(
                    BalanceError::Overflow
                )?;

        /*
         * Validate BOTH legs before writing either leg.
         *
         * Therefore settlement remains atomic.
         */
        if new_asset < 0 {
            return Err(
                BalanceError::InsufficientAvailable {
                    asset:
                        settlement.asset.clone(),
                    available:
                        current_asset.available,
                    requested:
                        settlement
                            .asset_delta
                            .unsigned_abs(),
                }
            );
        }

        if new_quote < 0 {
            return Err(
                BalanceError::InsufficientAvailable {
                    asset:
                        settlement
                            .quote_asset
                            .clone(),
                    available:
                        current_quote.available,
                    requested:
                        settlement
                            .quote_delta
                            .unsigned_abs(),
                }
            );
        }

        /*
         * Both legs have passed validation.
         *
         * Now mutate the ledger.
         */
        balances
            .entry(asset_key)
            .or_default()
            .available = new_asset;

        balances
            .entry(quote_key)
            .or_default()
            .available = new_quote;

        Ok(())
    }
}

impl Default for BalanceLedger {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub enum BalanceError {
    InvalidAmount,

    Overflow,

    InsufficientAvailable {
        asset: String,
        available: i128,
        requested: u128,
    },

    InsufficientLocked {
        asset: String,
        locked: i128,
        requested: i128,
    },

    SettlementNotFinal,
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::PEPDEX::settlement::{
        SettlementResult,
        SettlementStatus,
    };

    #[test]
    fn deposit_and_withdraw() {
        let ledger =
            BalanceLedger::new();

        ledger
            .deposit(
                "alice",
                "USDT",
                1000,
            )
            .expect("deposit failed");

        assert_eq!(
            ledger.available(
                "alice",
                "USDT"
            ),
            1000
        );

        ledger
            .withdraw(
                "alice",
                "USDT",
                300,
            )
            .expect("withdraw failed");

        assert_eq!(
            ledger.available(
                "alice",
                "USDT"
            ),
            700
        );
    }

    #[test]
    fn cannot_withdraw_more_than_available() {
        let ledger =
            BalanceLedger::new();

        ledger
            .deposit(
                "alice",
                "USDT",
                100,
            )
            .unwrap();

        let result =
            ledger.withdraw(
                "alice",
                "USDT",
                101,
            );

        assert!(matches!(
            result,
            Err(
                BalanceError
                    ::InsufficientAvailable { .. }
            )
        ));
    }

    #[test]
    fn lock_and_unlock() {
        let ledger =
            BalanceLedger::new();

        ledger
            .deposit(
                "alice",
                "USDT",
                1000,
            )
            .unwrap();

        ledger
            .lock(
                "alice",
                "USDT",
                400,
            )
            .unwrap();

        let balance =
            ledger.get(
                "alice",
                "USDT"
            );

        assert_eq!(
            balance.available,
            600
        );

        assert_eq!(
            balance.locked,
            400
        );

        ledger
            .unlock(
                "alice",
                "USDT",
                200,
            )
            .unwrap();

        let balance =
            ledger.get(
                "alice",
                "USDT"
            );

        assert_eq!(
            balance.available,
            800
        );

        assert_eq!(
            balance.locked,
            200
        );
    }

    #[test]
    fn buy_settlement_updates_two_assets() {
        let ledger =
            BalanceLedger::new();

        ledger
            .deposit(
                "alice",
                "USDT",
                100_000,
            )
            .unwrap();

        let settlement =
            SettlementResult {
                parent_order_id:
                    crate::PEPDEX::order::OrderId
                        ::new("ORDER-1"),

                user:
                    "alice".to_string(),

                asset:
                    "PEP".to_string(),

                quote_asset:
                    "USDT".to_string(),

                asset_delta:
                    100,

                quote_delta:
                    -60_220,

                execution_quantity:
                    100,

                execution_cost:
                    60_200,

                venue_fee:
                    20,

                status:
                    SettlementStatus::Settled,
            };

        ledger
            .apply_settlement(
                &settlement
            )
            .expect(
                "settlement failed"
            );

        assert_eq!(
            ledger.available(
                "alice",
                "PEP"
            ),
            100
        );

        assert_eq!(
            ledger.available(
                "alice",
                "USDT"
            ),
            39_780
        );
    }

    #[test]
    fn failed_settlement_does_not_change_balance() {
        let ledger =
            BalanceLedger::new();

        ledger
            .deposit(
                "alice",
                "USDT",
                1000,
            )
            .unwrap();

        let settlement =
            SettlementResult {
                parent_order_id:
                    crate::PEPDEX::order::OrderId
                        ::new("ORDER-2"),

                user:
                    "alice".to_string(),

                asset:
                    "PEP".to_string(),

                quote_asset:
                    "USDT".to_string(),

                asset_delta:
                    100,

                quote_delta:
                    -500,

                execution_quantity:
                    100,

                execution_cost:
                    500,

                venue_fee:
                    0,

                status:
                    SettlementStatus::Failed,
            };

        let result =
            ledger.apply_settlement(
                &settlement
            );

        assert!(matches!(
            result,
            Err(
                BalanceError
                    ::SettlementNotFinal
            )
        ));

        assert_eq!(
            ledger.available(
                "alice",
                "USDT"
            ),
            1000
        );

        assert_eq!(
            ledger.available(
                "alice",
                "PEP"
            ),
            0
        );
    }
}