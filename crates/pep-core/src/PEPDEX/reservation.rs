use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::PEPDEX::balance::{
    BalanceError,
    BalanceLedger,
};
use crate::PEPDEX::order::{
    OrderId,
    ParentOrder,
};
use crate::PEPDEX::venue::VenueOrderSide;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ReservationId(pub String);

impl ReservationId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone)]
pub struct Reservation {
    pub id: ReservationId,
    pub parent_order_id: OrderId,
    pub user: String,

    pub asset: String,

    /// Amount moved from available → locked.
    pub amount: i128,

    /// Amount already consumed by settlement.
    pub consumed: i128,

    pub status: ReservationStatus,
}

impl Reservation {
    pub fn remaining(&self) -> i128 {
        self.amount.saturating_sub(self.consumed)
    }

    pub fn is_finished(&self) -> bool {
        matches!(
            self.status,
            ReservationStatus::Consumed
                | ReservationStatus::Released
                | ReservationStatus::Cancelled
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReservationStatus {
    Active,
    PartiallyConsumed,
    Consumed,
    Released,
    Cancelled,
}

pub struct ReservationManager {
    balances: BalanceLedger,

    reservations:
        Arc<RwLock<HashMap<ReservationId, Reservation>>>,
}

impl ReservationManager {
    pub fn new(
        balances: BalanceLedger,
    ) -> Self {
        Self {
            balances,

            reservations: Arc::new(
                RwLock::new(HashMap::new())
            ),
        }
    }

    pub fn reserve(
        &self,
        order: &ParentOrder,
        asset: impl Into<String>,
        amount: i128,
    ) -> Result<ReservationId, ReservationError> {
        if amount <= 0 {
            return Err(
                ReservationError::InvalidAmount
            );
        }

        /*
         * First lock the actual balance.
         *
         * BalanceLedger guarantees that another
         * concurrent reservation cannot consume
         * the same available balance.
         */
        let asset = asset.into();

        self.balances
            .lock(
                &order.user,
                &asset,
                amount,
            )
            .map_err(
                ReservationError::Balance
            )?;

        let reservation_id =
            ReservationId::new(
                format!(
                    "{}-RESERVE",
                    order.id.as_str()
                )
            );

        let reservation =
            Reservation {
                id:
                    reservation_id.clone(),

                parent_order_id:
                    order.id.clone(),

                user:
                    order.user.clone(),

                asset,

                amount,

                consumed: 0,

                status:
                    ReservationStatus::Active,
            };

        let mut reservations =
            self.reservations
                .write()
                .expect(
                    "reservation store poisoned"
                );

        reservations.insert(
            reservation_id.clone(),
            reservation,
        );

        Ok(reservation_id)
    }

    pub fn get(
        &self,
        id: &ReservationId,
    ) -> Option<Reservation> {
        let reservations =
            self.reservations
                .read()
                .expect(
                    "reservation store poisoned"
                );

        reservations.get(id).cloned()
    }

    pub fn consume(
        &self,
        id: &ReservationId,
        amount: i128,
    ) -> Result<(), ReservationError> {
        if amount <= 0 {
            return Err(
                ReservationError::InvalidAmount
            );
        }

        let mut reservations =
            self.reservations
                .write()
                .expect(
                    "reservation store poisoned"
                );

        let reservation =
            reservations
                .get_mut(id)
                .ok_or(
                    ReservationError::NotFound
                )?;

        if reservation.is_finished() {
            return Err(
                ReservationError::AlreadyFinished
            );
        }

        if amount > reservation.remaining() {
            return Err(
                ReservationError::ExceedsReservation
            );
        }

        reservation.consumed += amount;

        if reservation.consumed
            == reservation.amount
        {
            reservation.status =
                ReservationStatus::Consumed;
        } else {
            reservation.status =
                ReservationStatus
                    ::PartiallyConsumed;
        }

        Ok(())
    }

    /// Release unused locked balance.
    pub fn release(
        &self,
        id: &ReservationId,
    ) -> Result<i128, ReservationError> {
        let mut reservations =
            self.reservations
                .write()
                .expect(
                    "reservation store poisoned"
                );

        let reservation =
            reservations
                .get_mut(id)
                .ok_or(
                    ReservationError::NotFound
                )?;

        if matches!(
            reservation.status,
            ReservationStatus::Released
                | ReservationStatus::Cancelled
        ) {
            return Err(
                ReservationError::AlreadyFinished
            );
        }

        let remaining =
            reservation.remaining();

        if remaining > 0 {
            self.balances
                .unlock(
                    &reservation.user,
                    &reservation.asset,
                    remaining,
                )
                .map_err(
                    ReservationError::Balance
                )?;
        }

        reservation.status =
            ReservationStatus::Released;

        Ok(remaining)
    }

    pub fn cancel(
        &self,
        id: &ReservationId,
    ) -> Result<i128, ReservationError> {
        let mut reservations =
            self.reservations
                .write()
                .expect(
                    "reservation store poisoned"
                );

        let reservation =
            reservations
                .get_mut(id)
                .ok_or(
                    ReservationError::NotFound
                )?;

        if reservation.is_finished() {
            return Err(
                ReservationError::AlreadyFinished
            );
        }

        let remaining =
            reservation.remaining();

        if remaining > 0 {
            self.balances
                .unlock(
                    &reservation.user,
                    &reservation.asset,
                    remaining,
                )
                .map_err(
                    ReservationError::Balance
                )?;
        }

        reservation.status =
            ReservationStatus::Cancelled;

        Ok(remaining)
    }

    pub fn active_for_user(
        &self,
        user: &str,
    ) -> Vec<Reservation> {
        let reservations =
            self.reservations
                .read()
                .expect(
                    "reservation store poisoned"
                );

        reservations
            .values()
            .filter(|reservation| {
                reservation.user == user
                    && !reservation.is_finished()
            })
            .cloned()
            .collect()
    }
}

#[derive(Debug)]
pub enum ReservationError {
    InvalidAmount,

    NotFound,

    AlreadyFinished,

    ExceedsReservation,

    Balance(BalanceError),
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::PEPDEX::market::MarketId;
    use crate::PEPDEX::order::ParentOrder;
    use crate::PEPDEX::venue::{
        VenueOrderSide,
        VenueOrderType,
    };

    fn order() -> ParentOrder {
        ParentOrder::new(
            "ORDER-1",
            "alice",
            MarketId::new(
                "PEP",
                "USDT",
            ),
            VenueOrderSide::Buy,
            VenueOrderType::Market,
            100,
            None,
        )
    }

    #[test]
    fn reserve_moves_balance_to_locked() {
        let balances =
            BalanceLedger::new();

        balances
            .deposit(
                "alice",
                "USDT",
                1000,
            )
            .unwrap();

        let manager =
            ReservationManager::new(
                balances.clone()
            );

        let reservation =
            manager
                .reserve(
                    &order(),
                    "USDT",
                    600,
                )
                .expect(
                    "reservation failed"
                );

        let balance =
            balances.get(
                "alice",
                "USDT",
            );

        assert_eq!(
            balance.available,
            400
        );

        assert_eq!(
            balance.locked,
            600
        );

        let stored =
            manager
                .get(&reservation)
                .unwrap();

        assert_eq!(
            stored.amount,
            600
        );

        assert_eq!(
            stored.remaining(),
            600
        );
    }

    #[test]
    fn reservation_cannot_exceed_balance() {
        let balances =
            BalanceLedger::new();

        balances
            .deposit(
                "alice",
                "USDT",
                500,
            )
            .unwrap();

        let manager =
            ReservationManager::new(
                balances
            );

        let result =
            manager.reserve(
                &order(),
                "USDT",
                501,
            );

        assert!(matches!(
            result,
            Err(
                ReservationError::Balance(
                    BalanceError
                        ::InsufficientAvailable { .. }
                )
            )
        ));
    }

    #[test]
    fn consume_tracks_used_amount() {
        let balances =
            BalanceLedger::new();

        balances
            .deposit(
                "alice",
                "USDT",
                1000,
            )
            .unwrap();

        let manager =
            ReservationManager::new(
                balances
            );

        let id =
            manager
                .reserve(
                    &order(),
                    "USDT",
                    600,
                )
                .unwrap();

        manager
            .consume(&id, 400)
            .unwrap();

        let reservation =
            manager.get(&id).unwrap();

        assert_eq!(
            reservation.consumed,
            400
        );

        assert_eq!(
            reservation.remaining(),
            200
        );

        assert_eq!(
            reservation.status,
            ReservationStatus
                ::PartiallyConsumed
        );
    }

    #[test]
    fn release_returns_unused_locked_balance() {
        let balances =
            BalanceLedger::new();

        balances
            .deposit(
                "alice",
                "USDT",
                1000,
            )
            .unwrap();

        let manager =
            ReservationManager::new(
                balances.clone()
            );

        let id =
            manager
                .reserve(
                    &order(),
                    "USDT",
                    600,
                )
                .unwrap();

        manager
            .consume(&id, 400)
            .unwrap();

        let released =
            manager
                .release(&id)
                .unwrap();

        assert_eq!(
            released,
            200
        );

        let balance =
            balances.get(
                "alice",
                "USDT",
            );

        /*
         * Initial: 1000
         *
         * Reserve 600:
         * available = 400
         * locked    = 600
         *
         * Consume 400:
         * locked remains 600 because the actual
         * settlement accounting happens separately.
         *
         * Release remaining reservation:
         * unlock 200
         *
         * available = 600
         * locked    = 400
         */
        assert_eq!(
            balance.available,
            600
        );

        assert_eq!(
            balance.locked,
            400
        );
    }

    #[test]
    fn cancel_unlocks_full_remaining_amount() {
        let balances =
            BalanceLedger::new();

        balances
            .deposit(
                "alice",
                "USDT",
                1000,
            )
            .unwrap();

        let manager =
            ReservationManager::new(
                balances.clone()
            );

        let id =
            manager
                .reserve(
                    &order(),
                    "USDT",
                    500,
                )
                .unwrap();

        let released =
            manager
                .cancel(&id)
                .unwrap();

        assert_eq!(
            released,
            500
        );

        let balance =
            balances.get(
                "alice",
                "USDT",
            );

        assert_eq!(
            balance.available,
            1000
        );

        assert_eq!(
            balance.locked,
            0
        );
    }

    #[test]
    fn double_consume_is_rejected() {
        let balances =
            BalanceLedger::new();

        balances
            .deposit(
                "alice",
                "USDT",
                1000,
            )
            .unwrap();

        let manager =
            ReservationManager::new(
                balances
            );

        let id =
            manager
                .reserve(
                    &order(),
                    "USDT",
                    500,
                )
                .unwrap();

        manager
            .consume(&id, 500)
            .unwrap();

        let result =
            manager.consume(&id, 1);

        assert!(matches!(
            result,
            Err(
                ReservationError
                    ::AlreadyFinished
            )
        ));
    }
}