use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use crate::PEPDEX::order::{
    ChildOrderStatus,
    OrderId,
    ParentOrder,
    ParentOrderStatus,
};
use crate::PEPDEX::order_manager::{
    ManagedOrder,
    OrderLifecycle,
};
use crate::PEPDEX::venue::{
    VenueId,
    VenueOrderSide,
    VenueOrderType,
};

/// ============================================================
/// STORED ORDER
/// ============================================================

#[derive(Debug, Clone)]
pub struct StoredOrder {
    pub order: ParentOrder,

    pub lifecycle: OrderLifecycle,

    pub reservation_id: Option<String>,

    pub child_orders: Vec<StoredChildOrder>,

    pub external_orders: Vec<StoredExternalOrder>,

    pub last_error: Option<String>,

    pub created_at_ms: u64,

    pub updated_at_ms: u64,
}

impl StoredOrder {
    pub fn from_managed(
        managed: &ManagedOrder,
    ) -> Self {
        let child_orders = managed
            .execution_plan
            .as_ref()
            .map(|plan| {
                plan.children
                    .iter()
                    .map(|child| {
                        StoredChildOrder {
                            child_order_id:
                                child.id.clone(),

                            venue:
                                child.venue,

                            client_order_id:
                                child.client_order_id
                                    .clone(),

                            quantity:
                                child.quantity,

                            price:
                                child.price,

                            status:
                                format!(
                                    "{:?}",
                                    child.status
                                ),
                        }
                    })
                    .collect()
            })
            .unwrap_or_default();

        let external_orders = managed
            .executions
            .iter()
            .filter_map(|execution| {
                execution
                    .external_order_id
                    .as_ref()
                    .map(|external_id| {
                        StoredExternalOrder {
                            child_order_id:
                                execution
                                    .child_order_id
                                    .clone(),

                            venue:
                                execution.venue,

                            external_order_id:
                                external_id.clone(),

                            filled_quantity:
                                execution
                                    .filled_quantity,

                            remaining_quantity:
                                execution
                                    .remaining_quantity,

                            status:
                                format!(
                                    "{:?}",
                                    execution.status
                                ),
                        }
                    })
            })
            .collect();

        let now =
            current_timestamp_ms();

        Self {
            order:
                managed.order.clone(),

            lifecycle:
                managed.lifecycle,

            reservation_id:
                managed
                    .reservation_id
                    .as_ref()
                    .map(|id| {
                        id.as_str().to_string()
                    }),

            child_orders,

            external_orders,

            last_error:
                None,

            created_at_ms:
                managed.order.timestamp_ms,

            updated_at_ms:
                now,
        }
    }

    pub fn is_reconcilable(
        &self,
    ) -> bool {
        matches!(
            self.lifecycle,
            OrderLifecycle::Executing
                | OrderLifecycle::Aggregated
                | OrderLifecycle::Failed
        ) && !self.external_orders.is_empty()
    }
}

#[derive(Debug, Clone)]
pub struct StoredChildOrder {
    pub child_order_id: OrderId,

    pub venue: VenueId,

    pub client_order_id: String,

    pub quantity: u64,

    pub price: Option<u64>,

    pub status: String,
}

#[derive(Debug, Clone)]
pub struct StoredExternalOrder {
    pub child_order_id: OrderId,

    pub venue: VenueId,

    pub external_order_id: String,

    pub filled_quantity: u64,

    pub remaining_quantity: u64,

    pub status: String,
}

/// ============================================================
/// STORE TRAIT
/// ============================================================

pub trait OrderStoreBackend:
    Send + Sync
{
    fn insert(
        &self,
        order: StoredOrder,
    ) -> Result<(), OrderStoreError>;

    fn get(
        &self,
        order_id: &OrderId,
    ) -> Result<
        Option<StoredOrder>,
        OrderStoreError,
    >;

    fn update(
        &self,
        order: StoredOrder,
    ) -> Result<(), OrderStoreError>;

    fn remove(
        &self,
        order_id: &OrderId,
    ) -> Result<
        Option<StoredOrder>,
        OrderStoreError,
    >;

    fn all(
        &self,
    ) -> Result<
        Vec<StoredOrder>,
        OrderStoreError,
    >;

    fn pending_reconciliation(
        &self,
    ) -> Result<
        Vec<StoredOrder>,
        OrderStoreError,
    >;
}

