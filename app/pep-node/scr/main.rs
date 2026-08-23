use std::{
    env,
    net::SocketAddr,
    sync::Arc,
    thread,
    time::Duration,
};

use axum::{
    extract::State,
    http::StatusCode,
    routing::{get, post},
    Json,
    Router,
};

use serde::{
    Deserialize,
    Serialize,
};

use tokio::sync::RwLock;

use pep_core::blockchain::network::core::Core;


// ============================================================
// PEP BOOTSTRAP REGISTRY
// ============================================================

#[derive(
    Clone,
    Default,
)]
struct BootstrapState {

    peers:
        Arc<
            RwLock<
                Vec<String>
            >
        >,
}


// ============================================================
// RESPONSE
// ============================================================

#[derive(
    Serialize,
)]
struct PeersResponse {

    peers:
        Vec<String>,
}


// ============================================================
// REGISTER REQUEST
// ============================================================

#[derive(
    Deserialize,
)]
struct RegisterRequest {

    address:
        String,
}


// ============================================================
// HEALTH
// ============================================================

async fn health()
    -> &'static str
{
    "PEP bootstrap OK"
}


// ============================================================
// GET /peers
// ============================================================

async fn get_peers(
    State(state): State<BootstrapState>,
) -> Json<PeersResponse> {

    let peers =
        state
            .peers
            .read()
            .await
            .clone();

    Json(
        PeersResponse {
            peers,
        }
    )
}


// ============================================================
// POST /register
// ============================================================
//
// A node sends:
//
// {
//     "address": "PUBLIC_IP:6000"
// }
//
// Bootstrap returns all currently known peers.
//
// ============================================================

async fn register_peer(
    State(state): State<BootstrapState>,

    Json(request): Json<RegisterRequest>,

) -> (
    StatusCode,
    Json<PeersResponse>,
) {

    let address =
        request
            .address
            .trim()
            .to_string();


    // --------------------------------------------------------
    // Validate SocketAddr
    // --------------------------------------------------------

    let parsed:
        SocketAddr =
        match address.parse()
        {

            Ok(address) =>
                address,

            Err(_) => {

                return (
                    StatusCode::BAD_REQUEST,

                    Json(
                        PeersResponse {
                            peers:
                                Vec::new(),
                        }
                    )
                );
            }
        };


    // --------------------------------------------------------
    // Do not allow unspecified address.
    //
    // 0.0.0.0:6000 is a bind address, not a peer address.
    // --------------------------------------------------------

    if parsed
        .ip()
        .is_unspecified()
        ||
        parsed.port() == 0
    {

        return (
            StatusCode::BAD_REQUEST,

            Json(
                PeersResponse {
                    peers:
                        Vec::new(),
                }
            )
        );
    }


    let mut peers =
        state
            .peers
            .write()
            .await;


    // --------------------------------------------------------
    // Deduplicate
    // --------------------------------------------------------

    if !peers
        .iter()
        .any(
            |peer|
                peer == &address
        )
    {

        println!(
            "[PEP Bootstrap] Registering peer {}",
            address
        );

        peers.push(
            address.clone()
        );
    }


    // --------------------------------------------------------
    // Return peers.
    //
    // Do not return ourselves to the registering node.
    // --------------------------------------------------------

    let result =
        peers
            .iter()
            .filter(
                |peer|
                    *peer != &address
            )
            .cloned()
            .collect::<Vec<_>>();


    println!(
        "[PEP Bootstrap] {} registered. Known peers: {}",
        address,
        peers.len()
    );


    (
        StatusCode::OK,

        Json(
            PeersResponse {
                peers:
                    result,
            }
        )
    )
}


// ============================================================
// POST /unregister
// ============================================================
//
// Used when a node shuts down cleanly.
//
// ============================================================

async fn unregister_peer(
    State(state): State<BootstrapState>,

    Json(request): Json<RegisterRequest>,

) -> (
    StatusCode,
    Json<PeersResponse>,
) {

    let address =
        request
            .address
            .trim()
            .to_string();


    let mut peers =
        state
            .peers
            .write()
            .await;


    let before =
        peers.len();


    peers.retain(
        |peer|
            peer != &address
    );


    let removed =
        before != peers.len();


    if removed {

        println!(
            "[PEP Bootstrap] Unregistered peer {}",
            address
        );
    }


    (
        StatusCode::OK,

        Json(
            PeersResponse {
                peers:
                    peers.clone(),
            }
        )
    )
}


// ============================================================
// BOOTSTRAP HTTP SERVER
// ============================================================

async fn run_bootstrap_server(
    bind_address: String,
    state: BootstrapState,
) {

    let app =
        Router::new()

            // ------------------------------------------------
            // Health
            // ------------------------------------------------

            .route(
                "/",
                get(health)
            )

            .route(
                "/health",
                get(health)
            )

            // ------------------------------------------------
            // Peer registry
            // ------------------------------------------------

            .route(
                "/peers",
                get(get_peers)
            )

            .route(
                "/register",
                post(register_peer)
            )

            .route(
                "/unregister",
                post(unregister_peer)
            )

            .with_state(
                state
            );


    // --------------------------------------------------------
    // Bind
    // --------------------------------------------------------

    let listener =
        match tokio::net::TcpListener::bind(
            &bind_address
        )
        .await
    {

        Ok(listener) =>
            listener,

        Err(error) => {

            eprintln!(
                "[PEP Bootstrap] Cannot bind {}: {}",
                bind_address,
                error
            );

            return;
        }
    };


    println!(
        "[PEP Bootstrap] HTTP listening on {}",
        bind_address
    );


    // --------------------------------------------------------
    // Serve forever
    // --------------------------------------------------------

    if let Err(error) =
        axum::serve(
            listener,
            app
        )
        .await
    {

        eprintln!(
            "[PEP Bootstrap] HTTP server stopped: {}",
            error
        );
    }
}


