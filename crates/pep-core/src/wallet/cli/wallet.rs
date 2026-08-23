use std::io::{self, Write};

use crate::blockchain::{
    network::client::Client,
    transaction::TransactionType,
};

use crate::wallet::{
    Address,
    Wallet,
};

pub struct WalletCli;

impl WalletCli {
    pub fn start() {
        println!();
        println!("========== PEP Network ==========");
        println!("Searching for PEP Network...");

        let node_address = match Client::discover_node() {
            Some(address) => {
                println!("✓ PEP Node found: {}", address);
                address
            }

            None => {
                println!();
                println!("✗ No PEP Node found.");
                println!("Make sure a PEP Core node is running on the same network.");
                return;
            }
        };

        println!("✓ Connected to PEP Network");

        loop {
            println!();
            println!("========== PEP Wallet ==========");
            println!("Network : Connected");
            println!("Node    : {}", node_address);
            println!();
            println!("1. Create Wallet");
            println!("2. Import Wallet");
            println!("0. Exit");
            print!("Select: ");

            io::stdout().flush().unwrap();

            let mut input = String::new();

            io::stdin()
                .read_line(&mut input)
                .unwrap();

            match input.trim() {
                // =========================================================
                // CREATE WALLET
                // =========================================================

                "1" => {
                    let wallet = Wallet::new();

                    println!();
                    println!("========== NEW WALLET ==========");

                    println!(
                        "Mnemonic : {}",
                        wallet.mnemonic()
                    );

                    println!(
                        "Address  : {}",
                        wallet.address()
                    );

                    println!(
                        "Public   : {:02x?}",
                        wallet.public_key().bytes(),
                    );

                    println!("===============================");

                    Self::wallet_menu(
                        &wallet,
                        &node_address,
                    );
                }

                // =========================================================
                // IMPORT WALLET
                // =========================================================

                "2" => {
                    println!();
                    println!("========== IMPORT WALLET ==========");

                    println!("Enter 25-word mnemonic:");

                    print!("> ");

                    io::stdout()
                        .flush()
                        .unwrap();

                    let mut phrase = String::new();

                    io::stdin()
                        .read_line(&mut phrase)
                        .unwrap();

                    match Wallet::from_phrase(
                        phrase.trim(),
                    ) {
                        Ok(wallet) => {
                            println!();
                            println!("Wallet Imported!");

                            println!(
                                "Address : {}",
                                wallet.address()
                            );

                            Self::wallet_menu(
                                &wallet,
                                &node_address,
                            );
                        }

                        Err(err) => {
                            println!(
                                "Import failed: {}",
                                err
                            );
                        }
                    }
                }

                // =========================================================
                // EXIT
                // =========================================================

                "0" => {
                    break;
                }

                _ => {
                    println!("Invalid choice.");
                }
            }
        }
    }

    // =====================================================================
    // WALLET MENU
    // =====================================================================