/// ============================================================
/// HIGH LEVEL ORDER STORE
/// ============================================================

#[derive(Clone)]
pub struct OrderStore {
    backend:
        Arc<dyn OrderStoreBackend>,
}

impl OrderStore {
    pub fn memory() -> Self {
        Self {
            backend:
                Arc::new(
                    MemoryOrderStore::new()
                ),
        }
    }

    pub fn file(
        path: impl Into<PathBuf>,
    ) -> Result<Self, OrderStoreError> {
        let backend =
            FileOrderStore::open(
                path.into()
            )?;

        Ok(Self {
            backend:
                Arc::new(backend),
        })
    }

    pub fn insert(
        &self,
        order: StoredOrder,
    ) -> Result<(), OrderStoreError> {
        self.backend.insert(order)
    }

    pub fn save_managed(
        &self,
        managed: &ManagedOrder,
    ) -> Result<(), OrderStoreError> {
        self.insert(
            StoredOrder::from_managed(
                managed
            )
        )
    }

    pub fn get(
        &self,
        order_id: &OrderId,
    ) -> Result<
        Option<StoredOrder>,
        OrderStoreError,
    > {
        self.backend.get(order_id)
    }

    pub fn update(
        &self,
        order: StoredOrder,
    ) -> Result<(), OrderStoreError> {
        self.backend.update(order)
    }

    pub fn update_lifecycle(
        &self,
        order_id: &OrderId,
        lifecycle: OrderLifecycle,
    ) -> Result<(), OrderStoreError> {
        let mut order =
            self.get(order_id)?
                .ok_or(
                    OrderStoreError::NotFound
                )?;

        order.lifecycle =
            lifecycle;

        order.updated_at_ms =
            current_timestamp_ms();

        self.update(order)
    }

    pub fn set_error(
        &self,
        order_id: &OrderId,
        error: impl Into<String>,
    ) -> Result<(), OrderStoreError> {
        let mut order =
            self.get(order_id)?
                .ok_or(
                    OrderStoreError::NotFound
                )?;

        order.last_error =
            Some(error.into());

        order.updated_at_ms =
            current_timestamp_ms();

        self.update(order)
    }

    pub fn remove(
        &self,
        order_id: &OrderId,
    ) -> Result<
        Option<StoredOrder>,
        OrderStoreError,
    > {
        self.backend.remove(order_id)
    }

    pub fn all(
        &self,
    ) -> Result<
        Vec<StoredOrder>,
        OrderStoreError,
    > {
        self.backend.all()
    }

    pub fn pending_reconciliation(
        &self,
    ) -> Result<
        Vec<StoredOrder>,
        OrderStoreError,
    > {
        self.backend
            .pending_reconciliation()
    }
}

/// ============================================================
/// MEMORY BACKEND
/// ============================================================

#[derive(Debug, Default)]
pub struct MemoryOrderStore {
    orders:
        RwLock<
            HashMap<OrderId, StoredOrder>
        >,
}

impl MemoryOrderStore {
    pub fn new() -> Self {
        Self::default()
    }
}

impl OrderStoreBackend
    for MemoryOrderStore
{
    fn insert(
        &self,
        order: StoredOrder,
    ) -> Result<(), OrderStoreError> {
        let mut orders =
            self.orders
                .write()
                .map_err(|_| {
                    OrderStoreError
                        ::LockPoisoned
                })?;

        orders.insert(
            order.order.id.clone(),
            order,
        );

        Ok(())
    }

    fn get(
        &self,
        order_id: &OrderId,
    ) -> Result<
        Option<StoredOrder>,
        OrderStoreError,
    > {
        let orders =
            self.orders
                .read()
                .map_err(|_| {
                    OrderStoreError
                        ::LockPoisoned
                })?;

        Ok(
            orders
                .get(order_id)
                .cloned()
        )
    }

    fn update(
        &self,
        order: StoredOrder,
    ) -> Result<(), OrderStoreError> {
        self.insert(order)
    }

    fn remove(
        &self,
        order_id: &OrderId,
    ) -> Result<
        Option<StoredOrder>,
        OrderStoreError,
    > {
        let mut orders =
            self.orders
                .write()
                .map_err(|_| {
                    OrderStoreError
                        ::LockPoisoned
                })?;

        Ok(
            orders.remove(order_id)
        )
    }

    fn all(
        &self,
    ) -> Result<
        Vec<StoredOrder>,
        OrderStoreError,
    > {
        let orders =
            self.orders
                .read()
                .map_err(|_| {
                    OrderStoreError
                        ::LockPoisoned
                })?;

        Ok(
            orders
                .values()
                .cloned()
                .collect()
        )
    }

    fn pending_reconciliation(
        &self,
    ) -> Result<
        Vec<StoredOrder>,
        OrderStoreError,
    > {
        Ok(
            self.all()?
                .into_iter()
                .filter(
                    |order| {
                        order
                            .is_reconcilable()
                    }
                )
                .collect()
        )
    }
}

