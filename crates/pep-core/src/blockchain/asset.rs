use std::{
    collections::HashMap,
    fmt,
    sync::{
        OnceLock,
        RwLock,
    },
};


// ============================================================
// ASSET ID
// ============================================================

#[derive(
    Clone,
    Debug,
    PartialEq,
    Eq,
    Hash,
)]
pub struct AssetId(
    String
);

impl AssetId {

    pub const PEP: &'static str = "PEP";
    pub const BTCP: &'static str = "BTCP";
    pub const ETHP: &'static str = "ETHP";
    pub const USDP: &'static str = "USDP";

    pub fn new(
        symbol: impl Into<String>,
    ) -> Self {

        Self(
            symbol
                .into()
                .to_uppercase()
        )
    }

    pub fn as_str(
        &self,
    ) -> &str {

        &self.0
    }
}

impl fmt::Display for AssetId {

    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {

        write!(
            f,
            "{}",
            self.0
        )
    }
}


// ============================================================
// ASSET TYPE
// ============================================================

#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
)]
pub enum AssetType {

    /*
     * PEP:
     *
     * Index asset của PEP ecosystem.
     */
    Index,

    /*
     * BTCP / ETHP / USDP:
     *
     * Native representation của
     * external-chain assets.
     */
    Pegged,

    /*
     * CCQ của các fund.
     */
    Ccq,

    /*
     * Asset native khác nếu sau này
     * protocol cần.
     */
    Native,
}


// ============================================================
// ASSET DEFINITION
// ============================================================

#[derive(
    Clone,
    Debug,
)]
pub struct AssetDefinition {

    pub id: AssetId,

    pub asset_type: AssetType,

    pub decimals: u8,

    /*
     * Tổng supply của asset.
     *
     * Với các asset core cũ,
     * giá trị này được đặt theo
     * protocol default.
     */
    pub supply: u64,

    /*
     * Địa chỉ deploy / contract / external
     * reference của asset nếu có.
     *
     * Không phải asset nào cũng có.
     */
    pub deploy_address:
        Option<String>,

    /*
     * Có thể transfer giữa accounts hay không.
     */
    pub transferable: bool,

    /*
     * Có thể dùng làm gas hay không.
     */
    pub gas_eligible: bool,

    /*
     * Nếu là pegged asset thì đây là
     * external asset tương ứng.
     *
     * Ví dụ:
     *
     * BTCP -> BTC
     * ETHP -> ETH
     * USDP -> USDT
     */
    pub peg:
        Option<&'static str>,
}


// ============================================================
// CUSTOM REGISTRY
// ============================================================

/*
 * Custom assets được đăng ký runtime.
 *
 * Sau bước này registry đã có khả năng
 * nhận asset mới.
 *
 * Persistence vào State sẽ được nối ở
 * bước tiếp theo.
 */
static CUSTOM_ASSETS:
    OnceLock<
        RwLock<
            HashMap<
                AssetId,
                AssetDefinition,
            >
        >
    > =
    OnceLock::new();


fn custom_assets()
    -> &'static RwLock<
        HashMap<
            AssetId,
            AssetDefinition,
        >
    >
{
    CUSTOM_ASSETS
        .get_or_init(|| {
            RwLock::new(
                HashMap::new()
            )
        })
}


// ============================================================
// REGISTRY
// ============================================================

pub struct AssetRegistry;

impl AssetRegistry {

    // ========================================================
    // GET
    // ========================================================

