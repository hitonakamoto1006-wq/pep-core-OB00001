use std::{
    io::{Read, Write},
    net::TcpStream,
    process::{Child, Command, Stdio},
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

use tauri::{Manager, State};

use pep_core::{
    blockchain::{
        network::client::Client,
        transaction::TransactionType,
    },
    wallet::{Address, Wallet},
};

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;


/*
 * ============================================================
 * CORE PROCESS
 * ============================================================
 */

struct CoreProcess {
    child: Arc<Mutex<Option<Child>>>,
    status: Arc<Mutex<String>>,
}

impl CoreProcess {

    fn new() -> Self {
        Self {
            child: Arc::new(
                Mutex::new(None)
            ),

            status: Arc::new(
                Mutex::new(
                    "Starting PEP Core..."
                        .to_string()
                )
            ),
        }
    }

    fn set_status(
        &self,
        value: &str,
    ) {
        if let Ok(mut status) =
            self.status.lock()
        {
            *status =
                value.to_string();
        }
    }

    fn start(&self) {

        let child_store =
            Arc::clone(&self.child);

        let status_store =
            Arc::clone(&self.status);

        thread::spawn(
            move || {

                /*
                 * Check local Core.
                 */

                if try_handshake(
                    "127.0.0.1:6000"
                ) {

                    if let Ok(
                        mut status
                    ) =
                        status_store.lock()
                    {
                        *status =
                            "PEP Core connected."
                                .to_string();
                    }

                    return;
                }


                /*
                 * Locate current executable.
                 */

                let executable =
                    match std::env::current_exe()
                    {
                        Ok(path) => path,

                        Err(error) => {

                            if let Ok(
                                mut status
                            ) =
                                status_store.lock()
                            {
                                *status =
                                    format!(
                                        "Cannot locate PEP Wallet executable: {error}"
                                    );
                            }

                            return;
                        }
                    };


                if let Ok(
                    mut status
                ) =
                    status_store.lock()
                {
                    *status =
                        "Starting PEP Core..."
                            .to_string();
                }


                println!(
                    "[PEP Wallet] Starting embedded PEP Node: {}",
                    executable.display()
                );


                /*
                 * Start same executable
                 * in daemon mode.
                 */

                let mut command =
                    Command::new(
                        &executable
                    );
                    let data_root =
    executable
        .parent()
        .and_then(|p| p.parent())
        .and_then(|p| p.parent())
        .map(|p| p.to_path_buf());

if let Some(root) = data_root {
    command.current_dir(root);
}

                command
    .arg("--pep-node-daemon")
    .stdin(Stdio::null())
    .stdout(Stdio::inherit())
    .stderr(Stdio::inherit());


                #[cfg(target_os = "windows")]
                command.creation_flags(
                    CREATE_NO_WINDOW
                );


                let child_result =
                    command.spawn();


                let child =
                    match child_result {

                        Ok(child) =>
                            child,

                        Err(error) => {

                            if let Ok(
                                mut status
                            ) =
                                status_store.lock()
                            {
                                *status =
                                    format!(
                                        "Failed to start PEP Core: {error}"
                                    );
                            }

                            return;
                        }
                    };


                {
                    let mut guard =
                        child_store
                            .lock()
                            .unwrap();

                    *guard =
                        Some(child);
                }


                if let Ok(
                    mut status
                ) =
                    status_store.lock()
                {
                    *status =
                        "PEP Core process started. Connecting..."
                            .to_string();
                }


                /*
                 * Wait up to ~10 seconds.
                 */

                let mut connected =
                    false;

                for _ in 0..100 {

                    if try_handshake(
                        "127.0.0.1:6000"
                    ) {

                        connected =
                            true;

                        break;
                    }

                    thread::sleep(
                        Duration::from_millis(
                            100
                        )
                    );
                }


                if connected {

                    if let Ok(
                        mut status
                    ) =
                        status_store.lock()
                    {
                        *status =
                            "PEP Core connected."
                                .to_string();
                    }

                } else {

                    if let Ok(
                        mut status
                    ) =
                        status_store.lock()
                    {
                        *status =
                            "PEP Core started, but connection failed."
                                .to_string();
                    }
                }
            }
        );
    }
}


/*
 * ============================================================
 * WALLET SESSION
 * ============================================================
 *
 * Wallet private material stays in Rust.
 * JavaScript only receives public information.
 * ============================================================
 */

struct WalletSession {
    mnemonic: Mutex<Option<String>>,
}

impl WalletSession {

    fn new() -> Self {
        Self {
            mnemonic: Mutex::new(None),
        }
    }

    fn set_mnemonic(
        &self,
        mnemonic: String,
    ) -> Result<(), String> {

        let mut guard =
            self.mnemonic
                .lock()
                .map_err(
                    |_| {
                        "Wallet session lock failed."
                            .to_string()
                    }
                )?;

        *guard =
            Some(mnemonic);

        Ok(())
    }

    fn wallet(
        &self,
    ) -> Result<Wallet, String> {

        let guard =
            self.mnemonic
                .lock()
                .map_err(
                    |_| {
                        "Wallet session lock failed."
                            .to_string()
                    }
                )?;

        let mnemonic =
            guard
                .as_ref()
                .ok_or_else(
                    || {
                        "No wallet loaded."
                            .to_string()
                    }
                )?;

        Wallet::from_phrase(
            mnemonic
        )
    }
}


/*
 * ============================================================
 * DROP
 * ============================================================
 */

impl Drop for CoreProcess {

    fn drop(
        &mut self
    ) {

        if let Ok(
            mut guard
        ) =
            self.child.lock()
        {

            if let Some(
                mut child
            ) =
                guard.take()
            {

                println!(
                    "[PEP Wallet] Stopping embedded PEP Node..."
                );

                let _ =
                    child.kill();

                let _ =
                    child.wait();
            }
        }
    }
}


/*
 * ============================================================
 * DAEMON MODE
 * ============================================================
 */

fn run_pep_node_daemon() {
    println!(
    "[PEP Node] Current directory: {:?}",
    std::env::current_dir()
        .unwrap_or_default()
);

println!(
    "[PEP Node] Data directory: {:?}",
    std::path::Path::new("data")
        .canonicalize()
        .unwrap_or_else(|_| {
            std::path::PathBuf::from("data")
        })
);

    println!(
        "[PEP Node] Starting embedded PEP Core..."
    );

    pep_core::blockchain::network::core::Core::start(
        "0.0.0.0:6000",
        None,
    );
}


/*
 * ============================================================
 * HANDSHAKE
 * ============================================================
 */

fn try_handshake(
    address: &str,
) -> bool {

    let mut stream =
        match TcpStream::connect(
            address
        ) {

            Ok(stream) =>
                stream,

            Err(_) =>
                return false,
        };


    let ping = [
        1u8,
        0u8,
        0u8,
        0u8,
        0u8,
    ];


    if stream
        .write_all(&ping)
        .is_err()
    {
        return false;
    }


    let mut header =
        [0u8; 5];


    if stream
        .read_exact(
            &mut header
        )
        .is_err()
    {
        return false;
    }


    let message_type =
        header[0];

    let payload_length =
        u32::from_be_bytes([
            header[1],
            header[2],
            header[3],
            header[4],
        ]);


    message_type == 2
        &&
    payload_length == 0
}


/*
 * ============================================================
 * CREATE WALLET
 * ============================================================
 */

#[derive(
    serde::Serialize
)]
struct CreateWalletResponse {

