use std::collections::HashMap;
use std::sync::{OnceLock, RwLock};

use crate::PEPDEX::balance::BalanceLedger;


// ============================================================
// DECIMALS
// ============================================================

const USDP_DECIMALS: u32 = 6;
const CCQ_DECIMALS: u32 = 8;


// ============================================================
// SIMULATED ASSET
// ============================================================

#[derive(Clone, Debug)]
pub struct SimAsset {

    pub symbol: String,

    pub decimals: u8,

    pub transferable: bool,

    pub kind: String,

    pub peg: Option<String>,

    pub total_supply: i128,
}


// ============================================================
// SIMULATED FUND
// ============================================================

#[derive(Clone, Debug)]
pub struct SimFund {

    pub id: String,

    pub name: String,

    pub symbol: String,

    pub underlying: String,

    pub ccq_asset: String,

    pub nav: i128,

    pub reserve: i128,

    pub ccq_supply: i128,
}


// ============================================================
// SIMULATED POL PROOF
// ============================================================

#[derive(Clone, Debug)]
pub struct PolProof {

    pub id: u64,

    pub fund_id: String,

    pub wallet: String,

    pub ccq_burned: i128,

    pub underlying_redeemed: i128,

    pub reserve_after: i128,
}


// ============================================================
// SIMULATION STATE
// ============================================================

pub struct Simulation {

    pub assets:
        HashMap<String, SimAsset>,

    pub funds:
        HashMap<String, SimFund>,

    pub pol:
        Vec<PolProof>,

    next_fund: u64,

    next_pol: u64,
}


// ============================================================
// CONSTRUCTOR
// ============================================================

impl Simulation {

    pub fn new() -> Self {

        let mut assets =
            HashMap::new();


        // ----------------------------------------------------
        // USDP
        // ----------------------------------------------------

        assets.insert(
            "USDP".to_string(),

            SimAsset {

                symbol:
                    "USDP".to_string(),

                decimals:
                    USDP_DECIMALS as u8,

                transferable:
                    true,

                kind:
                    "Pegged".to_string(),

                peg:
                    Some(
                        "USDT".to_string()
                    ),

                total_supply:
                    0,
            },
        );


        Self {

            assets,

            funds:
                HashMap::new(),

            pol:
                Vec::new(),

            next_fund:
                1,

            next_pol:
                1,
        }
    }


    // ========================================================
    // CREATE FUND
    // ========================================================

    pub fn create_fund(
        &mut self,
        name: String,
        symbol: String,
    ) -> Result<SimFund, String> {

        if name.trim().is_empty()
            ||
           symbol.trim().is_empty()
        {

            return Err(
                "fund name and symbol are required"
                    .to_string()
            );
        }


        let id =
            format!(
                "FUND-{:04}",
                self.next_fund
            );


        self.next_fund += 1;


        let symbol =
            symbol
                .trim()
                .to_uppercase();


        let ccq_asset =
            format!(
                "CCQ:{}",
                id
            );


        // ----------------------------------------------------
        // CREATE CCQ ASSET
        // ----------------------------------------------------

        self.assets.insert(

            ccq_asset.clone(),

            SimAsset {

                symbol:
                    ccq_asset.clone(),

                decimals:
                    CCQ_DECIMALS as u8,

                transferable:
                    true,

                kind:
                    "CCQ".to_string(),

                peg:
                    None,

                total_supply:
                    0,
            },
        );


        // ----------------------------------------------------
        // CREATE FUND
        // ----------------------------------------------------

        let fund =
            SimFund {

                id:
                    id.clone(),

                name:
                    name.trim().to_string(),

                symbol,

                underlying:
                    "USDP".to_string(),

                ccq_asset,

                nav:
                    0,

                reserve:
                    0,

                ccq_supply:
                    0,
            };


        self.funds.insert(
            id,
            fund.clone(),
        );


        Ok(fund)
    }


    // ========================================================
    // CREATE CUSTOM ASSET
    // ========================================================

    pub fn create_asset(
        &mut self,

        symbol: String,

        decimals: u8,

        transferable: bool,

        kind: String,

        peg: Option<String>,

    ) -> Result<SimAsset, String> {

        let symbol =
            symbol
                .trim()
                .to_uppercase();


        if symbol.is_empty() {

            return Err(
                "asset symbol cannot be empty"
                    .to_string()
            );
        }


        if self.assets.contains_key(
            &symbol
        ) {

            return Err(
                format!(
                    "asset already exists: {}",
                    symbol
                )
            );
        }


        let asset =
            SimAsset {

                symbol:
                    symbol.clone(),

                decimals,

                transferable,

                kind,

                peg,

                total_supply:
                    0,
            };


        self.assets.insert(
            symbol,
            asset.clone(),
        );


        Ok(asset)
    }