    fn wallet_menu(
        wallet: &Wallet,
        node_address: &str,
    ) {
        loop {
            println!();
            println!("========== Wallet ==========");

            println!(
                "Address : {}",
                wallet.address()
            );

            println!(
                "Node    : {}",
                node_address
            );

            println!();
            println!("1. Balance");
            println!("2. Transfer");
            println!("3. Mint $PEP");
            println!("4. Burn $PEP");
            println!("5. Wallet Info");
            println!("6. Add Asset");
            println!("0. Exit");

            print!("Select: ");

            io::stdout()
                .flush()
                .unwrap();

            let mut input = String::new();

            io::stdin()
                .read_line(&mut input)
                .unwrap();

            match input.trim() {
                // =========================================================
                // BALANCE
                // =========================================================

                "1" => {
                    println!();
                    println!("===== Balance =====");

                    match Client::get_balance(
                        node_address,
                        wallet.address(),
                    ) {
                        Some((
                            balances,
                            nonce,
                            stake,
                        )) => {
                            println!();
                            println!("===== Portfolio =====");

                            if balances.is_empty() {
                                println!("No assets");
                            } else {
                                for (
                                    asset,
                                    amount,
                                ) in balances {
                                    println!(
                                        "{:<12} {}",
                                        asset,
                                        amount
                                    );
                                }
                            }

                            println!();

                            println!(
                                "Nonce   : {}",
                                nonce
                            );

                            println!(
                                "Stake   : {}",
                                stake
                            );
                        }

                        None => {
                            println!(
                                "Cannot connect to node."
                            );
                        }
                    }
                }

                // =========================================================
                // TRANSFER
                // =========================================================

                "2" => {
                    println!();
                    println!("===== Transfer =====");

                    print!("To: ");

                    io::stdout()
                        .flush()
                        .unwrap();

                    let mut to = String::new();

                    io::stdin()
                        .read_line(&mut to)
                        .unwrap();

                    print!("Amount: ");

                    io::stdout()
                        .flush()
                        .unwrap();

                    let mut amount = String::new();

                    io::stdin()
                        .read_line(&mut amount)
                        .unwrap();

                    let address = Address::new(
                        to.trim().to_string(),
                    );

                    let amount: u64 =
                        match amount.trim().parse() {
                            Ok(value) => value,

                            Err(_) => {
                                println!(
                                    "Invalid amount."
                                );

                                continue;
                            }
                        };

                    wallet.send(
                        node_address,
                        &address,
                        amount,
                        TransactionType::Transfer,
                    );

                    println!(
                        "Transfer transaction broadcast."
                    );
                }

                // =========================================================
                // MINT
                // =========================================================

                "3" => {
                    println!();
                    println!("===== Mint $PEP =====");

                    print!("To: ");

                    io::stdout()
                        .flush()
                        .unwrap();

                    let mut to = String::new();

                    io::stdin()
                        .read_line(&mut to)
                        .unwrap();

                    print!("Amount: ");

                    io::stdout()
                        .flush()
                        .unwrap();

                    let mut amount = String::new();

                    io::stdin()
                        .read_line(&mut amount)
                        .unwrap();

                    let address = Address::new(
                        to.trim().to_string(),
                    );

                    let amount: u64 =
                        match amount.trim().parse() {
                            Ok(value) => value,

                            Err(_) => {
                                println!(
                                    "Invalid amount."
                                );

                                continue;
                            }
                        };

                    wallet.send(
                        node_address,
                        &address,
                        amount,
                        TransactionType::Mint,
                    );

                    println!(
                        "Mint transaction broadcast."
                    );
                }

                // =========================================================
                // BURN
                // =========================================================

                "4" => {
                    println!();
                    println!("===== Burn $PEP =====");

                    println!("Coming Soon");
                }

                // =========================================================
                // WALLET INFO
                // =========================================================

                "5" => {
                    println!();
                    println!("===== Wallet Info =====");

                    println!(
                        "Address : {}",
                        wallet.address()
                    );

                    println!(
                        "Public  : {:02x?}",
                        wallet.public_key().bytes(),
                    );

                    println!(
                        "Mnemonic: {}",
                        wallet.mnemonic()
                    );
                }

                // =========================================================
                // ADD ASSET
                // =========================================================

                "6" => {
                    Self::add_asset(
                        node_address
                    );
                }

                // =========================================================
                // EXIT WALLET
                // =========================================================

                "0" => {
                    break;
                }

                _ => {
                    println!("Invalid choice.");
                }
            }
        }
    }

    // =====================================================================
    // ADD ASSET
    // =====================================================================