/// ============================================================
/// FILE BACKEND
///
/// Append-only journal:
///
/// I|order...
/// U|order...
/// D|order_id
///
/// On startup the journal is replayed into memory.
///
/// This gives us persistence without forcing a database
/// dependency into PEPDEX yet.
/// ============================================================

pub struct FileOrderStore {
    path: PathBuf,

    orders:
        RwLock<
            HashMap<OrderId, StoredOrder>
        >,
}

impl FileOrderStore {
    pub fn open(
        path: PathBuf,
    ) -> Result<Self, OrderStoreError> {
        if let Some(parent) =
            path.parent()
        {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)
                    .map_err(
                        OrderStoreError
                            ::Io
                    )?;
            }
        }

        if !path.exists() {
            File::create(&path)
                .map_err(
                    OrderStoreError
                        ::Io
                )?;
        }

        let store =
            Self {
                path,
                orders:
                    RwLock::new(
                        HashMap::new()
                    ),
            };

        store.replay()?;

        Ok(store)
    }

    fn replay(
        &self,
    ) -> Result<(), OrderStoreError> {
        let file =
            File::open(&self.path)
                .map_err(
                    OrderStoreError
                        ::Io
                )?;

        let reader =
            BufReader::new(file);

        let mut orders =
            self.orders
                .write()
                .map_err(|_| {
                    OrderStoreError
                        ::LockPoisoned
                })?;

        for line in reader.lines() {
            let line =
                line.map_err(
                    OrderStoreError
                        ::Io
                )?;

            if line.is_empty() {
                continue;
            }

            let parts:
                Vec<&str> =
                line.split('|')
                    .collect();

            match parts.first()
                .copied()
            {
                Some("I")
                | Some("U") => {
                    let order =
                        decode_order(
                            &parts[1..]
                        )?;

                    orders.insert(
                        order.order.id.clone(),
                        order,
                    );
                }

                Some("D") => {
                    if parts.len() < 2 {
                        continue;
                    }

                    let id =
                        OrderId::new(
                            unescape(
                                parts[1]
                            )
                        );

                    orders.remove(&id);
                }

                _ => {
                    /*
                     * Ignore unknown journal records.
                     *
                     * This makes future journal versions
                     * forward-compatible.
                     */
                }
            }
        }

        Ok(())
    }

    fn append(
        &self,
        line: String,
    ) -> Result<(), OrderStoreError> {
        let mut file =
            OpenOptions::new()
                .create(true)
                .append(true)
                .open(&self.path)
                .map_err(
                    OrderStoreError
                        ::Io
                )?;

        file.write_all(
            line.as_bytes()
        )
        .map_err(
            OrderStoreError::Io
        )?;

        file.write_all(
            b"\n"
        )
        .map_err(
            OrderStoreError::Io
        )?;

        file.sync_data()
            .map_err(
                OrderStoreError::Io
            )?;

        Ok(())
    }
}