    pub fn get(
        asset: &AssetId,
    ) -> Option<AssetDefinition> {

        match asset.as_str() {

            // =================================================
            // PEP
            // =================================================

            AssetId::PEP => {

                Some(
                    AssetDefinition {

                        id:
                            AssetId::new(
                                AssetId::PEP
                            ),

                        asset_type:
                            AssetType::Index,

                        decimals:
                            8,

                        supply:
                            0,

                        deploy_address:
                            None,

                        transferable:
                            true,

                        gas_eligible:
                            true,

                        peg:
                            None,
                    }
                )
            }


            // =================================================
            // BTCP
            // =================================================

            AssetId::BTCP => {

                Some(
                    AssetDefinition {

                        id:
                            AssetId::new(
                                AssetId::BTCP
                            ),

                        asset_type:
                            AssetType::Pegged,

                        decimals:
                            8,

                        supply:
                            0,

                        deploy_address:
                            None,

                        transferable:
                            true,

                        gas_eligible:
                            false,

                        peg:
                            Some("BTC"),
                    }
                )
            }


            // =================================================
            // ETHP
            // =================================================

            AssetId::ETHP => {

                Some(
                    AssetDefinition {

                        id:
                            AssetId::new(
                                AssetId::ETHP
                            ),

                        asset_type:
                            AssetType::Pegged,

                        decimals:
                            18,

                        supply:
                            0,

                        deploy_address:
                            None,

                        transferable:
                            true,

                        gas_eligible:
                            false,

                        peg:
                            Some("ETH"),
                    }
                )
            }


            // =================================================
            // USDP
            // =================================================

            AssetId::USDP => {

                Some(
                    AssetDefinition {

                        id:
                            AssetId::new(
                                AssetId::USDP
                            ),

                        asset_type:
                            AssetType::Pegged,

                        decimals:
                            6,

                        supply:
                            0,

                        deploy_address:
                            None,

                        transferable:
                            true,

                        gas_eligible:
                            false,

                        peg:
                            Some("USDT"),
                    }
                )
            }


            // =================================================
            // CUSTOM / CCQ
            // =================================================

            _ => {

                /*
                 * CCQ:
                 *
                 * CCQ:<fund-id>
                 */
                if asset
                    .as_str()
                    .starts_with("CCQ:")
                {

                    return Some(
                        AssetDefinition {

                            id:
                                asset.clone(),

                            asset_type:
                                AssetType::Ccq,

                            decimals:
                                8,

                            supply:
                                0,

                            deploy_address:
                                None,

                            transferable:
                                true,

                            gas_eligible:
                                false,

                            peg:
                                None,
                        }
                    );
                }


                /*
                 * Custom asset.
                 */
                match custom_assets()
                    .read()
                {
                    Ok(registry) => {

                        registry
                            .get(asset)
                            .cloned()
                    }

                    Err(_) => None,
                }
            }
        }
    }


    // ========================================================
    // REGISTER CUSTOM ASSET
    // ========================================================

    pub fn register(
        definition: AssetDefinition,
    ) -> Result<(), String> {

        /*
         * Không cho ghi đè asset core.
         */
        match definition.id.as_str() {

            AssetId::PEP |
            AssetId::BTCP |
            AssetId::ETHP |
            AssetId::USDP => {

                return Err(
                    format!(
                        "Cannot overwrite core asset: {}",
                        definition.id
                    )
                );
            }

            _ => {}
        }


        /*
         * Asset name không được rỗng.
         */
        if definition
            .id
            .as_str()
            .trim()
            .is_empty()
        {

            return Err(
                "Asset ID cannot be empty"
                    .to_string()
            );
        }


        /*
         * Supply phải hợp lệ.
         *
         * 0 vẫn được phép ở bước registry,
         * vì asset có thể được tạo trước
         * rồi mint theo protocol.
         */
        if definition
            .decimals > 38
        {

            return Err(
                "Asset decimals cannot exceed 38"
                    .to_string()
            );
        }


        let mut registry =
            custom_assets()
                .write()
                .map_err(
                    |_| {
                        "Asset registry lock poisoned"
                            .to_string()
                    }
                )?;


        if registry
            .contains_key(
                &definition.id
            )
        {

            return Err(
                format!(
                    "Asset already exists: {}",
                    definition.id
                )
            );
        }


        registry.insert(
            definition.id.clone(),
            definition,
        );


        Ok(())
    }


    // ========================================================
    // CREATE NATIVE ASSET
    // ========================================================

    pub fn create_native(
        id: AssetId,
        supply: u64,
        deploy_address:
            Option<String>,
    ) -> AssetDefinition {

        AssetDefinition {

            id,

            asset_type:
                AssetType::Native,

            decimals:
                8,

            supply,

            deploy_address,

            transferable:
                true,

            gas_eligible:
                false,

            peg:
                None,
        }
    }


    // ========================================================
    // EXISTS
    // ========================================================

    pub fn exists(
        asset: &AssetId,
    ) -> bool {

        Self::get(
            asset
        )
        .is_some()
    }


    // ========================================================
    // GAS ASSET
    // ========================================================

    pub fn gas_asset()
        -> AssetId {

        AssetId::new(
            AssetId::PEP
        )
    }
}