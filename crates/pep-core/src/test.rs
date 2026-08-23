use std::io::{self, Write};
use primitive_types::U256;

use crate::wallet::ethereum::{
    Address,
    Transaction,
};

use crate::wallet::ethereum::broadcast::client::EvmClient;
use crate::wallet::{
    hd::master::MasterKey,
    mnemonic::Mnemonic,
    seed::Seed,
    ethereum::{
        Provider,
        Wallet,
    },
};

pub fn run() {

    println!("==============================");
    println!("PEP Portal Wallet Test");
    println!("==============================");
    println!();

    print!("Enter your 25-word mnemonic:\n> ");
    io::stdout().flush().unwrap();

    let mut phrase = String::new();

    io::stdin()
        .read_line(&mut phrase)
        .unwrap();

    let phrase = phrase.trim();

    let words: Vec<&str> =
        phrase.split_whitespace().collect();

    if words.len() != 25 {

        println!();
        println!(
            "Error: expected 25 words, got {}.",
            words.len(),
        );

        return;

    }

    let mnemonic =
        match Mnemonic::from_phrase(
            phrase,
        ) {

            Ok(m) => m,

            Err(e) => {

                println!();
                println!(
                    "Mnemonic Error: {}",
                    e,
                );

                return;

            }

        };

    let seed =
        Seed::from_mnemonic(
            &mnemonic,
            "",
        );

    let master =
        match MasterKey::new(
            &seed,
        ) {

            Ok(m) => m,

            Err(e) => {

                println!(
                    "MasterKey Error: {}",
                    e,
                );

                return;

            }

        };

    let wallet =
        match Wallet::from_master(
            &master,
            0,
            0,
        ) {

            Ok(w) => w,

            Err(e) => {

                println!(
                    "Wallet Error: {}",
                    e,
                );

                return;

            }

        };

    println!();
    println!("==============================");
    println!("Address");
    println!("==============================");
    println!(
        "{}",
        wallet.address_string(),
    );

    println!();

    println!("==============================");
    println!("Private Key");
    println!("==============================");
    println!(
        "{}",
        wallet.private_key_hex(),
    );

    println!();

    println!("==============================");
    println!("Public Key");
    println!("==============================");
    println!(
        "{}",
        wallet.public_key_hex(),
    );

    println!();

    let provider =
        Provider::new(
            "https://bsc-dataseed.binance.org/",
        );

    println!("==============================");
    println!("RPC");
    println!("==============================");

    match provider.chain_id() {

        Ok(id) => {

            println!(
                "Chain ID : {}",
                id,
            );

        }

        Err(e) => {

            println!(
                "Chain ID Error : {}",
                e,
            );

        }

    }

    match provider.balance(
        wallet.address(),
    ) {

        Ok(balance) => {

            println!(
                "Balance : {} wei",
                balance,
            );

        }

        Err(e) => {

            println!(
                "Balance Error : {}",
                e,
            );

        }

    }

    match provider.nonce(
        wallet.address(),
    ) {

        Ok(nonce) => {

            println!(
                "Nonce : {}",
                nonce,
            );

        }

        Err(e) => {

            println!(
                "Nonce Error : {}",
                e,
            );

        }

    }

    println!();
println!("==============================");
println!("SUCCESS");
println!("==============================");

let client = EvmClient::new(
    "https://bsc-dataseed.binance.org/",
);

print!("Receiver address:\n> ");
io::stdout().flush().unwrap();

let mut receiver = String::new();

io::stdin()
    .read_line(&mut receiver)
    .unwrap();

let receiver =
    Address::from_hex(
        receiver.trim(),
    )
    .unwrap();
println!();

print!("Amount (ETH/BNB, A = All):\n> ");
io::stdout().flush().unwrap();

let mut amount = String::new();

io::stdin()
    .read_line(&mut amount)
    .unwrap();

let gas_limit = U256::from(21_000u64);
let max_fee = U256::from(5_000_000_000u64);

let value = if amount.trim().eq_ignore_ascii_case("A") {

    let balance =
        wallet.balance(&provider).unwrap();

    let fee =
        gas_limit * max_fee;

    if balance <= fee {

        println!("Insufficient balance for gas.");
        return;

    }

    balance - fee

} else {

    let amount: f64 =
        match amount.trim().parse() {

            Ok(v) => v,

            Err(_) => {

                println!("Invalid amount.");
                return;

            }

        };

    U256::from(
        (amount
            * 1_000_000_000_000_000_000f64)
            as u128,
    )

};
let tx = Transaction::new()
    .set_chain_id(
        provider.chain_id().unwrap(),
    )
    .set_nonce(
        wallet.nonce(&provider).unwrap(),
    )
    .set_gas_limit(21_000)
    .set_max_priority_fee_per_gas(
        U256::from(1_000_000_000u64),
    )
    .set_max_fee_per_gas(
        U256::from(5_000_000_000u64),
    )
    .set_to(receiver)
    .set_value(value);

match client.send_transaction(
    wallet.private_key(),
    &tx,
) {

    Ok(hash) => {

        println!(
            "TX Hash: {}",
            hash,
        );

    }

    Err(e) => {

        println!(
            "Send Error: {}",
            e,
        );

    }
}
}