impl OrderStoreBackend
    for FileOrderStore
{
    fn insert(
        &self,
        order: StoredOrder,
    ) -> Result<(), OrderStoreError> {
        let encoded =
            encode_order(
                "I",
                &order
            );

        self.append(encoded)?;

        let mut orders =
            self.orders
                .write()
                .map_err(|_| {
                    OrderStoreError
                        ::LockPoisoned
                })?;

        orders.insert(
            order.order.id.clone(),
            order,
        );

        Ok(())
    }

    fn get(
        &self,
        order_id: &OrderId,
    ) -> Result<
        Option<StoredOrder>,
        OrderStoreError,
    > {
        let orders =
            self.orders
                .read()
                .map_err(|_| {
                    OrderStoreError
                        ::LockPoisoned
                })?;

        Ok(
            orders
                .get(order_id)
                .cloned()
        )
    }

    fn update(
        &self,
        order: StoredOrder,
    ) -> Result<(), OrderStoreError> {
        let encoded =
            encode_order(
                "U",
                &order
            );

        self.append(encoded)?;

        let mut orders =
            self.orders
                .write()
                .map_err(|_| {
                    OrderStoreError
                        ::LockPoisoned
                })?;

        orders.insert(
            order.order.id.clone(),
            order,
        );

        Ok(())
    }

    fn remove(
        &self,
        order_id: &OrderId,
    ) -> Result<
        Option<StoredOrder>,
        OrderStoreError,
    > {
        let previous =
            self.get(order_id)?;

        if previous.is_none() {
            return Ok(None);
        }

        let line =
            format!(
                "D|{}",
                escape(
                    order_id.as_str()
                )
            );

        self.append(line)?;

        let mut orders =
            self.orders
                .write()
                .map_err(|_| {
                    OrderStoreError
                        ::LockPoisoned
                })?;

        Ok(
            orders.remove(order_id)
        )
    }

    fn all(
        &self,
    ) -> Result<
        Vec<StoredOrder>,
        OrderStoreError,
    > {
        let orders =
            self.orders
                .read()
                .map_err(|_| {
                    OrderStoreError
                        ::LockPoisoned
                })?;

        Ok(
            orders
                .values()
                .cloned()
                .collect()
        )
    }

    fn pending_reconciliation(
        &self,
    ) -> Result<
        Vec<StoredOrder>,
        OrderStoreError,
    > {
        Ok(
            self.all()?
                .into_iter()
                .filter(
                    |order| {
                        order
                            .is_reconcilable()
                    }
                )
                .collect()
        )
    }
}

/// ============================================================
/// SERIALIZATION
///
/// Internal journal format only.
/// ============================================================

fn encode_order(
    op: &str,
    order: &StoredOrder,
) -> String {
    let constraints =
        &order.order.constraints;

    let max_spend =
        constraints
            .max_spend
            .map(|v| v.to_string())
            .unwrap_or_default();

    let max_slippage =
        constraints
            .max_slippage_bps
            .map(|v| v.to_string())
            .unwrap_or_default();

    let max_quantity =
        constraints
            .max_quantity
            .map(|v| v.to_string())
            .unwrap_or_default();

    let limit_price =
        order
            .order
            .limit_price
            .map(|v| v.to_string())
            .unwrap_or_default();

    let reservation =
        order
            .reservation_id
            .clone()
            .unwrap_or_default();

    /*
     * Core order:
     *
     * 0  op
     * 1  order_id
     * 2  user
     * 3  base
     * 4  quote
     * 5  side
     * 6  order_type
     * 7  quantity
     * 8  limit_price
     * 9  max_spend
     * 10 max_slippage
     * 11 max_quantity
     * 12 lifecycle
     * 13 reservation
     * 14 last_error
     * 15 created_at
     * 16 updated_at
     * 17 children
     * 18 external_orders
     */
    format!(
        "{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
        op,

        escape(
            order.order.id.as_str()
        ),

        escape(
            &order.order.user
        ),

        escape(
            &order.order.market.base
        ),

        escape(
            &order.order.market.quote
        ),

        encode_side(
            order.order.side
        ),

        encode_order_type(
            order.order.order_type
        ),

        order.order.quantity,

        limit_price,

        max_spend,

        max_slippage,

        max_quantity,

        encode_lifecycle(
            order.lifecycle
        ),

        escape(
            &reservation
        ),

        escape(
            order
                .last_error
                .as_deref()
                .unwrap_or("")
        ),

        order.created_at_ms,

        order.updated_at_ms,

        encode_children(
            &order.child_orders
        ),

        encode_external_orders(
            &order.external_orders
        ),
    )
}