    // ========================================================
    // GET ASSET
    // ========================================================

    pub fn asset(
        &self,
        symbol: &str,
    ) -> Option<SimAsset> {

        self.assets
            .get(
                &symbol
                    .trim()
                    .to_uppercase()
            )
            .cloned()
    }


    // ========================================================
    // MINT CUSTOM ASSET
    // ========================================================

    pub fn mint_asset(
        &mut self,

        ledger:
            &BalanceLedger,

        wallet:
            &str,

        symbol:
            &str,

        amount:
            i128,

    ) -> Result<SimAsset, String> {

        if amount <= 0 {

            return Err(
                "mint amount must be greater than zero"
                    .to_string()
            );
        }


        let symbol =
            symbol
                .trim()
                .to_uppercase();


        let asset =
            self.assets
                .get_mut(&symbol)
                .ok_or_else(|| {

                    format!(
                        "asset not found: {}",
                        symbol
                    )
                })?;


        asset.total_supply =
            asset
                .total_supply
                .checked_add(amount)
                .ok_or_else(|| {

                    "asset supply overflow"
                        .to_string()

                })?;


        ledger
            .deposit(
                wallet,
                &symbol,
                amount,
            )
            .map_err(|e| {

                format!(
                    "asset credit failed: {:?}",
                    e
                )

            })?;


        Ok(
            asset.clone()
        )
    }


    // ========================================================
    // ISSUE CCQ
    // ========================================================

    pub fn issue_ccq(

        &mut self,

        ledger:
            &BalanceLedger,

        wallet:
            &str,

        fund_id:
            &str,

        usdp_amount:
            i128,

    ) -> Result<SimFund, String> {

        if usdp_amount <= 0 {

            return Err(
                "USDP amount must be greater than zero"
                    .to_string()
            );
        }


        let fund =
            self.funds
                .get(fund_id)
                .cloned()
                .ok_or_else(|| {

                    format!(
                        "fund not found: {}",
                        fund_id
                    )

                })?;


        /*
         * USDP:
         *
         * 6 decimals
         *
         * CCQ:
         *
         * 8 decimals
         *
         *
         * 1 USDP = 1 CCQ
         *
         * therefore:
         *
         * 1_000_000 USDP units
         * =
         * 100_000_000 CCQ units
         */

        let ccq_amount =
            usdp_amount
                .checked_mul(
                    10_i128.pow(
                        CCQ_DECIMALS
                            - USDP_DECIMALS
                    )
                )
                .ok_or_else(|| {

                    "CCQ amount overflow"
                        .to_string()

                })?;


        // ----------------------------------------------------
        // TAKE USDP
        // ----------------------------------------------------

        ledger
            .withdraw(
                wallet,
                "USDP",
                usdp_amount,
            )
            .map_err(|e| {

                format!(
                    "USDP debit failed: {:?}",
                    e
                )

            })?;


        // ----------------------------------------------------
        // UPDATE FUND
        // ----------------------------------------------------

        let fund =
            self.funds
                .get_mut(fund_id)
                .ok_or_else(|| {

                    "fund disappeared"
                        .to_string()

                })?;


        fund.reserve =
            fund
                .reserve
                .checked_add(
                    usdp_amount
                )
                .ok_or_else(|| {

                    "fund reserve overflow"
                        .to_string()

                })?;


        fund.nav =
            fund
                .nav
                .checked_add(
                    usdp_amount
                )
                .ok_or_else(|| {

                    "fund NAV overflow"
                        .to_string()

                })?;


        fund.ccq_supply =
            fund
                .ccq_supply
                .checked_add(
                    ccq_amount
                )
                .ok_or_else(|| {

                    "CCQ supply overflow"
                        .to_string()

                })?;


        let ccq_asset =
            fund.ccq_asset.clone();


        // ----------------------------------------------------
        // UPDATE CCQ SUPPLY
        // ----------------------------------------------------

        let asset =
            self.assets
                .get_mut(&ccq_asset)
                .ok_or_else(|| {

                    "CCQ asset missing"
                        .to_string()

                })?;


        asset.total_supply =
            asset
                .total_supply
                .checked_add(
                    ccq_amount
                )
                .ok_or_else(|| {

                    "CCQ total supply overflow"
                        .to_string()

                })?;


        // ----------------------------------------------------
        // CREDIT CCQ
        // ----------------------------------------------------

        ledger
            .deposit(
                wallet,
                &ccq_asset,
                ccq_amount,
            )
            .map_err(|e| {

                format!(
                    "CCQ credit failed: {:?}",
                    e
                )

            })?;


        Ok(
            self.funds
                .get(fund_id)
                .cloned()
                .unwrap()
        )
    }


    // ========================================================
    // REDEEM
    // ========================================================

