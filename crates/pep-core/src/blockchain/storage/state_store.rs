use std::{
    fs::{self, File},
    io::{Read, Write},
    path::Path,
};

use crate::blockchain::{
    asset::AssetId,
    state::{
        Account,
        State,
    },
};

use crate::wallet::Address;
pub struct StateStore;

impl StateStore {

    const PATH: &'static str =
        "data/state.dat";

    const SNAPSHOT_DIR: &'static str =
        "data/snapshots";


    // =========================
    // Current State
    // =========================

    pub fn save(
        state: &State,
    ) {

        fs::create_dir_all("data")
            .unwrap();

        let mut file =
            File::create(
                Self::PATH
            )
            .unwrap();

        Self::write_state(
            &mut file,
            state,
        );
    }


    pub fn load() -> Option<State> {

        Self::load_from_file(
            Self::PATH,
        )
    }


    // =========================
    // Snapshot
    // =========================

    pub fn save_snapshot(
        state: &State,
    ) {

        fs::create_dir_all(
            Self::SNAPSHOT_DIR,
        )
        .unwrap();

        let path = format!(
            "{}/{}.dat",
            Self::SNAPSHOT_DIR,
            state.height,
        );

        let mut file =
            File::create(path)
                .unwrap();

        Self::write_state(
            &mut file,
            state,
        );
    }


    pub fn load_snapshot(
        height: u64,
    ) -> Option<State> {

        let path = format!(
            "{}/{}.dat",
            Self::SNAPSHOT_DIR,
            height,
        );

        Self::load_from_file(
            &path,
        )
    }


    pub fn latest_snapshot()
        -> Option<State>
    {

        let mut latest =
            0u64;

        for entry in fs::read_dir(
            Self::SNAPSHOT_DIR,
        )
        .ok()?
        {

            let entry =
                entry.ok()?;

            let name =
                entry.file_name();

            let name =
                name.to_string_lossy();

            if let Some(height) =
                name.strip_suffix(".dat")
            {

                if let Ok(h) =
                    height.parse::<u64>()
                {

                    latest =
                        latest.max(h);
                }
            }
        }

        Self::load_snapshot(
            latest,
        )
    }


    pub fn rollback(
        height: u64,
    ) -> Option<State>
    {

        let state =
            Self::load_snapshot(
                height,
            )?;

        Self::save(
            &state,
        );

        Some(state)
    }


    pub fn prune(
        keep_after: u64,
    ) {

        let Ok(entries) =
            fs::read_dir(
                Self::SNAPSHOT_DIR,
            )
        else {
            return;
        };

        for entry in entries {

            let Ok(entry) = entry
            else {
                continue;
            };

            let name =
                entry.file_name();

            let name =
                name.to_string_lossy();

            let Some(height) =
                name.strip_suffix(".dat")
            else {
                continue;
            };

            let Ok(height) =
                height.parse::<u64>()
            else {
                continue;
            };

            if height < keep_after {

                let _ =
                    fs::remove_file(
                        entry.path(),
                    );
            }
        }
    }


    // =========================
    // Internal
    // =========================

    fn write_state(
        file: &mut File,
        state: &State,
    ) {

        /*
         * First line:
         *
         * blockchain height
         */
        writeln!(
            file,
            "{}",
            state.height,
        )
        .unwrap();


        for account
            in state.accounts.values()
        {

            /*
             * ACCOUNT
             *
             * A|address|nonce|stake
             */
            writeln!(
                file,
                "A|{}|{}|{}",
                account.address,
                account.nonce,
                account.stake,
            )
            .unwrap();


            /*
             * BALANCES
             *
             * B|address|asset|amount
             */
            for (
                asset,
                amount,
            ) in &account.balances
            {

                writeln!(
                    file,
                    "B|{}|{}|{}",
                    account.address,
                    asset,
                    amount,
                )
                .unwrap();
            }
        }
    }


    // =========================
    // Load
    // =========================

    fn load_from_file(
        path: &str,
    ) -> Option<State>
    {

        if !Path::new(path).exists() {
            return None;
        }


        let mut text =
            String::new();

        File::open(path)
            .ok()?
            .read_to_string(
                &mut text,
            )
            .ok()?;


        let mut lines =
            text.lines();


        let height =
            lines
                .next()?
                .parse::<u64>()
                .ok()?;


        let mut state =
            State::new();

        state.height =
            height;


        for line in lines {

            let parts:
                Vec<&str> =
                line.split('|')
                    .collect();


            // =========================================
            // NEW ACCOUNT FORMAT
            // =========================================
            //
            // A|address|nonce|stake
            //

            if parts.len() == 4
                && parts[0] == "A"
            {

                let address =
                    Address::new(
                        parts[1].to_string(),
                    );


                let nonce =
                    parts[2]
                        .parse::<u64>()
                        .unwrap_or(0);


                let stake =
                    parts[3]
                        .parse::<u64>()
                        .unwrap_or(0);


                state.accounts.insert(
                    address.clone(),
                    Account {

                        address,

                        balances:
                            std::collections
                                ::HashMap::new(),

                        nonce,

                        stake,
                    },
                );

                continue;
            }


            // =========================================
            // NEW BALANCE FORMAT
            // =========================================
            //
            // B|address|asset|amount
            //

            if parts.len() == 4
                && parts[0] == "B"
            {

                let address =
                    Address::new(
                        parts[1].to_string(),
                    );


                let asset =
                    AssetId::new(
                        parts[2],
                    );


                let amount =
                    parts[3]
                        .parse::<u64>()
                        .unwrap_or(0);


                /*
                 * Đề phòng balance record xuất hiện
                 * trước account record.
                 */
                state.create_account(
                    &address
                );


                if let Some(account) =
                    state.get_account_mut(
                        &address
                    )
                {

                    account
                        .balances
                        .insert(
                            asset,
                            amount,
                        );
                }

                continue;
            }


            // =========================================
            // LEGACY FORMAT
            // =========================================
            //
            // address|balance|nonce|stake
            //
            // Balance cũ được migrate thành PEP.
            //

            if parts.len() == 4 {

                let address =
                    Address::new(
                        parts[0].to_string(),
                    );


                let balance =
                    parts[1]
                        .parse::<u64>()
                        .unwrap_or(0);


                let nonce =
                    parts[2]
                        .parse::<u64>()
                        .unwrap_or(0);


                let stake =
                    parts[3]
                        .parse::<u64>()
                        .unwrap_or(0);


                let mut account =
                    Account::new(
                        address.clone()
                    );


                account.nonce =
                    nonce;

                account.stake =
                    stake;


                /*
                 * Legacy balance
                 * = PEP.
                 */
                if balance > 0 {

                    account
                        .balances
                        .insert(
                            AssetId::new(
                                AssetId::PEP
                            ),
                            balance,
                        );
                }


                state.accounts.insert(
                    address,
                    account,
                );
            }
        }


        Some(state)
    }
}