fn decode_order(
    fields: &[&str],
) -> Result<StoredOrder, OrderStoreError> {
    if fields.len() < 18 {
        return Err(
            OrderStoreError
                ::CorruptedRecord
        );
    }

    let order_id =
        unescape(fields[0]);

    let user =
        unescape(fields[1]);

    let base =
        unescape(fields[2]);

    let quote =
        unescape(fields[3]);

    let side =
        decode_side(fields[4])?;

    let order_type =
        decode_order_type(fields[5])?;

    let quantity =
        fields[6]
            .parse::<u64>()
            .map_err(|_| {
                OrderStoreError
                    ::CorruptedRecord
            })?;

    let limit_price =
        parse_optional_u64(
            fields[7]
        )?;

    let max_spend =
        parse_optional_u64(
            fields[8]
        )?;

    let max_slippage_bps =
        parse_optional_u64(
            fields[9]
        )?;

    let max_quantity =
        parse_optional_u64(
            fields[10]
        )?;

    let lifecycle =
        decode_lifecycle(
            fields[11]
        )?;

    let reservation =
        optional_string(
            fields[12]
        );

    let last_error =
        optional_string(
            fields[13]
        );

    let created_at_ms =
        fields[14]
            .parse::<u64>()
            .map_err(|_| {
                OrderStoreError
                    ::CorruptedRecord
            })?;

    let updated_at_ms =
        fields[15]
            .parse::<u64>()
            .map_err(|_| {
                OrderStoreError
                    ::CorruptedRecord
            })?;

    let children =
        decode_children(
            fields[16]
        )?;

    let external_orders =
        decode_external_orders(
            fields[17]
        )?;

    let order =
        ParentOrder {
            id:
                OrderId::new(
                    order_id
                ),

            user,

            market:
                crate::PEPDEX::market
                    ::MarketId::new(
                        base,
                        quote,
                    ),

            side,

            order_type,

            quantity,

            limit_price,

            constraints:
                crate::PEPDEX::risk
                    ::OrderConstraints {
                        max_spend,
                        max_slippage_bps,
                        max_quantity,
                    },

            timestamp_ms:
                created_at_ms,

            status:
                parent_status_from_lifecycle(
                    lifecycle
                ),
        };

    Ok(
        StoredOrder {
            order,

            lifecycle,

            reservation_id:
                reservation,

            child_orders:
                children,

            external_orders,

            last_error,

            created_at_ms,

            updated_at_ms,
        }
    )
}

/// ============================================================
/// CHILD / EXTERNAL ENCODING
/// ============================================================

fn encode_children(
    children: &[StoredChildOrder],
) -> String {
    children
        .iter()
        .map(|child| {
            format!(
                "{}~{}~{}~{}~{}~{}",
                escape(
                    child
                        .child_order_id
                        .as_str()
                ),
                encode_venue(
                    child.venue
                ),
                escape(
                    &child.client_order_id
                ),
                child.quantity,
                child
                    .price
                    .map(|v| v.to_string())
                    .unwrap_or_default(),
                escape(
                    &child.status
                ),
            )
        })
        .collect::<Vec<_>>()
        .join(";")
}

fn decode_children(
    encoded: &str,
) -> Result<
    Vec<StoredChildOrder>,
    OrderStoreError,
> {
    if encoded.is_empty() {
        return Ok(Vec::new());
    }

    let mut result =
        Vec::new();

    for item in encoded.split(';') {
        let fields:
            Vec<&str> =
            item.split('~')
                .collect();

        if fields.len() != 6 {
            return Err(
                OrderStoreError
                    ::CorruptedRecord
            );
        }

        result.push(
            StoredChildOrder {
                child_order_id:
                    OrderId::new(
                        unescape(fields[0])
                    ),

                venue:
                    decode_venue(
                        fields[1]
                    )?,

                client_order_id:
                    unescape(fields[2]),

                quantity:
                    fields[3]
                        .parse()
                        .map_err(|_| {
                            OrderStoreError
                                ::CorruptedRecord
                        })?,

                price:
                    parse_optional_u64(
                        fields[4]
                    )?,

                status:
                    unescape(fields[5]),
            }
        );
    }

    Ok(result)
}

fn encode_external_orders(
    orders: &[StoredExternalOrder],
) -> String {
    orders
        .iter()
        .map(|order| {
            format!(
                "{}~{}~{}~{}~{}~{}",
                escape(
                    order
                        .child_order_id
                        .as_str()
                ),
                encode_venue(
                    order.venue
                ),
                escape(
                    &order
                        .external_order_id
                ),
                order
                    .filled_quantity,
                order
                    .remaining_quantity,
                escape(
                    &order.status
                ),
            )
        })
        .collect::<Vec<_>>()
        .join(";")
}

fn decode_external_orders(
    encoded: &str,
) -> Result<
    Vec<StoredExternalOrder>,
    OrderStoreError,
