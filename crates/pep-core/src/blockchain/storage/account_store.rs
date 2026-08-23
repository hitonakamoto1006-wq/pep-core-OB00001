use std::{
    collections::HashMap,
    fs::{self, File},
    io::{Read, Write},
    path::Path,
};

use crate::blockchain::{
    asset::AssetId,
    state::Account,
};

use crate::wallet::Address;

pub struct AccountStore;

impl AccountStore {

    const DIR: &'static str =
        "data/accounts";


    // ========================================================
    // PUT
    // ========================================================

    pub fn put(
        account: &Account,
    ) {

        fs::create_dir_all(
            Self::DIR
        )
        .unwrap();


        let path = format!(
            "{}/{}.acc",
            Self::DIR,
            account.address,
        );


        let mut file =
            File::create(path)
                .unwrap();


        /*
         * Header
         */
        writeln!(
            file,
            "{}",
            account.address,
        )
        .unwrap();


        /*
         * Nonce
         */
        writeln!(
            file,
            "NONCE|{}",
            account.nonce,
        )
        .unwrap();


        /*
         * Stake
         */
        writeln!(
            file,
            "STAKE|{}",
            account.stake,
        )
        .unwrap();


        /*
         * Multi-asset balances
         *
         * BALANCE|asset|amount
         */
        for (
            asset,
            amount,
        ) in &account.balances
        {

            writeln!(
                file,
                "BALANCE|{}|{}",
                asset,
                amount,
            )
            .unwrap();
        }
    }


    // ========================================================
    // GET
    // ========================================================

    pub fn get(
        address: &Address,
    ) -> Option<Account>
    {

        let path = format!(
            "{}/{}.acc",
            Self::DIR,
            address,
        );


        if !Path::new(&path).exists() {
            return None;
        }


        let mut text =
            String::new();


        File::open(path)
            .ok()?
            .read_to_string(
                &mut text
            )
            .ok()?;


        let lines:
            Vec<&str> =
            text.lines().collect();


        if lines.is_empty() {
            return None;
        }


        // ====================================================
        // LEGACY FORMAT
        // ====================================================
        //
        // address
        // balance
        // nonce
        // stake
        //
        // Legacy balance -> PEP
        //

        if lines.len() >= 4
            && !lines[1].contains('|')
        {

            let account_address =
                Address::new(
                    lines[0].to_string()
                );


            let balance =
                lines[1]
                    .parse::<u64>()
                    .ok()?;


            let nonce =
                lines[2]
                    .parse::<u64>()
                    .ok()?;


            let stake =
                lines[3]
                    .parse::<u64>()
                    .ok()?;


            let mut balances =
                HashMap::new();


            if balance > 0 {

                balances.insert(
                    AssetId::new(
                        AssetId::PEP
                    ),
                    balance,
                );
            }


            return Some(
                Account {

                    address:
                        account_address,

                    balances,

                    nonce,

                    stake,
                }
            );
        }


        // ====================================================
        // NEW MULTI-ASSET FORMAT
        // ====================================================

        let account_address =
            Address::new(
                lines[0].to_string()
            );


        let nonce =
            lines
                .get(1)?
                .strip_prefix("NONCE|")?
                .parse::<u64>()
                .ok()?;


        let stake =
            lines
                .get(2)?
                .strip_prefix("STAKE|")?
                .parse::<u64>()
                .ok()?;


        let mut balances =
            HashMap::new();


        for line in
            lines.iter().skip(3)
        {

            let parts:
                Vec<&str> =
                line.split('|')
                    .collect();


            if parts.len() != 3 {
                continue;
            }


            if parts[0] != "BALANCE" {
                continue;
            }


            let asset =
                AssetId::new(
                    parts[1]
                );


            let amount =
                parts[2]
                    .parse::<u64>()
                    .unwrap_or(0);


            balances.insert(
                asset,
                amount,
            );
        }


        Some(
            Account {

                address:
                    account_address,

                balances,

                nonce,

                stake,
            }
        )
    }


    // ========================================================
    // CONTAINS
    // ========================================================

    pub fn contains(
        address: &Address,
    ) -> bool {

        let path = format!(
            "{}/{}.acc",
            Self::DIR,
            address,
        );

        Path::new(&path).exists()
    }