// ============================================================
// START HTTP BOOTSTRAP
// ============================================================

fn start_bootstrap_http(
    bind_address: String,
    state: BootstrapState,
) {

    thread::spawn(
        move || {

            let runtime =
                match tokio::runtime::Runtime::new()
                {

                    Ok(runtime) =>
                        runtime,

                    Err(error) => {

                        eprintln!(
                            "[PEP Bootstrap] Cannot create Tokio runtime: {}",
                            error
                        );

                        return;
                    }
                };


            runtime.block_on(
                run_bootstrap_server(
                    bind_address,
                    state,
                )
            );
        }
    );
}


// ============================================================
// MAIN
// ============================================================

fn main() {

    println!();

    println!(
        "======================================"
    );

    println!(
        "          PEP NETWORK NODE            "
    );

    println!(
        "======================================"
    );


    // ========================================================
    // MODE
    // ========================================================

    let bootstrap_only =
        env::var(
            "PEP_BOOTSTRAP_ONLY"
        )
        .unwrap_or_else(
            |_| "false".to_string()
        )
        .trim()
        .eq_ignore_ascii_case(
            "true"
        );


    if bootstrap_only {

        println!(
            "[PEP Node] Mode: BOOTSTRAP SERVER"
        );

    } else {

        println!(
            "[PEP Node] Mode: FULL P2P NODE"
        );
    }


    // ========================================================
    // P2P LISTEN
    // ========================================================

    let listen_address =
        env::var(
            "PEP_LISTEN"
        )
        .unwrap_or_else(
            |_| {
                "0.0.0.0:6000"
                    .to_string()
            }
        );


    println!(
        "[PEP Node] P2P listen: {}",
        listen_address
    );


    // ========================================================
// BOOTSTRAP URL
// ========================================================
//
// Nếu PEP_BOOTSTRAP được cấu hình thì dùng nó.
// Nếu không, node tự dùng bootstrap chính thức.
//
// ========================================================

const DEFAULT_PEP_BOOTSTRAP: &str =
    "https://pep-core-bootstrap-node.onrender.com";

let bootstrap =
    env::var("PEP_BOOTSTRAP")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .or_else(|| {
            Some(
                DEFAULT_PEP_BOOTSTRAP.to_string()
            )
        });

match bootstrap.as_deref() {

    Some(address) => {
        println!(
            "[PEP Node] Bootstrap: {}",
            address
        );
    }

    None => {
        println!(
            "[PEP Node] Bootstrap: none"
        );
    }
}


    // ========================================================
    // HTTP PORT
    // ========================================================
    //
    // Render provides:
    //
    //     PORT
    //
    // Local:
    //
    //     PORT=8080
    //
    // ========================================================

    let http_port =
        env::var(
            "PORT"
        )
        .unwrap_or_else(
            |_| "8080".to_string()
        );


    let http_bind =
        format!(
            "0.0.0.0:{}",
            http_port
        );


    println!(
        "[PEP Bootstrap] HTTP bind: {}",
        http_bind
    );


        // ========================================================
    // BOOTSTRAP REGISTRY
    // ========================================================

    if bootstrap_only {

        let bootstrap_state =
            BootstrapState::default();

        start_bootstrap_http(
            http_bind,
            bootstrap_state,
        );
    }


    // ========================================================
    // BOOTSTRAP-ONLY MODE
    // ========================================================
    //
    // Render bootstrap MUST NOT start the raw P2P Core.
    //
    // Render Web Service exposes its HTTP service port.
    // The PEP TCP :6000 listener therefore belongs to actual
    // P2P nodes, not the Render bootstrap registry.
    //
    // ========================================================

    if bootstrap_only {

        println!();

        println!(
            "[PEP Bootstrap] Bootstrap server mode active."
        );

        println!(
            "[PEP Bootstrap] P2P Core is NOT started."
        );

        println!(
            "[PEP Bootstrap] Waiting for HTTP requests..."
        );

        loop {

            thread::sleep(
                Duration::from_secs(3600)
            );
        }
    }


    // ========================================================
    // BOOTSTRAP-ONLY MODE
    // ========================================================
    //
    // Render bootstrap MUST NOT start the raw P2P Core.
    //
    // Render Web Service exposes its HTTP service port.
    // The PEP TCP :6000 listener therefore belongs to actual
    // P2P nodes, not the Render bootstrap registry.
    //
    // ========================================================

    if bootstrap_only {

        println!();
        println!(
            "[PEP Bootstrap] Bootstrap server mode active."
        );

        println!(
            "[PEP Bootstrap] P2P Core is NOT started."
        );

        println!(
            "[PEP Bootstrap] Waiting for HTTP requests..."
        );

        loop {

            thread::sleep(
                Duration::from_secs(3600)
            );
        }
    }


    // ========================================================
    // START P2P CORE
    // ========================================================

    println!();

    println!(
        "[PEP Node] Starting P2P core..."
    );


    Core::start(
        &listen_address,
        bootstrap.as_deref(),
    );


    // ========================================================
    // CORE STOPPED
    // ========================================================

    println!(
        "[PEP Node] P2P core stopped."
    );


    // Keep HTTP bootstrap alive if Core ever exits.

    loop {

        thread::sleep(
            Duration::from_secs(3600)
        );
    }
}