> {
    if encoded.is_empty() {
        return Ok(Vec::new());
    }

    let mut result =
        Vec::new();

    for item in encoded.split(';') {
        let fields:
            Vec<&str> =
            item.split('~')
                .collect();

        if fields.len() != 6 {
            return Err(
                OrderStoreError
                    ::CorruptedRecord
            );
        }

        result.push(
            StoredExternalOrder {
                child_order_id:
                    OrderId::new(
                        unescape(fields[0])
                    ),

                venue:
                    decode_venue(
                        fields[1]
                    )?,

                external_order_id:
                    unescape(fields[2]),

                filled_quantity:
                    fields[3]
                        .parse()
                        .map_err(|_| {
                            OrderStoreError
                                ::CorruptedRecord
                        })?,

                remaining_quantity:
                    fields[4]
                        .parse()
                        .map_err(|_| {
                            OrderStoreError
                                ::CorruptedRecord
                        })?,

                status:
                    unescape(fields[5]),
            }
        );
    }

    Ok(result)
}

/// ============================================================
/// ENUM ENCODING
/// ============================================================

fn encode_side(
    side: VenueOrderSide,
) -> &'static str {
    match side {
        VenueOrderSide::Buy => "B",
        VenueOrderSide::Sell => "S",
    }
}

fn decode_side(
    value: &str,
) -> Result<
    VenueOrderSide,
    OrderStoreError,
> {
    match value {
        "B" => Ok(
            VenueOrderSide::Buy
        ),

        "S" => Ok(
            VenueOrderSide::Sell
        ),

        _ => Err(
            OrderStoreError
                ::CorruptedRecord
        ),
    }
}

fn encode_order_type(
    order_type: VenueOrderType,
) -> &'static str {
    match order_type {
        VenueOrderType::Market => "M",
        VenueOrderType::Limit => "L",
    }
}

fn decode_order_type(
    value: &str,
) -> Result<
    VenueOrderType,
    OrderStoreError,
> {
    match value {
        "M" => Ok(
            VenueOrderType::Market
        ),

        "L" => Ok(
            VenueOrderType::Limit
        ),

        _ => Err(
            OrderStoreError
                ::CorruptedRecord
        ),
    }
}

fn encode_venue(
    venue: VenueId,
) -> &'static str {
    match venue {
        VenueId::Aster => "ASTER",
        VenueId::Binance => "BINANCE",
        VenueId::EdgeX => "EDGEX",
        VenueId::Okx => "OKX",
    }
}

fn decode_venue(
    value: &str,
) -> Result<
    VenueId,
    OrderStoreError,
> {
    match value {
        "ASTER" => Ok(
            VenueId::Aster
        ),

        "BINANCE" => Ok(
            VenueId::Binance
        ),

        "EDGEX" => Ok(
            VenueId::EdgeX
        ),

        "OKX" => Ok(
            VenueId::Okx
        ),

        _ => Err(
            OrderStoreError
                ::CorruptedRecord
        ),
    }
}

fn encode_lifecycle(
    lifecycle: OrderLifecycle,
) -> &'static str {
    match lifecycle {
        OrderLifecycle::Received => "RECEIVED",
        OrderLifecycle::RiskChecked => "RISK",
        OrderLifecycle::Reserved => "RESERVED",
        OrderLifecycle::MarketData => "MARKET_DATA",
        OrderLifecycle::Routed => "ROUTED",
        OrderLifecycle::Split => "SPLIT",
        OrderLifecycle::Executing => "EXECUTING",
        OrderLifecycle::Aggregated => "AGGREGATED",
        OrderLifecycle::Settled => "SETTLED",
        OrderLifecycle::PartiallySettled => {
            "PARTIAL_SETTLED"
        }
        OrderLifecycle::Failed => "FAILED",
    }
}

fn decode_lifecycle(
    value: &str,
) -> Result<
    OrderLifecycle,
    OrderStoreError,
> {
    match value {
        "RECEIVED" => Ok(
            OrderLifecycle::Received
        ),

        "RISK" => Ok(
            OrderLifecycle::RiskChecked
        ),

        "RESERVED" => Ok(
            OrderLifecycle::Reserved
        ),

        "MARKET_DATA" => Ok(
            OrderLifecycle::MarketData
        ),

        "ROUTED" => Ok(
            OrderLifecycle::Routed
        ),

        "SPLIT" => Ok(
            OrderLifecycle::Split
        ),

        "EXECUTING" => Ok(
            OrderLifecycle::Executing
        ),

        "AGGREGATED" => Ok(
            OrderLifecycle::Aggregated
        ),

        "SETTLED" => Ok(
            OrderLifecycle::Settled
        ),

        "PARTIAL_SETTLED" => Ok(
            OrderLifecycle
                ::PartiallySettled
        ),

        "FAILED" => Ok(
            OrderLifecycle::Failed
        ),

        _ => Err(
            OrderStoreError
                ::CorruptedRecord
        ),
    }
}

