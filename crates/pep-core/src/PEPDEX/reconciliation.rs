use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::PEPDEX::execution::{
    Execution,
    ExecutionStatus,
};
use crate::PEPDEX::order::{
    ChildOrder,
    OrderId,
};
use crate::PEPDEX::venue::{
    Venue,
    VenueError,
    VenueId,
    VenueOrder,
    VenueOrderStatus,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReconciliationStatus {
    Unknown,
    Pending,
    Matched,
    Updated,
    Missing,
    Conflict,
    Failed,
}

#[derive(Debug, Clone)]
pub struct ReconciliationRecord {
    pub child_order_id: OrderId,

    pub parent_order_id: OrderId,

    pub venue: VenueId,

    pub external_order_id: String,

    pub status: ReconciliationStatus,

    pub last_known_execution:
        Option<Execution>,

    pub attempts: u32,

    pub last_error:
        Option<String>,
}

impl ReconciliationRecord {
    pub fn new(
        child: &ChildOrder,
        external_order_id: impl Into<String>,
    ) -> Self {
        Self {
            child_order_id:
                child.id.clone(),

            parent_order_id:
                child.parent_id.clone(),

            venue:
                child.venue,

            external_order_id:
                external_order_id.into(),

            status:
                ReconciliationStatus::Unknown,

            last_known_execution:
                None,

            attempts: 0,

            last_error:
                None,
        }
    }
}

#[derive(Debug, Default)]
pub struct ReconciliationStore {
    records:
        Arc<RwLock<
            HashMap<
                OrderId,
                ReconciliationRecord
            >
        >>,
}

impl ReconciliationStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(
        &self,
        record: ReconciliationRecord,
    ) {
        let mut records =
            self.records
                .write()
                .expect(
                    "reconciliation store poisoned"
                );

        records.insert(
            record.child_order_id.clone(),
            record,
        );
    }

    pub fn get(
        &self,
        child_order_id: &OrderId,
    ) -> Option<ReconciliationRecord> {
        let records =
            self.records
                .read()
                .expect(
                    "reconciliation store poisoned"
                );

        records
            .get(child_order_id)
            .cloned()
    }

    pub fn update(
        &self,
        record: ReconciliationRecord,
    ) {
        let mut records =
            self.records
                .write()
                .expect(
                    "reconciliation store poisoned"
                );

        records.insert(
            record.child_order_id.clone(),
            record,
        );
    }

    pub fn pending(
        &self,
    ) -> Vec<ReconciliationRecord> {
        let records =
            self.records
                .read()
                .expect(
                    "reconciliation store poisoned"
                );

        records
            .values()
            .filter(|record| {
                matches!(
                    record.status,
                    ReconciliationStatus::Unknown
                        | ReconciliationStatus::Pending
                        | ReconciliationStatus::Conflict
                )
            })
            .cloned()
            .collect()
    }
}

pub struct ReconciliationEngine {
    store:
        ReconciliationStore,
}

impl ReconciliationEngine {
    pub fn new() -> Self {
        Self {
            store:
                ReconciliationStore::new(),
        }
    }

    pub fn store(
        &self,
    ) -> &ReconciliationStore {
        &self.store
    }

    /// Reconcile one child order against its external venue.
    ///
    /// PEPDEX does NOT assume that a timeout means failure.
    ///
    /// It asks the venue:
    ///
    /// "What is the state of external_order_id?"
    pub fn reconcile<V>(
        &self,
        venue: &V,
        record: &mut ReconciliationRecord,
    ) -> Result<ReconciliationResult, ReconciliationError>
    where
        V: Venue + ?Sized,
    {
        record.attempts += 1;
        record.status =
            ReconciliationStatus::Pending;

        let external =
            venue
                .get_order(
                    &record.external_order_id
                )
                .map_err(|error| {
                    record.status =
                        ReconciliationStatus
                            ::Failed;

                    record.last_error =
                        Some(
                            format!("{:?}", error)
                        );

                    ReconciliationError
                        ::Venue(error)
                })?;

        let execution =
            normalize_external_order(
                &external,
                record,
            )?;

        /*
         * Compare what PEPDEX previously knew
         * with what the venue currently reports.
         */
        let status =
            match &record.last_known_execution {
                None => {
                    ReconciliationStatus
                        ::Matched
                }

                Some(previous) => {
                    compare_execution(
                        previous,
                        &execution,
                    )
                }
            };

        record.status =
            status;

        record.last_known_execution =
            Some(execution.clone());

        record.last_error =
            None;

        self.store.update(
            record.clone()
        );

        Ok(
            ReconciliationResult {
                child_order_id:
                    record
                        .child_order_id
                        .clone(),

                parent_order_id:
                    record
                        .parent_order_id
                        .clone(),

                venue:
                    record.venue,

                external_order_id:
                    record
                        .external_order_id
                        .clone(),

                status,

                execution,
            }
        )
    }
}

