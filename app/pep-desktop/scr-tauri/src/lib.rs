use std::{
    io::{Read, Write},
    net::TcpStream,
    process::{Child, Command, Stdio},
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

use tauri::{Manager, State};
use pep_core::wallet::Wallet;
use pep_core::blockchain::{
    network::client::Client,
    transaction::TransactionType,
};

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

use tauri::{Manager, State};

use pep_core::wallet::Wallet;

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;


/*
 * ============================================================
 * WINDOWS PROCESS FLAGS
 * ============================================================
 *
 * CREATE_NO_WINDOW:
 *
 * Node process chạy background hoàn toàn.
 *
 * User chỉ thấy PEP Wallet.
 *
 * ============================================================
 */


/*
 * ============================================================
 * CORE PROCESS
 * ============================================================
 *
 * Wallet GUI và PEP Node vẫn là hai process riêng.
 *
 * Nhưng cả hai process đều sử dụng:
 *
 *     PEP Wallet.exe
 *
 * Process chính:
 *
 *     PEP Wallet.exe
 *
 * Process daemon:
 *
 *     PEP Wallet.exe --pep-node-daemon
 *
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


    /*
     * ========================================================
     * START PEP NODE DAEMON
     * ========================================================
     */

    fn start(&self) {

        let child_store =
            Arc::clone(&self.child);

        let status_store =
            Arc::clone(&self.status);


        thread::spawn(
            move || {

                /*
                 * ==================================================
                 * STEP 1
                 *
                 * Check whether PEP Node is already running.
                 * ==================================================
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
                 * ==================================================
                 * STEP 2
                 *
                 * No Node.
                 *
                 * Launch THIS executable again.
                 *
                 * ==================================================
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
                 * ==================================================
                 * CHILD PROCESS
                 *
                 * Same executable.
                 *
                 * Argument:
                 *
                 *     --pep-node-daemon
                 *
                 * ==================================================
                 */

                let mut command =
                    Command::new(
                        &executable
                    );


                command
                    .arg("--pep-node-daemon")
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null());


                /*
                 * Windows:
                 *
                 * Do not create console window.
                 */

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


                /*
                 * Save child process.
                 */

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
                 * ==================================================
                 * STEP 3
                 *
                 * Wait for Node.
                 *
                 * 100 × 100ms
                 *
                 * Maximum ≈ 10 seconds.
                 * ==================================================
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


                /*
                 * ==================================================
                 * STEP 4
                 * ==================================================
                 */

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
 * DROP
 * ============================================================
 *
 * IMPORTANT:
 *
 * Wallet đóng KHÔNG kill Node.
 *
 * Node là daemon độc lập.
 *
 * ============================================================
 */

impl Drop for CoreProcess {

    fn drop(&mut self) {

        /*
         * Wallet GUI đang quản lý daemon mà nó đã spawn.
         *
         * Khi Wallet đóng:
         *
         *     PEP Wallet GUI
         *          ↓
         *        DROP
         *          ↓
         *     kill embedded node
         *
         * Vì vậy Node không tồn tại độc lập sau khi
         * Wallet đóng.
         */

        if let Ok(mut guard) = self.child.lock() {

            if let Some(mut child) = guard.take() {

                println!(
                    "[PEP Wallet] Stopping embedded PEP Node..."
                );

                let _ = child.kill();

                let _ = child.wait();
            }
        }
    }
}


/*
 * ============================================================
 * PEP NODE DAEMON MODE
 * ============================================================
 *
 * Đây chính là phần khiến một EXE có thể đóng vai trò Node.
 *
 * Khi chạy:
 *
 *     PEP Wallet.exe
 *
 * → Wallet GUI.
 *
 *
 * Khi chạy:
 *
 *     PEP Wallet.exe --pep-node-daemon
 *
 * → PEP Core / Node.
 *
 * ============================================================
 */

fn run_pep_node_daemon() {

    println!(
        "[PEP Node] Starting embedded PEP Core..."
    );


    /*
     * Core::start() là Node engine hiện tại
     * của PEP Chain.
     *
     * Không mở Wallet UI ở mode này.
     */

    pep_core::blockchain::network::core::Core::start(
        "0.0.0.0:6000",
        None,
    );
}


/*
 * ============================================================
 * HANDSHAKE
 * ============================================================
 *
 * Ping:
 *
 *     01 00 00 00 00
 *
 * Pong:
 *
 *     02 00 00 00 00
 *
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
        .read_exact(&mut header)
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
)
    -> Result<
        CreateWalletResponse,
        String
    >
{

    /*
     * Wallet creation requires Core.
     */

    if !try_handshake(
        "127.0.0.1:6000"
    ) {

        return Err(
            "PEP Core is not connected."
                .to_string()
        );
    }


    /*
     * Use existing PEP wallet engine.
     */

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
        pep_core::wallet::Wallet::from_phrase(
            &phrase
        )?;


    let address =
        wallet
            .address()
            .to_string();


    Ok(
        ImportWalletResponse {
            address,
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
    state: State<'_, CoreProcess>,
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
 * TAURI ENTRY POINT
 * ============================================================
 */

#[cfg_attr(
    mobile,
    tauri::mobile_entry_point
)]
pub fn run() {

    /*
     * ========================================================
     * IMPORTANT
     *
     * Check daemon mode BEFORE creating Tauri.
     *
     * Otherwise the Node child would also create
     * a Wallet GUI.
     * ========================================================
     */

    let is_node_daemon =
        std::env::args()
            .any(
                |arg|
                    arg ==
                    "--pep-node-daemon"
            );


    if is_node_daemon {

        run_pep_node_daemon();

        return;
    }


    /*
     * ========================================================
     * NORMAL WALLET MODE
     * ========================================================
     */

    tauri::Builder::default()

        /*
         * ====================================================
         * SETUP
         * ====================================================
         */

        .setup(
            |app| {

                let core =
                    CoreProcess::new();


                /*
                 * Automatically start/check Node.
                 */

                core.start();


                /*
                 * Register Core state.
                 */

                app.manage(
                    core
                );


                Ok(())
            }
        )


        /*
         * ====================================================
         * COMMANDS
         * ====================================================
         */

        .invoke_handler(
            tauri::generate_handler![
                core_status,
                create_wallet,
                import_wallet
            ]
        )


        /*
         * ====================================================
         * RUN
         * ====================================================
         */

        .run(
            tauri::generate_context!()
        )

        .expect(
            "error while running PEP Desktop"
        );
}