fn parent_status_from_lifecycle(
    lifecycle: OrderLifecycle,
) -> ParentOrderStatus {
    match lifecycle {
        OrderLifecycle::Settled => {
            ParentOrderStatus::Filled
        }

        OrderLifecycle::PartiallySettled => {
            ParentOrderStatus
                ::PartiallyFilled
        }

        OrderLifecycle::Failed => {
            ParentOrderStatus::Failed
        }

        _ => ParentOrderStatus::New,
    }
}

/// ============================================================
/// ESCAPING
/// ============================================================

fn escape(
    value: &str,
) -> String {
    value
        .replace('\\', "\\\\")
        .replace('|', "\\p")
        .replace('~', "\\t")
        .replace(';', "\\s")
        .replace('\n', "\\n")
}

fn unescape(
    value: &str,
) -> String {
    let mut result =
        String::new();

    let mut chars =
        value.chars();

    while let Some(ch) =
        chars.next()
    {
        if ch != '\\' {
            result.push(ch);
            continue;
        }

        match chars.next() {
            Some('p') => {
                result.push('|')
            }

            Some('t') => {
                result.push('~')
            }

            Some('s') => {
                result.push(';')
            }

            Some('n') => {
                result.push('\n')
            }

            Some('\\') => {
                result.push('\\')
            }

            Some(other) => {
                result.push('\\');
                result.push(other);
            }

            None => {
                result.push('\\')
            }
        }
    }

    result
}

fn parse_optional_u64(
    value: &str,
) -> Result<
    Option<u64>,
    OrderStoreError,
> {
    if value.is_empty() {
        return Ok(None);
    }

    value
        .parse::<u64>()
        .map(Some)
        .map_err(|_| {
            OrderStoreError
                ::CorruptedRecord
        })
}

fn optional_string(
    value: &str,
) -> Option<String> {
    if value.is_empty() {
        None
    } else {
        Some(
            unescape(value)
        )
    }
}

/// ============================================================
/// ERRORS
/// ============================================================

#[derive(Debug)]
pub enum OrderStoreError {
    NotFound,

    Io(
        std::io::Error
    ),

    LockPoisoned,

    CorruptedRecord,
}