impl Default for ReconciliationEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct ReconciliationResult {
    pub child_order_id: OrderId,

    pub parent_order_id: OrderId,

    pub venue: VenueId,

    pub external_order_id: String,

    pub status: ReconciliationStatus,

    pub execution: Execution,
}

fn normalize_external_order(
    external: &VenueOrder,
    record: &ReconciliationRecord,
) -> Result<Execution, ReconciliationError> {
    if external.external_order_id
        != record.external_order_id
    {
        return Err(
            ReconciliationError
                ::ExternalOrderMismatch
        );
    }

    let requested =
        external.requested_quantity;

    let filled =
        external.filled_quantity;

    if filled > requested {
        return Err(
            ReconciliationError
                ::InvalidExternalQuantity
        );
    }

    let remaining =
        requested.saturating_sub(
            filled
        );

    let status =
        match external.status {
            VenueOrderStatus::New => {
                ExecutionStatus::Submitted
            }

            VenueOrderStatus::Open => {
                ExecutionStatus::Open
            }

            VenueOrderStatus::PartiallyFilled => {
                ExecutionStatus::PartiallyFilled
            }

            VenueOrderStatus::Filled => {
                ExecutionStatus::Filled
            }

            VenueOrderStatus::Cancelled => {
                ExecutionStatus::Cancelled
            }

            VenueOrderStatus::Rejected => {
                ExecutionStatus::Rejected
            }
        };

    Ok(
        Execution {
            child_order_id:
                record
                    .child_order_id
                    .clone(),

            parent_order_id:
                record
                    .parent_order_id
                    .clone(),

            venue:
                record.venue,

            requested_quantity:
                requested,

            filled_quantity:
                filled,

            remaining_quantity:
                remaining,

            average_price:
                external.average_price,

            venue_fee: 0,

            status,

            external_order_id:
                Some(
                    external
                        .external_order_id
                        .clone()
                ),
        }
    )
}

fn compare_execution(
    previous: &Execution,
    current: &Execution,
) -> ReconciliationStatus {
    /*
     * Same state:
     */
    if previous.filled_quantity
        == current.filled_quantity
        && previous.status
            == current.status
    {
        return ReconciliationStatus
            ::Matched;
    }

    /*
     * Venue reports more filled quantity.
     *
     * This is normal after a timeout:
     *
     * PEPDEX:
     *     filled = 20
     *
     * Venue:
     *     filled = 60
     *
     * Therefore update local state.
     */
    if current.filled_quantity
        > previous.filled_quantity
    {
        return ReconciliationStatus
            ::Updated;
    }

    /*
     * Venue reports less than PEPDEX previously
     * believed.
     *
     * This is dangerous and should not be silently
     * accepted.
     */
    if current.filled_quantity
        < previous.filled_quantity
    {
        return ReconciliationStatus
            ::Conflict;
    }

    ReconciliationStatus::Updated
}

#[derive(Debug)]
pub enum ReconciliationError {
    Venue(VenueError),

    ExternalOrderMismatch,

    InvalidExternalQuantity,
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::PEPDEX::execution::{
        Execution,
        ExecutionStatus,
    };