    fn add_asset(
        node_address: &str,
    ) {
        println!();
        println!("===== Add Asset =====");

        // -----------------------------------------------------------------
        // NAME
        // -----------------------------------------------------------------

        print!("Asset name: ");

        io::stdout()
            .flush()
            .unwrap();

        let mut asset_name = String::new();

        io::stdin()
            .read_line(&mut asset_name)
            .unwrap();

        let asset_name =
            asset_name.trim().to_string();

        if asset_name.is_empty() {
            println!(
                "Asset name cannot be empty."
            );

            return;
        }

        // -----------------------------------------------------------------
        // TYPE
        // -----------------------------------------------------------------

        print!("Type (Native/Pegged): ");

        io::stdout()
            .flush()
            .unwrap();

        let mut asset_type = String::new();

        io::stdin()
            .read_line(&mut asset_type)
            .unwrap();

        let asset_type =
            asset_type.trim().to_string();

        if asset_type.is_empty() {
            println!(
                "Asset type cannot be empty."
            );

            return;
        }

        // -----------------------------------------------------------------
        // DECIMALS
        // -----------------------------------------------------------------

        print!("Decimals: ");

        io::stdout()
            .flush()
            .unwrap();

        let mut decimals = String::new();

        io::stdin()
            .read_line(&mut decimals)
            .unwrap();

        let decimals: u8 =
            match decimals.trim().parse() {
                Ok(value) => value,

                Err(_) => {
                    println!(
                        "Invalid decimals."
                    );

                    return;
                }
            };

        // -----------------------------------------------------------------
        // SUPPLY
        // -----------------------------------------------------------------

        print!("Supply: ");

        io::stdout()
            .flush()
            .unwrap();

        let mut supply = String::new();

        io::stdin()
            .read_line(&mut supply)
            .unwrap();

        let supply: u64 =
            match supply.trim().parse() {
                Ok(value) => value,

                Err(_) => {
                    println!(
                        "Invalid supply."
                    );

                    return;
                }
            };

        // -----------------------------------------------------------------
        // DEPLOY ADDRESS
        // -----------------------------------------------------------------

        print!("Deploy address: ");

        io::stdout()
            .flush()
            .unwrap();

        let mut deploy_address =
            String::new();

        io::stdin()
            .read_line(&mut deploy_address)
            .unwrap();

        let deploy_address =
            deploy_address.trim().to_string();

        // -----------------------------------------------------------------
        // TRANSFERABLE
        // -----------------------------------------------------------------

        print!(
            "Transferable (true/false): "
        );

        io::stdout()
            .flush()
            .unwrap();

        let mut transferable =
            String::new();

        io::stdin()
            .read_line(&mut transferable)
            .unwrap();

        let transferable: bool =
            match transferable.trim().parse() {
                Ok(value) => value,

                Err(_) => {
                    println!(
                        "Invalid value. Use true or false."
                    );

                    return;
                }
            };

        // -----------------------------------------------------------------
        // GAS
        // -----------------------------------------------------------------

        print!(
            "Gas eligible (true/false): "
        );

        io::stdout()
            .flush()
            .unwrap();

        let mut gas_eligible =
            String::new();

        io::stdin()
            .read_line(&mut gas_eligible)
            .unwrap();

        let gas_eligible: bool =
            match gas_eligible.trim().parse() {
                Ok(value) => value,

                Err(_) => {
                    println!(
                        "Invalid value. Use true or false."
                    );

                    return;
                }
            };

        // -----------------------------------------------------------------
        // PEG
        // -----------------------------------------------------------------

        print!(
            "PEPDEX pricing address: "
        );

        io::stdout()
            .flush()
            .unwrap();

        let mut peg = String::new();

        io::stdin()
            .read_line(&mut peg)
            .unwrap();

        let peg =
            peg.trim().to_string();

        // -----------------------------------------------------------------
        // BUILD ASSET TYPE
        // -----------------------------------------------------------------

        let asset_type_payload =
            format!(
                "{}|{}",
                asset_name,
                asset_type,
            );

        // -----------------------------------------------------------------
        // DISPLAY
        // -----------------------------------------------------------------

        println!();
        println!(
            "===== Asset Registration ====="
        );

        println!(
            "Name       : {}",
            asset_name
        );

        println!(
            "Type       : {}",
            asset_type
        );

        println!(
            "Decimals   : {}",
            decimals
        );

        println!(
            "Supply     : {}",
            supply
        );

        println!(
            "Deploy     : {}",
            deploy_address
        );

        println!(
            "TRS        : {}",
            transferable
        );

        println!(
            "GAS        : {}",
            gas_eligible
        );

        println!(
            "PEG        : {}",
            if peg.is_empty() {
                "-"
            } else {
                &peg
            }
        );

        println!();

        // -----------------------------------------------------------------
        // SEND TO CORE
        // -----------------------------------------------------------------

        match Client::register_asset(
            node_address,
            &asset_type_payload,
            decimals,
            supply,
            &deploy_address,
            transferable,
            gas_eligible,
            &peg,
        ) {
            Some(response) => {
                println!();
                println!(
                    "===== Asset Registration Result ====="
                );

                println!(
                    "{}",
                    response
                );
            }

            None => {
                println!();
                println!(
                    "Asset registration failed."
                );
            }
        }
    }
}