    // ========================================================
    // DELETE
    // ========================================================

    pub fn delete(
        address: &Address,
    ) {

        let path = format!(
            "{}/{}.acc",
            Self::DIR,
            address,
        );


        if Path::new(&path).exists() {

            let _ =
                fs::remove_file(path);
        }
    }


    // ========================================================
    // COUNT
    // ========================================================

    pub fn count() -> usize {

        fs::read_dir(Self::DIR)
            .map(|d| d.count())
            .unwrap_or(0)
    }


    // ========================================================
    // LOAD ALL
    // ========================================================

    pub fn load_all() -> Vec<Account> {

        let mut accounts =
            Vec::new();


        let Ok(entries) =
            fs::read_dir(Self::DIR)
        else {
            return accounts;
        };


        for entry in entries {

            let Ok(entry) = entry
            else {
                continue;
            };


            let path =
                entry.path();


            /*
             * Chỉ đọc file .acc.
             */
            if path
                .extension()
                .and_then(|x| x.to_str())
                != Some("acc")
            {
                continue;
            }


            let mut text =
                String::new();


            if File::open(&path)
                .and_then(|mut f| {
                    f.read_to_string(
                        &mut text
                    )
                })
                .is_err()
            {
                continue;
            }


            let lines:
                Vec<&str> =
                text.lines().collect();


            if lines.is_empty() {
                continue;
            }


            // =================================================
            // LEGACY
            // =================================================

            if lines.len() >= 4
                && !lines[1].contains('|')
            {

                let Some(address) =
                    lines.get(0)
                else {
                    continue;
                };


                let Some(balance) =
                    lines.get(1)
                else {
                    continue;
                };


                let Some(nonce) =
                    lines.get(2)
                else {
                    continue;
                };


                let Some(stake) =
                    lines.get(3)
                else {
                    continue;
                };


                let balance =
                    balance
                        .parse::<u64>()
                        .unwrap_or(0);


                let nonce =
                    nonce
                        .parse::<u64>()
                        .unwrap_or(0);


                let stake =
                    stake
                        .parse::<u64>()
                        .unwrap_or(0);


                let mut balances =
                    HashMap::new();


                if balance > 0 {

                    balances.insert(
                        AssetId::new(
                            AssetId::PEP
                        ),
                        balance,
                    );
                }


                accounts.push(
                    Account {

                        address:
                            Address::new(
                                address.to_string()
                            ),

                        balances,

                        nonce,

                        stake,
                    }
                );


                continue;
            }


            // =================================================
            // MULTI-ASSET
            // =================================================

            let Some(address) =
                lines.get(0)
            else {
                continue;
            };


            let Some(nonce_line) =
                lines.get(1)
            else {
                continue;
            };


            let Some(stake_line) =
                lines.get(2)
            else {
                continue;
            };


            let Some(nonce) =
                nonce_line
                    .strip_prefix("NONCE|")
            else {
                continue;
            };


            let Some(stake) =
                stake_line
                    .strip_prefix("STAKE|")
            else {
                continue;
            };


            let nonce =
                nonce
                    .parse::<u64>()
                    .unwrap_or(0);


            let stake =
                stake
                    .parse::<u64>()
                    .unwrap_or(0);


            let mut balances =
                HashMap::new();


            for line in
                lines.iter().skip(3)
            {

                let parts:
                    Vec<&str> =
                    line.split('|')
                        .collect();


                if parts.len() != 3 {
                    continue;
                }


                if parts[0] != "BALANCE" {
                    continue;
                }


                let asset =
                    AssetId::new(
                        parts[1]
                    );


                let amount =
                    parts[2]
                        .parse::<u64>()
                        .unwrap_or(0);


                balances.insert(
                    asset,
                    amount,
                );
            }


            accounts.push(
                Account {

                    address:
                        Address::new(
                            address.to_string()
                        ),

                    balances,

                    nonce,

                    stake,
                }
            );
        }


        accounts
    }


    // ========================================================
    // CLEAR
    // ========================================================

    pub fn clear() {

        if Path::new(Self::DIR).exists() {

            let _ =
                fs::remove_dir_all(
                    Self::DIR,
                );
        }
    }
}