fn current_timestamp_ms() -> u64 {
    use std::time::{
        SystemTime,
        UNIX_EPOCH,
    };

    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// ============================================================
/// TESTS
/// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    use crate::PEPDEX::market::MarketId;
    use crate::PEPDEX::risk::OrderConstraints;

    fn order() -> ParentOrder {
        ParentOrder::new(
            "ORDER-1",
            "alice",
            MarketId::new(
                "PEP",
                "USDT",
            ),
            VenueOrderSide::Buy,
            VenueOrderType::Limit,
            100,
            Some(600),
        )
        .with_constraints(
            OrderConstraints::new()
                .with_max_spend(
                    60_000
                )
                .with_max_slippage_bps(
                    100
                )
        )
    }

    fn stored() -> StoredOrder {
        StoredOrder {
            order: order(),

            lifecycle:
                OrderLifecycle::Executing,

            reservation_id:
                Some(
                    "ORDER-1-RESERVE"
                        .to_string()
                ),

            child_orders:
                vec![
                    StoredChildOrder {
                        child_order_id:
                            OrderId::new(
                                "CHILD-1"
                            ),

                        venue:
                            VenueId::Aster,

                        client_order_id:
                            "PEPDEX-CHILD-1"
                                .to_string(),

                        quantity:
                            100,

                        price:
                            Some(600),

                        status:
                            "Open".to_string(),
                    }
                ],

            external_orders:
                vec![
                    StoredExternalOrder {
                        child_order_id:
                            OrderId::new(
                                "CHILD-1"
                            ),

                        venue:
                            VenueId::Aster,

                        external_order_id:
                            "ASTER-123"
                                .to_string(),

                        filled_quantity:
                            40,

                        remaining_quantity:
                            60,

                        status:
                            "PartiallyFilled"
                                .to_string(),
                    }
                ],

            last_error:
                None,

            created_at_ms:
                100,

            updated_at_ms:
                200,
        }
    }

    #[test]
    fn memory_store_round_trip() {
        let store =
            OrderStore::memory();

        store
            .insert(stored())
            .unwrap();

        let loaded =
            store
                .get(
                    &OrderId::new(
                        "ORDER-1"
                    )
                )
                .unwrap()
                .unwrap();

        assert_eq!(
            loaded.order.id.as_str(),
            "ORDER-1"
        );

        assert_eq!(
            loaded.order.user,
            "alice"
        );

        assert_eq!(
            loaded.order.quantity,
            100
        );

        assert_eq!(
            loaded
                .external_orders
                .len(),
            1
        );

        assert_eq!(
            loaded
                .external_orders[0]
                .external_order_id,
            "ASTER-123"
        );
    }

    #[test]
    fn file_store_survives_reopen() {
        let path =
            std::env::temp_dir()
                .join(
                    format!(
                        "pep_dex_order_store_{}.db",
                        current_timestamp_ms()
                    )
                );

        {
            let store =
                OrderStore::file(
                    &path
                )
                .unwrap();

            store
                .insert(stored())
                .unwrap();
        }

        /*
         * Simulate process restart.
         */
        {
            let store =
                OrderStore::file(
                    &path
                )
                .unwrap();

            let loaded =
                store
                    .get(
                        &OrderId::new(
                            "ORDER-1"
                        )
                    )
                    .unwrap()
                    .unwrap();

            assert_eq!(
                loaded.order.id.as_str(),
                "ORDER-1"
            );

            assert_eq!(
                loaded
                    .external_orders[0]
                    .external_order_id,
                "ASTER-123"
            );

            assert_eq!(
                loaded
                    .external_orders[0]
                    .filled_quantity,
                40
            );
        }

        let _ =
            fs::remove_file(
                path
            );
    }

    #[test]
    fn file_store_updates_order() {
        let path =
            std::env::temp_dir()
                .join(
                    format!(
                        "pep_dex_order_update_{}.db",
                        current_timestamp_ms()
                    )
                );

        let store =
            OrderStore::file(
                &path
            )
            .unwrap();

        let mut order =
            stored();

        store
            .insert(
                order.clone()
            )
            .unwrap();

        order.lifecycle =
            OrderLifecycle::Settled;

        order.updated_at_ms =
            999;

        store
            .update(order)
            .unwrap();

        let loaded =
            store
                .get(
                    &OrderId::new(
                        "ORDER-1"
                    )
                )
                .unwrap()
                .unwrap();

        assert_eq!(
            loaded.lifecycle,
            OrderLifecycle::Settled
        );

        assert_eq!(
            loaded.updated_at_ms,
            999
        );

        let _ =
            fs::remove_file(
                path
            );
    }

    #[test]
    fn file_store_delete_survives_reopen() {
        let path =
            std::env::temp_dir()
                .join(
                    format!(
                        "pep_dex_order_delete_{}.db",
                        current_timestamp_ms()
                    )
                );

        {
            let store =
                OrderStore::file(
                    &path
                )
                .unwrap();

            store
                .insert(stored())
                .unwrap();

            store
                .remove(
                    &OrderId::new(
                        "ORDER-1"
                    )
                )
                .unwrap();
        }

        {
            let store =
                OrderStore::file(
                    &path
                )
                .unwrap();

            assert!(
                store
                    .get(
                        &OrderId::new(
                            "ORDER-1"
                        )
                    )
                    .unwrap()
                    .is_none()
            );
        }

        let _ =
            fs::remove_file(
                path
            );
    }

    #[test]
    fn pending_reconciliation_is_found() {
        let store =
            OrderStore::memory();

        store
            .insert(stored())
            .unwrap();

        let pending =
            store
                .pending_reconciliation()
                .unwrap();

        assert_eq!(
            pending.len(),
            1
        );

        assert!(
            pending[0]
                .is_reconcilable()
        );
    }

    #[test]
    fn escaping_round_trip() {
        let value =
            "abc|def~ghi;jkl\\xyz\n123";

        let encoded =
            escape(value);

        let decoded =
            unescape(&encoded);

        assert_eq!(
            decoded,
            value
        );
    }
}