    address: String,

    mnemonic: String,
}


#[tauri::command]
fn create_wallet(
    state: State<'_, CoreProcess>,
    wallet_session: State<'_, WalletSession>,
)
    -> Result<
        CreateWalletResponse,
        String
    >
{

    if !try_handshake(
        "127.0.0.1:6000"
    ) {

        return Err(
            "PEP Core is not connected."
                .to_string()
        );
    }


    let wallet =
        Wallet::new();


    let address =
        wallet
            .address()
            .to_string();


    let mnemonic =
        wallet
            .mnemonic()
            .to_string();


    wallet_session
        .set_mnemonic(
            mnemonic.clone()
        )?;


    let _ =
        state;


    Ok(
        CreateWalletResponse {

            address,

            mnemonic,
        }
    )
}


/*
 * ============================================================
 * IMPORT WALLET
 * ============================================================
 */

#[derive(
    serde::Serialize
)]
struct ImportWalletResponse {

    address: String,
}


#[tauri::command]
fn import_wallet(
    mnemonic: String,
    wallet_session: State<'_, WalletSession>,
)
    -> Result<
        ImportWalletResponse,
        String
    >
{

    let phrase =
        mnemonic
            .trim()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");


    if phrase.is_empty() {

        return Err(
            "Recovery phrase is empty."
                .to_string()
        );
    }


    let wallet =
        Wallet::from_phrase(
            &phrase
        )?;


    let address =
        wallet
            .address()
            .to_string();


    wallet_session
        .set_mnemonic(
            phrase
        )?;


    Ok(
        ImportWalletResponse {
            address,
        }
    )
}


/*
 * ============================================================
 * BALANCE
 * ============================================================
 */

#[derive(
    serde::Serialize
)]
struct BalanceItem {

    asset: String,

    amount: u64,
}


#[derive(
    serde::Serialize
)]
struct BalanceResponse {

    balances:
        Vec<BalanceItem>,

    nonce: u64,

    stake: u64,
}


#[tauri::command]
fn wallet_balance(
    wallet_session:
        State<'_, WalletSession>,
)
    -> Result<
        BalanceResponse,
        String
    >
{

    let wallet =
        wallet_session
            .wallet()?;


    let result =
        Client::get_balance(
            "127.0.0.1:6000",
            wallet.address(),
        )
        .ok_or_else(
            || {
                "Cannot connect to local PEP Core."
                    .to_string()
            }
        )?;


    let (
        balances,
        nonce,
        stake,
    ) =
        result;


    Ok(
        BalanceResponse {

            balances:
                balances
                    .into_iter()
                    .map(
                        |(
                            asset,
                            amount
                        )| {

                            BalanceItem {
                                asset,
                                amount,
                            }
                        }
                    )
                    .collect(),

            nonce,

            stake,
        }
    )
}