    #[test]
    fn new_record_is_unknown() {
        let child = ChildOrder {
            id:
                OrderId::new(
                    "CHILD-1"
                ),

            parent_id:
                OrderId::new(
                    "PARENT-1"
                ),

            venue:
                VenueId::Aster,

            market:
                crate::PEPDEX::market
                    ::MarketId::new(
                        "PEP",
                        "USDT",
                    ),

            side:
                crate::PEPDEX::venue
                    ::VenueOrderSide::Buy,

            order_type:
                crate::PEPDEX::venue
                    ::VenueOrderType::Market,

            quantity: 100,

            price: None,

            client_order_id:
                "PEPDEX-CHILD-1"
                    .to_string(),

            timestamp_ms: 0,

            status:
                crate::PEPDEX::order
                    ::ChildOrderStatus
                    ::Pending,
        };

        let record =
            ReconciliationRecord::new(
                &child,
                "ASTER-123",
            );

        assert_eq!(
            record.status,
            ReconciliationStatus::Unknown
        );

        assert_eq!(
            record.external_order_id,
            "ASTER-123"
        );
    }

    #[test]
    fn detects_new_fill() {
        let previous =
            Execution {
                child_order_id:
                    OrderId::new(
                        "CHILD"
                    ),

                parent_order_id:
                    OrderId::new(
                        "PARENT"
                    ),

                venue:
                    VenueId::Aster,

                requested_quantity:
                    100,

                filled_quantity:
                    20,

                remaining_quantity:
                    80,

                average_price:
                    Some(600),

                venue_fee: 1,

                status:
                    ExecutionStatus
                        ::PartiallyFilled,

                external_order_id:
                    Some(
                        "A-1".into()
                    ),
            };

        let current =
            Execution {
                child_order_id:
                    previous
                        .child_order_id
                        .clone(),

                parent_order_id:
                    previous
                        .parent_order_id
                        .clone(),

                venue:
                    VenueId::Aster,

                requested_quantity:
                    100,

                filled_quantity:
                    60,

                remaining_quantity:
                    40,

                average_price:
                    Some(601),

                venue_fee: 2,

                status:
                    ExecutionStatus
                        ::PartiallyFilled,

                external_order_id:
                    Some(
                        "A-1".into()
                    ),
            };

        assert_eq!(
            compare_execution(
                &previous,
                &current,
            ),
            ReconciliationStatus::Updated
        );
    }

    #[test]
    fn detects_conflicting_lower_fill() {
        let previous =
            Execution {
                child_order_id:
                    OrderId::new(
                        "CHILD"
                    ),

                parent_order_id:
                    OrderId::new(
                        "PARENT"
                    ),

                venue:
                    VenueId::Aster,

                requested_quantity:
                    100,

                filled_quantity:
                    60,

                remaining_quantity:
                    40,

                average_price:
                    Some(600),

                venue_fee: 1,

                status:
                    ExecutionStatus
                        ::PartiallyFilled,

                external_order_id:
                    Some(
                        "A-1".into()
                    ),
            };

        let current =
            Execution {
                child_order_id:
                    previous
                        .child_order_id
                        .clone(),

                parent_order_id:
                    previous
                        .parent_order_id
                        .clone(),

                venue:
                    VenueId::Aster,

                requested_quantity:
                    100,

                filled_quantity:
                    20,

                remaining_quantity:
                    80,

                average_price:
                    Some(600),

                venue_fee: 1,

                status:
                    ExecutionStatus
                        ::PartiallyFilled,

                external_order_id:
                    Some(
                        "A-1".into()
                    ),
            };

        assert_eq!(
            compare_execution(
                &previous,
                &current,
            ),
            ReconciliationStatus::Conflict
        );
    }

    #[test]
    fn detects_same_state() {
        let execution =
            Execution {
                child_order_id:
                    OrderId::new(
                        "CHILD"
                    ),

                parent_order_id:
                    OrderId::new(
                        "PARENT"
                    ),

                venue:
                    VenueId::Aster,

                requested_quantity:
                    100,

                filled_quantity:
                    100,

                remaining_quantity:
                    0,

                average_price:
                    Some(602),

                venue_fee: 1,

                status:
                    ExecutionStatus
                        ::Filled,

                external_order_id:
                    Some(
                        "A-1".into()
                    ),
            };

        assert_eq!(
            compare_execution(
                &execution,
                &execution,
            ),
            ReconciliationStatus::Matched
        );
    }
}