    pub fn redeem(

        &mut self,

        ledger:
            &BalanceLedger,

        wallet:
            &str,

        fund_id:
            &str,

        ccq_amount:
            i128,

    ) -> Result<PolProof, String> {

        if ccq_amount <= 0 {

            return Err(
                "CCQ amount must be greater than zero"
                    .to_string()
            );
        }


        let snapshot =
            self.funds
                .get(fund_id)
                .cloned()
                .ok_or_else(|| {

                    format!(
                        "fund not found: {}",
                        fund_id
                    )

                })?;


        let factor =
            10_i128.pow(
                CCQ_DECIMALS
                    - USDP_DECIMALS
            );


        if ccq_amount % factor != 0 {

            return Err(
                "CCQ amount is not exactly redeemable"
                    .to_string()
            );
        }


        let usdp_amount =
            ccq_amount / factor;


        // ----------------------------------------------------
        // CHECK POOL
        // ----------------------------------------------------

        if snapshot.reserve
            < usdp_amount
        {

            return Err(
                "fund reserve is insufficient"
                    .to_string()
            );
        }


        // ----------------------------------------------------
        // BURN CCQ
        // ----------------------------------------------------

        ledger
            .withdraw(
                wallet,
                &snapshot.ccq_asset,
                ccq_amount,
            )
            .map_err(|e| {

                format!(
                    "CCQ burn failed: {:?}",
                    e
                )

            })?;


        // ----------------------------------------------------
        // UPDATE FUND
        // ----------------------------------------------------

        let fund =
            self.funds
                .get_mut(fund_id)
                .unwrap();


        fund.reserve -=
            usdp_amount;


        fund.nav -=
            usdp_amount;


        fund.ccq_supply -=
            ccq_amount;


        // ----------------------------------------------------
        // UPDATE CCQ SUPPLY
        // ----------------------------------------------------

        let asset =
            self.assets
                .get_mut(
                    &fund.ccq_asset
                )
                .unwrap();


        asset.total_supply -=
            ccq_amount;


        // ----------------------------------------------------
        // RETURN UNDERLYING
        // ----------------------------------------------------

        ledger
            .deposit(
                wallet,
                "USDP",
                usdp_amount,
            )
            .map_err(|e| {

                format!(
                    "USDP redemption credit failed: {:?}",
                    e
                )

            })?;


        // ----------------------------------------------------
        // CREATE PoL PROOF
        // ----------------------------------------------------

        let proof =
            PolProof {

                id:
                    self.next_pol,

                fund_id:
                    fund_id.to_string(),

                wallet:
                    wallet.to_string(),

                ccq_burned:
                    ccq_amount,

                underlying_redeemed:
                    usdp_amount,

                reserve_after:
                    fund.reserve,
            };


        self.next_pol += 1;


        self.pol.push(
            proof.clone()
        );


        Ok(proof)
    }


    // ========================================================
    // TRANSFER
    // ========================================================

    pub fn transfer(

        &self,

        ledger:
            &BalanceLedger,

        from:
            &str,

        to:
            &str,

        asset:
            &str,

        amount:
            i128,

    ) -> Result<(), String> {

        if amount <= 0 {

            return Err(
                "transfer amount must be greater than zero"
                    .to_string()
            );
        }


        let asset =
            asset
                .trim()
                .to_uppercase();


        let definition =
            self.assets
                .get(&asset)
                .ok_or_else(|| {

                    format!(
                        "asset not found: {}",
                        asset
                    )

                })?;


        if !definition.transferable {

            return Err(
                format!(
                    "asset is not transferable: {}",
                    asset
                )
            );
        }


        ledger
            .withdraw(
                from,
                &asset,
                amount,
            )
            .map_err(|e| {

                format!(
                    "sender debit failed: {:?}",
                    e
                )

            })?;


        ledger
            .deposit(
                to,
                &asset,
                amount,
            )
            .map_err(|e| {

                format!(
                    "receiver credit failed: {:?}",
                    e
                )

            })?;


        Ok(())
    }


    // ========================================================
    // GET FUND
    // ========================================================

    pub fn fund(
        &self,
        id: &str,
    ) -> Option<SimFund> {

        self.funds
            .get(id)
            .cloned()
    }
}


// ============================================================
// DEFAULT
// ============================================================

impl Default for Simulation {

    fn default() -> Self {
        Self::new()
    }
}


// ============================================================
// GLOBAL SIMULATION STATE
// ============================================================

static SIMULATION:
    OnceLock<RwLock<Simulation>> =
    OnceLock::new();


pub fn simulation()
    -> &'static RwLock<Simulation>
{
    SIMULATION.get_or_init(
        || {
            RwLock::new(
                Simulation::new()
            )
        }
    )
}