/*
 * ============================================================
 * WALLET INFO
 * ============================================================
 */

#[derive(
    serde::Serialize
)]
struct WalletInfoResponse {

    address: String,

    public_key: String,
}


#[tauri::command]
fn wallet_info(
    wallet_session:
        State<'_, WalletSession>,
)
    -> Result<
        WalletInfoResponse,
        String
    >
{

    let wallet =
        wallet_session
            .wallet()?;


    Ok(
        WalletInfoResponse {

            address:
                wallet
                    .address()
                    .to_string(),

            public_key:
                format!(
                    "{:02x?}",
                    wallet
                        .public_key()
                        .bytes()
                ),
        }
    )
}


/*
 * ============================================================
 * TRANSFER
 * ============================================================
 */

#[tauri::command]
fn transfer(
    wallet_session:
        State<'_, WalletSession>,

    to: String,

    amount: u64,
)
    -> Result<
        String,
        String
    >
{

    if amount == 0 {

        return Err(
            "Amount must be greater than zero."
                .to_string()
        );
    }


    let wallet =
        wallet_session
            .wallet()?;


    let address =
        Address::new(
            to.trim().to_string()
        );


    wallet.send(
        "127.0.0.1:6000",
        &address,
        amount,
        TransactionType::Transfer,
    );


    Ok(
        "Transfer transaction broadcast."
            .to_string()
    )
}


/*
 * ============================================================
 * MINT
 * ============================================================
 */

#[tauri::command]
fn mint(
    wallet_session:
        State<'_, WalletSession>,

    to: String,

    amount: u64,
)
    -> Result<
        String,
        String
    >
{

    if amount == 0 {

        return Err(
            "Amount must be greater than zero."
                .to_string()
        );
    }


    let wallet =
        wallet_session
            .wallet()?;


    let address =
        Address::new(
            to.trim().to_string()
        );


    wallet.send(
        "127.0.0.1:6000",
        &address,
        amount,
        TransactionType::Mint,
    );


    Ok(
        "Mint transaction broadcast."
            .to_string()
    )
}


/*
 * ============================================================
 * ADD ASSET
 * ============================================================
 */

#[tauri::command]
fn add_asset(
    name: String,
    asset_type: String,
    decimals: u8,
    supply: u64,
    deploy_address: String,
    transferable: bool,
    gas_eligible: bool,
    peg: String,
)
    -> Result<
        String,
        String
    >
{

    let name =
        name.trim();

    let asset_type =
        asset_type.trim();


    if name.is_empty() {

        return Err(
            "Asset name cannot be empty."
                .to_string()
        );
    }


    if asset_type.is_empty() {

        return Err(
            "Asset type cannot be empty."
                .to_string()
        );
    }


    Client::register_asset(
        "127.0.0.1:6000",

        &format!(
            "{}|{}",
            name,
            asset_type
        ),

        decimals,

        supply,

        deploy_address
            .trim(),

        transferable,

        gas_eligible,

        peg.trim(),
    )
    .ok_or_else(
        || {
            "Asset registration failed."
                .to_string()
        }
    )
}


/*
 * ============================================================
 * CORE STATUS
 * ============================================================
 */

#[tauri::command]
fn core_status(
    state:
        State<'_, CoreProcess>,
)
    -> String
{

    if try_handshake(
        "127.0.0.1:6000"
    ) {

        state.set_status(
            "PEP Core connected."
        );

        return
            "PEP Core connected."
                .to_string();
    }


    state
        .status
        .lock()
        .map(
            |status|
                status.clone()
        )
        .unwrap_or_else(
            |_| {
                "Unknown"
                    .to_string()
            }
        )
}


/*
 * ============================================================
 * TAURI ENTRY
 * ============================================================
 */

#[cfg_attr(
    mobile,
    tauri::mobile_entry_point
)]
pub fn run() {

    let is_node_daemon =
        std::env::args()
            .any(
                |arg|
                    arg ==
                    "--pep-node-daemon"
            );


    /*
     * Node mode:
     *
     * No Tauri GUI.
     */

    if is_node_daemon {

        run_pep_node_daemon();

        return;
    }


    /*
     * Normal Desktop Wallet.
     */

    tauri::Builder::default()

        .setup(
            |app| {

                let core =
                    CoreProcess::new();


                core.start();


                app.manage(
                    core
                );


                app.manage(
                    WalletSession::new()
                );


                Ok(())
            }
        )

        .invoke_handler(
            tauri::generate_handler![
                core_status,
                create_wallet,
                import_wallet,
                wallet_balance,
                wallet_info,
                transfer,
                mint,
                add_asset
            ]
        )

        .run(
            tauri::generate_context!()
        )

        .expect(
            "error while running PEP Desktop"
        );
}