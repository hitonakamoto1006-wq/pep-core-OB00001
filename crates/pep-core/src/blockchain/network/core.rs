use std::{
    env,
    net::{
        SocketAddr,
        TcpListener,
        TcpStream,
        UdpSocket,
    },
    sync::{
        Arc,
        Mutex,
    },
    thread,
    time::Duration,
};

use serde::Deserialize;

use crate::blockchain::asset::{
    AssetDefinition,
    AssetId,
    AssetRegistry,
    AssetType,
};

use crate::blockchain::block::Block;

use crate::blockchain::network::{
    message::Message,
    peer::PeerManager,
};

use crate::blockchain::node::Node;

use crate::wallet::Address;


// ============================================================
// PEP NETWORK CONSTANTS
// ============================================================



const PEP_PROTOCOL_VERSION: u32 = 1;

const PEP_CHAIN_ID: u64 = 1;

const PEP_NETWORK: &str = "mainnet";

const MAX_BLOCK_BATCH: usize = 128;


// ============================================================
// BOOTSTRAP RESPONSE
// ============================================================

#[derive(Debug, Deserialize)]
struct BootstrapPeer {
    address: String,
    last_seen: u64,
}

#[derive(Debug, Deserialize)]
struct BootstrapPeersResponse {
    peer: BootstrapPeer,
    peers: Vec<BootstrapPeer>,
}


// ============================================================
// CORE
// ============================================================

pub struct Core;

impl Core {
        // ========================================================
    // DISCOVERY PORT
    // ========================================================

    fn discovery_port() -> u16 {
    if let Ok(value) = env::var("PEP_DISCOVERY_PORT") {
        if let Ok(port) = value.trim().parse::<u16>() {
            if port != 0 {
                return port;
            }
        }
    }

    if let Ok(value) = env::var("PEP_P2P_PORT") {
        if let Ok(port) = value.trim().parse::<u16>() {
            if port != 0 {
                return port.saturating_add(1);
            }
        }
    }

    6001
}

    // ========================================================
    // START
    // ========================================================

        pub fn start(
        address: &str,
        bootstrap: Option<&str>,
    ) {

        let discovery_port =
            Self::discovery_port();

        let listener =
            TcpListener::bind(address)
                .expect(
                    "Cannot bind PEP Core TCP address"
                );

        println!(
            "PEP Core listening on {}",
            address
        );

        let peers =
            Arc::new(
                Mutex::new(
                    PeerManager::new()
                )
            );

        let node =
            Arc::new(
                Mutex::new(
                    Node::new()
                )
            );

        // ====================================================
        // LAN DISCOVERY
        // ====================================================

        Self::start_discovery_listener(
    address,
    discovery_port,
    Arc::clone(&peers),
);

Self::start_discovery_broadcaster(
    address,
    discovery_port,
    Arc::clone(&peers),
);

        // ====================================================
        // INTERNET BOOTSTRAP
        // ====================================================

        if let Some(
            bootstrap_address
        ) = bootstrap {

            println!(
                "Bootstrapping through {}...",
                bootstrap_address
            );

            match Self::bootstrap(
                bootstrap_address,
                address,
                Arc::clone(&peers),
                Arc::clone(&node),
            ) {

                Ok(()) => {

                    let count =
                        peers
                            .lock()
                            .map(
                                |manager|
                                    manager.len()
                            )
                            .unwrap_or(0);

                    println!(
                        "Bootstrap completed."
                    );

                    println!(
                        "Known peers: {}",
                        count
                    );
                }

                Err(error) => {

                    println!(
                        "Bootstrap failed: {}",
                        error
                    );
                }
            }
        }

        // ====================================================
        // TCP P2P SERVER
        // ====================================================

        for incoming in
            listener.incoming()
        {

            match incoming {

                Ok(mut stream) => {

                    let peer_address =
                        match stream.peer_addr() {

                            Ok(address) =>
                                address,

                            Err(error) => {

                                println!(
                                    "Cannot identify peer: {}",
                                    error
                                );

                                continue;
                            }
                        };

                    if peer_address
                        .ip()
                        .is_unspecified()
                    {
                        continue;
                    }

                    if let Ok(
                        mut manager
                    ) =
                        peers.lock()
                    {
                        manager.add(
                            peer_address
                        );
                    }

                    println!(
                        "Connection received from {}",
                        peer_address
                    );

                    let node_clone =
                        Arc::clone(&node);

                    let peers_clone =
                        Arc::clone(&peers);

                    thread::spawn(
                        move || {

                            Self::handle_connection(
                                &mut stream,
                                &node_clone,
                                &peers_clone,
                                peer_address,
                            );
                        }
                    );
                }

                Err(error) => {

                    println!(
                        "PEP Core connection error: {}",
                        error
                    );
                }
            }
        }
    }


    // ========================================================
    // ADVERTISED ADDRESS
    // ========================================================
    //
    // PEP_ADVERTISE is the address that other nodes should
    // actually use to connect to this node.
    //
    // Example:
    //
    // PEP_ADVERTISE=113.x.x.x:6000
    //
    // If absent, the listen address is used.
    //
    // IMPORTANT:
    //
    // 0.0.0.0 is a bind address, not a public peer address.
    //
    // ========================================================

    fn advertised_address(
    listen_address: &str,
) -> Result<SocketAddr, String> {

    // --------------------------------------------------------
    // Explicit advertised address
    // --------------------------------------------------------

    if let Ok(value) = env::var("PEP_ADVERTISE") {
        let value = value.trim();

        if !value.is_empty() {
            let address: SocketAddr =
                value.parse().map_err(|error| {
                    format!(
                        "Invalid PEP_ADVERTISE '{}': {}",
                        value,
                        error
                    )
                })?;

            if address.ip().is_unspecified() {
                return Err(format!(
                    "PEP_ADVERTISE cannot use unspecified address: {}",
                    address
                ));
            }

            if address.port() == 0 {
                return Err(
                    "PEP_ADVERTISE port cannot be 0."
                        .to_string()
                );
            }

            return Ok(address);
        }
    }

    // --------------------------------------------------------
    // Listen address
    // --------------------------------------------------------

    let listen: SocketAddr =
        listen_address.parse().map_err(|error| {
            format!(
                "Invalid listen address '{}': {}",
                listen_address,
                error
            )
        })?;

    // Already a usable advertised address.
    if !listen.ip().is_unspecified() {
        return Ok(listen);
    }

    // --------------------------------------------------------
    // 0.0.0.0 means "bind all interfaces".
    // Resolve the actual LAN IP automatically.
    // --------------------------------------------------------

    let local_ip =
        Self::local_ip_for(
            "1.1.1.1:80"
                .parse()
                .map_err(|error| {
                    format!(
                        "Cannot create route target: {}",
                        error
                    )
                })?,
        )?;

    Ok(SocketAddr::new(
        local_ip,
        listen.port(),
    ))
}


    // ========================================================
    // LAN DISCOVERY LISTENER
    // ========================================================

    fn start_discovery_listener(
    listen_address: &str,
    discovery_port: u16,
    peers: Arc<
        Mutex<PeerManager>
    >,
) {

        let listen_socket:
            SocketAddr =
            match listen_address.parse() {

                Ok(address) =>
                    address,

                Err(error) => {

                    println!(
                        "Invalid PEP listen address: {}",
                        error
                    );

                    return;
                }
            };

        let discovery_socket =
            match UdpSocket::bind(
                format!(
                    "0.0.0.0:{}",
                    discovery_port
                )
            ) {

                Ok(socket) =>
                    socket,

                Err(error) => {

                    println!(
                        "Cannot bind PEP discovery UDP {}: {}",
                        discovery_port,
                        error
                    );

                    return;
                }
            };

        println!(
            "PEP discovery listening on UDP {}",
            discovery_port
        );

        thread::spawn(
            move || {

                let mut buffer =
                    [0u8; 256];

                loop {

                    let (
                        size,
                        sender,
                    ) =
                        match discovery_socket
                            .recv_from(
                                &mut buffer
                            )
                        {

                            Ok(result) =>
                                result,

                            Err(error) => {

                                println!(
                                    "PEP discovery receive error: {}",
                                    error
                                );

                                continue;
                            }
                        };

                    let request =
                        String::from_utf8_lossy(
                            &buffer[..size]
                        )
                        .trim()
                        .to_string();

                    if request !=
                        "PEP_DISCOVER"
                    {
                        continue;
                    }

                    // ----------------------------------------
                    // Ignore our own LAN discovery broadcast.
                    // ----------------------------------------

                    let local_ip =
                        match Self::local_ip_for(
                            sender
                        ) {

                            Ok(ip) =>
                                ip,

                            Err(error) => {

                                println!(
                                    "Cannot determine local IP for discovery sender {}: {}",
                                    sender,
                                    error
                                );

                                continue;
                            }
                        };

                    if sender.ip() ==
                        local_ip
                    {
                        continue;
                    }

                    let advertised =
                        SocketAddr::new(
                            local_ip,
                            listen_socket.port()
                        );

                    // ----------------------------------------
                    // Remember the sender as a peer.
                    // ----------------------------------------

                    if advertised != sender {
                        if let Ok(mut manager) = peers.lock() {
                            manager.add(advertised);
                        }
                    }

                    let response =
                        format!(
                            "PEP_NODE|{}",
                            advertised
                        );

                    if let Err(error) =
                        discovery_socket.send_to(
                            response.as_bytes(),
                            sender
                        )
                    {

                        println!(
                            "Failed PEP discovery response to {}: {}",
                            sender,
                            error
                        );

                        continue;
                    }

                    println!(
                        "PEP discovery: {} -> {}",
                        sender,
                        advertised
                    );
                }
            }
        );
    }


    // ========================================================
    // LAN DISCOVERY BROADCASTER
    // ========================================================

    fn start_discovery_broadcaster(
    listen_address: &str,
    discovery_port: u16,
    peers: Arc<
        Mutex<PeerManager>
    >,
) {

        let listen_socket:
            SocketAddr =
            match listen_address.parse() {

                Ok(address) =>
                    address,

                Err(error) => {

                    println!(
                        "Invalid PEP listen address: {}",
                        error
                    );

                    return;
                }
            };

        thread::spawn(
            move || {

                let socket =
                    match UdpSocket::bind(
                        "0.0.0.0:0"
                    ) {

                        Ok(socket) =>
                            socket,

                        Err(error) => {

                            println!(
                                "Cannot create discovery socket: {}",
                                error
                            );

                            return;
                        }
                    };

                if let Err(error) =
                    socket.set_broadcast(
                        true
                    )
                {

                    println!(
                        "Cannot enable UDP broadcast: {}",
                        error
                    );

                    return;
                }

                loop {

                    let _ =
                        socket.send_to(
                            b"PEP_DISCOVER",
                            format!(
                                "255.255.255.255:{}",
                                discovery_port
                            ),
                        );

                    let _ =
                        socket.set_read_timeout(
                            Some(
                                Duration::from_secs(2)
                            )
                        );

                    let mut buffer =
                        [0u8; 256];

                    loop {

                        let received =
                            socket.recv_from(
                                &mut buffer
                            );

                        let (
                            size,
                            _
                        ) =
                            match received {

                                Ok(result) =>
                                    result,

                                Err(error)
                                    if error.kind()
                                        ==
                                        std::io::ErrorKind::WouldBlock
                                        ||
                                        error.kind()
                                        ==
                                        std::io::ErrorKind::TimedOut =>
                                {
                                    break;
                                }

                                Err(error) => {

                                    println!(
                                        "Discovery response error: {}",
                                        error
                                    );

                                    break;
                                }
                            };

                        let response =
                            String::from_utf8_lossy(
                                &buffer[..size]
                            )
                            .trim()
                            .to_string();

                        let address =
                            match response
                                .strip_prefix(
                                    "PEP_NODE|"
                                )
                            {

                                Some(value) =>
                                    value.trim(),

                                None =>
                                    continue,
                            };

                        let peer:
                            SocketAddr =
                            match address.parse() {

                                Ok(address) =>
                                    address,

                                Err(_) =>
                                    continue,
                            };

                        let local_ip =
                            match Self::local_ip_for(
                                peer
                            ) {

                                Ok(ip) =>
                                    ip,

                                Err(_) =>
                                    continue,
                            };

                        let local =
                            SocketAddr::new(
                                local_ip,
                                listen_socket.port()
                            );

                        if peer ==
                            local
                        {
                            continue;
                        }

                        let was_new =
                            match peers.lock() {

                                Ok(mut manager) =>
                                    manager.add(
                                        peer
                                    ),

                                Err(_) =>
                                    false,
                            };

                        if was_new {

                            println!(
                                "Discovered PEP peer {}",
                                peer
                            );

                            if Self::send_node_address(
                                peer,
                                local
                            ).is_ok()
                            {

                                if let Ok(
                                    mut manager
                                ) =
                                    peers.lock()
                                {
                                    manager.mark_success(
                                        peer
                                    );
                                }
                            }
                            else {

                                if let Ok(
                                    mut manager
                                ) =
                                    peers.lock()
                                {
                                    manager.mark_failure(
                                        peer
                                    );
                                }
                            }
                        }
                    }

                    thread::sleep(
                        Duration::from_secs(5)
                    );
                }
            }
        );
    }


    // ========================================================
    // BOOTSTRAP
    // ========================================================

    fn bootstrap(
        bootstrap_address: &str,
        listen_address: &str,
        peers: Arc<
            Mutex<PeerManager>
        >,
        node: Arc<
            Mutex<Node>
        >,
    ) -> Result<(), String> {

        if bootstrap_address
            .starts_with("http://")
            ||
            bootstrap_address
                .starts_with("https://")
        {

            return Self::bootstrap_http(
                bootstrap_address,
                listen_address,
                peers,
                node,
            );
        }

        // ----------------------------------------------------
        // Legacy TCP bootstrap
        // ----------------------------------------------------

        let bootstrap:
            SocketAddr =
            bootstrap_address
                .parse()
                .map_err(
                    |error|
                        format!(
                            "Invalid bootstrap address: {}",
                            error
                        )
                )?;

        let listen:
            SocketAddr =
            Self::advertised_address(
                listen_address
            )?;

        if bootstrap ==
            listen
        {
            return Ok(());
        }

        if let Ok(
            mut manager
        ) =
            peers.lock()
        {
            manager.add(
                bootstrap
            );
        }

        match Self::send_node_address(
            bootstrap,
            listen,
        ) {

            Ok(()) => {

                if let Ok(
                    mut manager
                ) =
                    peers.lock()
                {
                    manager.mark_success(
                        bootstrap
                    );
                }
            }

            Err(error) => {

                if let Ok(
                    mut manager
                ) =
                    peers.lock()
                {
                    manager.mark_failure(
                        bootstrap
                    );
                }

                println!(
                    "Bootstrap NodeAddress failed: {}",
                    error
                );
            }
        }

        let discovered =
            Self::request_peers(
                bootstrap
            )?;

        for peer in
            &discovered
        {

            if *peer ==
                listen
            {
                continue;
            }

            if let Ok(
                mut manager
            ) =
                peers.lock()
            {
                manager.add(
                    *peer
                );
            }
        }

        Self::sync_from_peer(
            bootstrap,
            &node
        )?;

        for peer in
            discovered
        {

            if peer ==
                listen
            {
                continue;
            }

            if let Err(error) =
                Self::sync_from_peer(
                    peer,
                    &node
                )
            {

                println!(
                    "Initial sync from {} failed: {}",
                    peer,
                    error
                );
            }
        }

        Ok(())
    }


    // ============================================================
// REGISTER WITH HTTP BOOTSTRAP
// ============================================================

fn register_with_bootstrap(
    bootstrap_url: &str,
    advertised_address: SocketAddr,
) -> Result<Vec<SocketAddr>, String> {

    let url =
        format!(
            "{}/register",
            bootstrap_url
                .trim_end_matches('/')
        );

    println!(
        "[PEP Bootstrap] Register URL: {}",
        url
    );

    // --------------------------------------------------------
    // HTTP client
    // --------------------------------------------------------

    let client =
        reqwest::blocking::Client::builder()
            .connect_timeout(
                Duration::from_secs(30)
            )
            .timeout(
                Duration::from_secs(90)
            )
            .build()
            .map_err(
                |error|
                    format!(
                        "Cannot create bootstrap HTTP client: {}",
                        error
                    )
            )?;

    let body =
        serde_json::json!({
            "address":
                advertised_address.to_string()
        });

    // --------------------------------------------------------
    // Retry
    //
    // Render Free có thể đang cold-start.
    // Đừng chết ngay ở request đầu tiên.
    // --------------------------------------------------------

    let max_attempts = 3;

    let mut last_error =
        String::new();

    for attempt in 1..=max_attempts {

        println!(
            "[PEP Bootstrap] Register attempt {}/{}...",
            attempt,
            max_attempts
        );

        let response =
            match client
                .post(&url)
                .header(
                    reqwest::header::CONTENT_TYPE,
                    "application/json"
                )
                .json(&body)
                .send()
            {

                Ok(response) =>
                    response,

                Err(error) => {

                    last_error =
                        format!(
                            "HTTP request error: {} | debug: {:?}",
                            error,
                            error
                        );

                    println!(
                        "[PEP Bootstrap] {}",
                        last_error
                    );

                    if attempt <
                        max_attempts
                    {
                        println!(
                            "[PEP Bootstrap] Retrying in 3 seconds..."
                        );

                        thread::sleep(
                            Duration::from_secs(3)
                        );
                    }

                    continue;
                }
            };

        let status =
            response.status();

        let response_body =
            match response.text()
            {

                Ok(body) =>
                    body,

                Err(error) => {

                    last_error =
                        format!(
                            "Failed reading bootstrap response: {}",
                            error
                        );

                    println!(
                        "[PEP Bootstrap] {}",
                        last_error
                    );

                    if attempt <
                        max_attempts
                    {
                        thread::sleep(
                            Duration::from_secs(3)
                        );
                    }

                    continue;
                }
            };

        println!(
            "[PEP Bootstrap] HTTP status: {}",
            status
        );

        println!(
            "[PEP Bootstrap] Response: {}",
            response_body
        );

        // ----------------------------------------------------
        // HTTP error
        // ----------------------------------------------------

        if !status.is_success() {

            last_error =
                format!(
                    "Bootstrap returned HTTP {}: {}",
                    status,
                    response_body
                );

            if attempt <
                max_attempts
            {
                println!(
                    "[PEP Bootstrap] Retrying in 3 seconds..."
                );

                thread::sleep(
                    Duration::from_secs(3)
                );

                continue;
            }

            return Err(
                last_error
            );
        }

        // ----------------------------------------------------
        // Parse JSON
        // ----------------------------------------------------

        let response:
            BootstrapPeersResponse =
            match serde_json::from_str(
                &response_body
            ) {

                Ok(response) =>
                    response,

                Err(error) => {

                    return Err(
                        format!(
                            "Invalid bootstrap register JSON: {} | body: {}",
                            error,
                            response_body
                        )
                    );
                }
            };

        // ----------------------------------------------------
        // Parse peer addresses
        // ----------------------------------------------------

        let mut peers = Vec::new();

for value in response.peers {
    match value.address.parse::<SocketAddr>() {
        Ok(peer) => {
            if peer != advertised_address
                && !peers.contains(&peer)
            {
                peers.push(peer);
            }
        }

        Err(error) => {
            println!(
                "[PEP Bootstrap] Ignoring invalid peer {}: {}",
                value.address,
                error
            );
        }
    }
}

        println!(
            "[PEP Bootstrap] Registration successful. Peers returned: {}",
            peers.len()
        );

        return Ok(
            peers
        );
    }

    Err(
        format!(
            "Bootstrap registration failed after {} attempts: {}",
            max_attempts,
            last_error
        )
    )
}


    // ========================================================
    // HTTP / HTTPS BOOTSTRAP
    // ========================================================

    fn bootstrap_http(
        bootstrap_url: &str,
        listen_address: &str,
        peers: Arc<
            Mutex<PeerManager>
        >,
        node: Arc<
            Mutex<Node>
        >,
    ) -> Result<(), String> {

        let advertised =
            Self::advertised_address(
                listen_address
            )?;

        println!(
            "PEP advertised address: {}",
            advertised
        );

        println!(
            "Registering node with bootstrap {}...",
            bootstrap_url
        );

        // ----------------------------------------------------
        // Register first.
        // The response also gives us the current peer list.
        // ----------------------------------------------------

        let discovered =
            Self::register_with_bootstrap(
                bootstrap_url,
                advertised,
            )?;

        println!(
            "Bootstrap returned {} peer(s).",
            discovered.len()
        );

        if let Ok(
            mut manager
        ) =
            peers.lock()
        {
            manager.add_many(
                discovered
                    .iter()
                    .copied()
            );
        }

        // ----------------------------------------------------
        // Try peers until one provides a valid chain.
        // ----------------------------------------------------

        let mut synchronized =
            false;

        for peer in
            discovered
        {

            if peer ==
                advertised
            {
                continue;
            }

            println!(
                "Trying PEP peer {}...",
                peer
            );

            match Self::connect_and_sync(
                peer,
                &node,
                &peers,
            ) {

                Ok(()) => {

                    println!(
                        "PEP sync from {} completed.",
                        peer
                    );

                    synchronized =
                        true;

                    break;
                }

                Err(error) => {

                    println!(
                        "Could not sync from {}: {}",
                        peer,
                        error
                    );
                }
            }
        }

        if !synchronized {

            let has_peers =
                peers
                    .lock()
                    .map(
                        |manager|
                            !manager.is_empty()
                    )
                    .unwrap_or(false);

            if has_peers {

                println!(
                    "Peers discovered, but no peer was available for initial sync."
                );
            }
            else {

                println!(
                    "Bootstrap registry is currently empty."
                );

                println!(
                    "This node will continue running and accepting P2P connections."
                );
            }
        }

        Ok(())
    }


    // ========================================================
    // CONNECT + SYNC
    // ========================================================

    fn connect_and_sync(
        peer: SocketAddr,
        node: &Arc<
            Mutex<Node>
        >,
        peers: &Arc<
            Mutex<PeerManager>
        >,
    ) -> Result<(), String> {

        Self::perform_handshake(
            peer,
            node,
        )?;

        match Self::sync_from_peer(
            peer,
            node,
        ) {

            Ok(()) => {

                if let Ok(
                    mut manager
                ) =
                    peers.lock()
                {
                    manager.mark_success(
                        peer
                    );
                }

                Ok(())
            }

            Err(error) => {

                if let Ok(
                    mut manager
                ) =
                    peers.lock()
                {
                    manager.mark_failure(
                        peer
                    );
                }

                Err(error)
            }
        }
    }


    // ========================================================
    // HANDSHAKE
    // ========================================================

    fn perform_handshake(
        peer: SocketAddr,
        node: &Arc<
            Mutex<Node>
        >,
    ) -> Result<(), String> {

        let (
            height,
            tip,
        ) =
            Self::local_chain_status(
                node
            )?;

        let payload =
            Message::hello_payload(
                PEP_NETWORK,
                PEP_PROTOCOL_VERSION,
                PEP_CHAIN_ID,
                height as u64,
                &tip,
            );

        let mut stream =
            TcpStream::connect_timeout(
                &peer,
                Duration::from_secs(5)
            )
            .map_err(
                |error|
                    format!(
                        "Handshake connection to {} failed: {}",
                        peer,
                        error
                    )
            )?;

        Message::Hello
            .write_to(
                &mut stream,
                &payload,
            )
            .map_err(
                |error|
                    format!(
                        "Failed to send HELLO to {}: {}",
                        peer,
                        error
                    )
            )?;

        let (
            message,
            payload,
        ) =
            Message::read_from(
                &mut stream
            )
            .map_err(
                |error|
                    format!(
                        "Failed to read HELLO_ACK from {}: {}",
                        peer,
                        error
                    )
            )?;

        if message !=
            Message::HelloAck
        {

            return Err(
                format!(
                    "Peer {} returned {:?} instead of HELLO_ACK",
                    peer,
                    message
                )
            );
        }

        Self::validate_hello_payload(
            &payload,
            peer,
        )?;

        Ok(())
    }


    // ========================================================
    // VALIDATE HELLO
    // ========================================================

    fn validate_hello_payload(
        payload: &[u8],
        peer: SocketAddr,
    ) -> Result<(), String> {

        let data =
            String::from_utf8_lossy(
                payload
            );

        let parts =
            data.trim()
                .split('|')
                .collect::<Vec<_>>();

        if parts.len() != 5 {

            return Err(
                format!(
                    "Invalid HELLO from {}",
                    peer
                )
            );
        }

        let network =
            parts[0];

        let protocol:
            u32 =
            parts[1]
                .parse()
                .map_err(
                    |_| {
                        format!(
                            "Invalid protocol version from {}",
                            peer
                        )
                    }
                )?;

        let chain_id:
            u64 =
            parts[2]
                .parse()
                .map_err(
                    |_| {
                        format!(
                            "Invalid chain ID from {}",
                            peer
                        )
                    }
                )?;

        let _height:
            u64 =
            parts[3]
                .parse()
                .map_err(
                    |_| {
                        format!(
                            "Invalid peer height from {}",
                            peer
                        )
                    }
                )?;

        let tip =
            parts[4];

        if network !=
            PEP_NETWORK
        {

            return Err(
                format!(
                    "Network mismatch with {}: {}",
                    peer,
                    network
                )
            );
        }

        if protocol !=
            PEP_PROTOCOL_VERSION
        {

            return Err(
                format!(
                    "Protocol mismatch with {}: {}",
                    peer,
                    protocol
                )
            );
        }

        if chain_id !=
            PEP_CHAIN_ID
        {

            return Err(
                format!(
                    "Chain ID mismatch with {}: {}",
                    peer,
                    chain_id
                )
            );
        }

        if tip.is_empty() {

            return Err(
                format!(
                    "Peer {} sent empty chain tip",
                    peer
                )
            );
        }

        Ok(())
    }


    // ========================================================
    // SEND NODE ADDRESS
    // ========================================================

    fn send_node_address(
        peer: SocketAddr,
        advertised: SocketAddr,
    ) -> Result<(), String> {

        let mut stream =
            TcpStream::connect_timeout(
                &peer,
                Duration::from_secs(3)
            )
            .map_err(
                |error|
                    format!(
                        "Cannot connect to {}: {}",
                        peer,
                        error
                    )
            )?;

        Message::NodeAddress
            .write_to(
                &mut stream,
                advertised
                    .to_string()
                    .as_bytes()
            )
            .map_err(
                |error|
                    format!(
                        "Failed to send NodeAddress: {}",
                        error
                    )
            )?;

        Ok(())
    }


    // ========================================================
    // REQUEST PEERS
    // ========================================================

    fn request_peers(
        peer: SocketAddr,
    ) -> Result<
        Vec<SocketAddr>,
        String,
    > {

        let mut stream =
            TcpStream::connect_timeout(
                &peer,
                Duration::from_secs(3)
            )
            .map_err(
                |error|
                    format!(
                        "Cannot connect to {}: {}",
                        peer,
                        error
                    )
            )?;

        Message::GetPeers
            .write_to(
                &mut stream,
                &[]
            )
            .map_err(
                |error|
                    format!(
                        "Failed to send GetPeers: {}",
                        error
                    )
            )?;

        let (
            message,
            payload,
        ) =
            Message::read_from(
                &mut stream
            )
            .map_err(
                |error|
                    format!(
                        "Failed to read Peers: {}",
                        error
                    )
            )?;

        if !matches!(
            message,
            Message::Peers
        ) {

            return Err(
                format!(
                    "Unexpected peer response: {:?}",
                    message
                )
            );
        }

        let data =
            String::from_utf8_lossy(
                &payload
            );

        let mut result =
            Vec::new();

        for value in
            data.split(',')
                .map(str::trim)
                .filter(
                    |value|
                        !value.is_empty()
                )
        {

            if let Ok(address) =
                value.parse::<SocketAddr>()
            {

                if !result
                    .contains(&address)
                {
                    result.push(
                        address
                    );
                }
            }
        }

        Ok(result)
    }


    // ========================================================
    // LOCAL CHAIN STATUS
    // ========================================================

    fn local_chain_status(
        node: &Arc<
            Mutex<Node>
        >,
    ) -> Result<
        (usize, String),
        String,
    > {

        let guard =
            node.lock()
                .map_err(
                    |_| {
                        "Node lock poisoned."
                            .to_string()
                    }
                )?;

        let height =
            guard
                .blockchain()
                .blocks
                .len()
                .saturating_sub(1);

        let tip =
            guard
                .blockchain()
                .blocks
                .last()
                .map(
                    |block|
                        block
                            .hash()
                            .to_string()
                )
                .unwrap_or_else(
                    || "0".to_string()
                );

        Ok(
            (
                height,
                tip,
            )
        )
    }


    // ========================================================
    // REQUEST STATUS
    // ========================================================

    fn request_status(
        peer: SocketAddr,
    ) -> Result<
        (usize, String),
        String,
    > {

        let mut stream =
            TcpStream::connect_timeout(
                &peer,
                Duration::from_secs(5)
            )
            .map_err(
                |error|
                    format!(
                        "Cannot connect to {}: {}",
                        peer,
                        error
                    )
            )?;

        Message::GetStatus
            .write_to(
                &mut stream,
                &[]
            )
            .map_err(
                |error|
                    format!(
                        "Failed to send GetStatus: {}",
                        error
                    )
            )?;

        let (
            message,
            payload,
        ) =
            Message::read_from(
                &mut stream
            )
            .map_err(
                |error|
                    format!(
                        "Failed to read Status: {}",
                        error
                    )
            )?;

        if !matches!(
            message,
            Message::Status
        ) {

            return Err(
                format!(
                    "Unexpected status response: {:?}",
                    message
                )
            );
        }

        let data =
            String::from_utf8_lossy(
                &payload
            );

        let parts =
            data.trim()
                .split('|')
                .collect::<Vec<_>>();

        if parts.len() != 2 {

            return Err(
                "Invalid Status payload."
                    .to_string()
            );
        }

        let height:
            usize =
            parts[0]
                .parse()
                .map_err(
                    |_| {
                        "Invalid remote height."
                            .to_string()
                    }
                )?;

        Ok(
            (
                height,
                parts[1].to_string()
            )
        )
    }


    // ========================================================
    // REQUEST BLOCKS
    // ========================================================

    fn request_blocks(
        peer: SocketAddr,
        start: usize,
        count: usize,
    ) -> Result<
        Vec<Block>,
        String,
    > {

        let mut stream =
            TcpStream::connect_timeout(
                &peer,
                Duration::from_secs(5)
            )
            .map_err(
                |error|
                    format!(
                        "Cannot connect to {}: {}",
                        peer,
                        error
                    )
            )?;

        let request =
            format!(
                "{}|{}",
                start,
                count.min(
                    MAX_BLOCK_BATCH
                )
            );

        Message::GetBlocks
            .write_to(
                &mut stream,
                request.as_bytes()
            )
            .map_err(
                |error|
                    format!(
                        "Failed to request blocks: {}",
                        error
                    )
            )?;

        let (
            message,
            payload,
        ) =
            Message::read_from(
                &mut stream
            )
            .map_err(
                |error|
                    format!(
                        "Failed to read Blocks: {}",
                        error
                    )
            )?;

        if !matches!(
            message,
            Message::Blocks
        ) {

            return Err(
                format!(
                    "Unexpected block response: {:?}",
                    message
                )
            );
        }

        let data =
            String::from_utf8_lossy(
                &payload
            );

        let mut blocks =
            Vec::new();

        for line in
            data.lines()
        {

            let line =
                line.trim();

            if line.is_empty() {
                continue;
            }

            blocks.push(
                Block::deserialize(
                    line
                )?
            );
        }

        Ok(blocks)
    }


    // ========================================================
    // SYNC FROM PEER
    // ========================================================

    fn sync_from_peer(
        peer: SocketAddr,
        node: &Arc<
            Mutex<Node>
        >,
    ) -> Result<(), String> {

        let local_height =
            Self::local_chain_status(
                node
            )?
            .0;

        let (
            remote_height,
            remote_tip,
        ) =
            Self::request_status(
                peer
            )?;

        println!(
            "Sync check {}: local={} remote={} tip={}",
            peer,
            local_height,
            remote_height,
            remote_tip
        );

        if remote_height <=
            local_height
        {
            return Ok(());
        }

        let mut next =
            local_height
                .saturating_add(1);

        let mut downloaded =
            Vec::<Block>::new();

        while next <=
            remote_height
        {

            let count =
                remote_height
                    .saturating_sub(next)
                    .saturating_add(1)
                    .min(
                        MAX_BLOCK_BATCH
                    );

            let blocks =
                Self::request_blocks(
                    peer,
                    next,
                    count
                )?;

            if blocks.is_empty() {

                return Err(
                    format!(
                        "Peer {} returned no blocks at height {}",
                        peer,
                        next
                    )
                );
            }

            next =
                next.saturating_add(
                    blocks.len()
                );

            downloaded.extend(
                blocks
            );
        }

        let mut guard =
            node.lock()
                .map_err(
                    |_| {
                        "Node lock poisoned."
                            .to_string()
                    }
                )?;

        let current_height =
            guard
                .blockchain()
                .blocks
                .len()
                .saturating_sub(1);

        if current_height >
            local_height
        {
            return Ok(());
        }

        for block in
            downloaded
        {

            guard
                .accept_block(
                    block
                )
                .map_err(
                    |error|
                        format!(
                            "Rejected block from {}: {}",
                            peer,
                            error
                        )
                )?;
        }

        println!(
            "Synchronized from {}: {} -> {}",
            peer,
            local_height,
            guard
                .blockchain()
                .blocks
                .len()
                .saturating_sub(1)
        );

        Ok(())
    }


    // ========================================================
    // LOCAL IP
    // ========================================================

    fn local_ip_for(
        remote: SocketAddr,
    ) -> Result<
        std::net::IpAddr,
        String,
    > {

        let socket =
            UdpSocket::bind(
                "0.0.0.0:0"
            )
            .map_err(
                |error|
                    format!(
                        "Cannot create route socket: {}",
                        error
                    )
            )?;

        socket
            .connect(remote)
            .map_err(
                |error|
                    format!(
                        "Cannot determine local route: {}",
                        error
                    )
            )?;

        socket
            .local_addr()
            .map(
                |address|
                    address.ip()
            )
            .map_err(
                |error|
                    format!(
                        "Cannot read local address: {}",
                        error
                    )
            )
    }


    // ========================================================
    // PARSE BOOL
    // ========================================================

    fn parse_bool(
        value: &str,
        field: &str,
    ) -> Result<
        bool,
        String,
    > {

        match value
            .trim()
            .to_ascii_lowercase()
            .as_str()
        {

            "true" |
            "1" |
            "yes" =>
                Ok(true),

            "false" |
            "0" |
            "no" =>
                Ok(false),

            _ =>
                Err(
                    format!(
                        "Invalid {} value: {}",
                        field,
                        value
                    )
                ),
        }
    }


    // ========================================================
    // PARSE ASSET
    // ========================================================

    fn parse_asset(
        payload: &[u8],
    ) -> Result<
        AssetDefinition,
        String,
    > {

        let data =
            String::from_utf8(
                payload.to_vec()
            )
            .map_err(
                |_| {
                    "Asset payload is not valid UTF-8"
                        .to_string()
                }
            )?;

        let fields =
            data.trim()
                .split('|')
                .collect::<Vec<_>>();

        if fields.len() != 8 {

            return Err(
                format!(
                    "Invalid asset payload: expected 8 fields, got {}",
                    fields.len()
                )
            );
        }

        let id =
            fields[0].trim();

        if id.is_empty() {

            return Err(
                "Asset ID cannot be empty."
                    .to_string()
            );
        }

        let asset_id =
            AssetId::new(
                id
            );

        let asset_type =
            match fields[1]
                .trim()
                .to_ascii_lowercase()
                .as_str()
            {

                "index" =>
                    AssetType::Index,

                "pegged" =>
                    AssetType::Pegged,

                "ccq" =>
                    AssetType::Ccq,

                "native" =>
                    AssetType::Native,

                other =>
                    return Err(
                        format!(
                            "Unknown asset type: {}",
                            other
                        )
                    ),
            };

        let decimals:
            u8 =
            fields[2]
                .trim()
                .parse()
                .map_err(
                    |_| {
                        "Invalid decimals."
                            .to_string()
                    }
                )?;

        let supply:
            u64 =
            fields[3]
                .trim()
                .parse()
                .map_err(
                    |_| {
                        "Invalid supply."
                            .to_string()
                    }
                )?;

        let deploy_address =
            if fields[4]
                .trim()
                .is_empty()
            {
                None
            } else {

                Some(
                    fields[4]
                        .trim()
                        .to_string()
                )
            };

        let transferable =
            Self::parse_bool(
                fields[5],
                "TRS"
            )?;

        let gas_eligible =
            Self::parse_bool(
                fields[6],
                "GAS"
            )?;

        let peg =
            match fields[7]
                .trim()
                .to_ascii_uppercase()
                .as_str()
            {

                "" =>
                    None,

                "BTC" =>
                    Some("BTC"),

                "ETH" =>
                    Some("ETH"),

                "USDT" =>
                    Some("USDT"),

                "USDC" =>
                    Some("USDC"),

                other =>
                    return Err(
                        format!(
                            "Unsupported PEG: {}",
                            other
                        )
                    ),
            };

        Ok(
            AssetDefinition {

                id:
                    asset_id,

                asset_type,

                decimals,

                supply,

                deploy_address,

                transferable,

                gas_eligible,

                peg,
            }
        )
    }


    // ========================================================
    // HANDLE CONNECTION
    // ========================================================

    fn handle_connection(
        stream: &mut TcpStream,
        node: &Arc<
            Mutex<Node>
        >,
        peers: &Arc<
            Mutex<PeerManager>
        >,
        peer_address: SocketAddr,
    ) {

        let (
            message,
            payload,
        ) =
            match Message::read_from(
                stream
            ) {

                Ok(result) =>
                    result,

                Err(error) => {

                    println!(
                        "Failed to read message from {}: {}",
                        peer_address,
                        error
                    );

                    if let Ok(
                        mut manager
                    ) =
                        peers.lock()
                    {
                        manager.mark_failure(
                            peer_address
                        );
                    }

                    return;
                }
            };

        match message {

            // =================================================
            // HELLO
            // =================================================

            Message::Hello => {

                match Self::validate_hello_payload(
                    &payload,
                    peer_address,
                ) {

                    Ok(()) => {

                        let (
                            height,
                            tip,
                        ) =
                            match Self::local_chain_status(
                                node
                            ) {

                                Ok(status) =>
                                    status,

                                Err(error) => {

                                    println!(
                                        "Cannot read local chain status: {}",
                                        error
                                    );

                                    return;
                                }
                            };

                        let response =
                            Message::hello_ack_payload(
                                PEP_NETWORK,
                                PEP_PROTOCOL_VERSION,
                                PEP_CHAIN_ID,
                                height as u64,
                                &tip,
                            );

                        if let Err(error) =
                            Message::HelloAck
                                .write_to(
                                    stream,
                                    &response,
                                )
                        {

                            println!(
                                "Failed HELLO_ACK to {}: {}",
                                peer_address,
                                error
                            );

                            if let Ok(
                                mut manager
                            ) =
                                peers.lock()
                            {
                                manager.mark_failure(
                                    peer_address
                                );
                            }

                            return;
                        }

                        if let Ok(
                            mut manager
                        ) =
                            peers.lock()
                        {
                            manager.mark_success(
                                peer_address
                            );
                        }

                        println!(
                            "HELLO accepted from {}",
                            peer_address
                        );
                    }

                    Err(error) => {

                        println!(
                            "HELLO rejected from {}: {}",
                            peer_address,
                            error
                        );
                    }
                }
            }


            // =================================================
            // HELLO ACK
            // =================================================

            Message::HelloAck => {

                println!(
                    "Received unexpected HELLO_ACK from {}",
                    peer_address
                );
            }


            // =================================================
            // PING
            // =================================================

            Message::Ping => {

                if let Err(error) =
                    Message::Pong
                        .write_to(
                            stream,
                            &[]
                        )
                {

                    println!(
                        "Failed PONG to {}: {}",
                        peer_address,
                        error
                    );
                }
            }


            // =================================================
            // PONG
            // =================================================

            Message::Pong => {

                println!(
                    "Received PONG from {}",
                    peer_address
                );
            }


            // =================================================
            // GET PEERS
            // =================================================

            Message::GetPeers => {

                let list =
                    peers
                        .lock()
                        .map(
                            |manager|
                                manager
                                    .all()
                                    .iter()
                                    .map(
                                        |address|
                                            address.to_string()
                                    )
                                    .collect::<Vec<_>>()
                                    .join(",")
                        )
                        .unwrap_or_default();

                if let Err(error) =
                    Message::Peers
                        .write_to(
                            stream,
                            list.as_bytes()
                        )
                {

                    println!(
                        "Failed PEERS to {}: {}",
                        peer_address,
                        error
                    );
                }
            }


            // =================================================
            // PEERS
            // =================================================

            Message::Peers => {

                println!(
                    "Received unexpected PEERS from {}",
                    peer_address
                );
            }


            // =================================================
            // NODE ADDRESS
            // =================================================

            Message::NodeAddress => {

                let advertised =
                    String::from_utf8_lossy(
                        &payload
                    )
                    .trim()
                    .to_string();

                let address:
                    SocketAddr =
                    match advertised.parse() {

                        Ok(address) =>
                            address,

                        Err(error) => {

                            println!(
                                "Invalid NodeAddress from {}: {}",
                                peer_address,
                                error
                            );

                            return;
                        }
                    };

                if address
                    .ip()
                    .is_unspecified()
                {
                    return;
                }

                if let Ok(
                    mut manager
                ) =
                    peers.lock()
                {

                    manager.add(
                        address
                    );

                    manager.mark_success(
                        peer_address
                    );
                }

                println!(
                    "Node {} advertises {}",
                    peer_address,
                    address
                );

                let node_clone =
                    Arc::clone(
                        node
                    );

                let peers_clone =
                    Arc::clone(
                        peers
                    );

                thread::spawn(
                    move || {

                        if let Err(error) =
                            Self::connect_and_sync(
                                address,
                                &node_clone,
                                &peers_clone,
                            )
                        {

                            println!(
                                "Sync from {} failed: {}",
                                address,
                                error
                            );
                        }
                    }
                );
            }


            // =================================================
            // GET STATUS
            // =================================================

            Message::GetStatus => {

                let (
                    height,
                    tip,
                ) =
                    match Self::local_chain_status(
                        node
                    ) {

                        Ok(status) =>
                            status,

                        Err(error) => {

                            println!(
                                "Cannot get local status: {}",
                                error
                            );

                            return;
                        }
                    };

                let response =
                    format!(
                        "{}|{}",
                        height,
                        tip
                    );

                if let Err(error) =
                    Message::Status
                        .write_to(
                            stream,
                            response.as_bytes()
                        )
                {

                    println!(
                        "Failed Status to {}: {}",
                        peer_address,
                        error
                    );
                }
            }


            // =================================================
            // STATUS
            // =================================================

            Message::Status => {

                println!(
                    "Received unexpected Status from {}",
                    peer_address
                );
            }


            // =================================================
            // GET BLOCKS
            // =================================================

            Message::GetBlocks => {

                let request =
                    String::from_utf8_lossy(
                        &payload
                    );

                let parts =
                    request
                        .trim()
                        .split('|')
                        .collect::<Vec<_>>();

                if parts.len() != 2 {

                    println!(
                        "Invalid GetBlocks request from {}",
                        peer_address
                    );

                    return;
                }

                let start:
                    usize =
                    match parts[0]
                        .parse::<usize>()
                    {

                        Ok(value) =>
                            value,

                        Err(_) => {

                            println!(
                                "Invalid GetBlocks start from {}",
                                peer_address
                            );

                            return;
                        }
                    };

                let count:
                    usize =
                    match parts[1]
                        .parse::<usize>()
                    {

                        Ok(value) =>
                            value.min(
                                MAX_BLOCK_BATCH
                            ),

                        Err(_) => {

                            println!(
                                "Invalid GetBlocks count from {}",
                                peer_address
                            );

                            return;
                        }
                    };

                let blocks =
                    match node.lock() {

                        Ok(guard) => {

                            guard
                                .blockchain()
                                .blocks
                                .iter()
                                .skip(start)
                                .take(count)
                                .map(
                                    |block|
                                        block.serialize()
                                )
                                .collect::<Vec<_>>()
                                .join("\n")
                        }

                        Err(_) => {

                            println!(
                                "Node lock poisoned."
                            );

                            return;
                        }
                    };

                if let Err(error) =
                    Message::Blocks
                        .write_to(
                            stream,
                            blocks.as_bytes()
                        )
                {

                    println!(
                        "Failed Blocks response to {}: {}",
                        peer_address,
                        error
                    );
                }
            }


            // =================================================
            // BLOCKS
            // =================================================

            Message::Blocks => {

                println!(
                    "Received unexpected Blocks from {}",
                    peer_address
                );
            }


            // =================================================
            // TRANSACTION
            // =================================================

            Message::Transaction => {

                let data =
                    String::from_utf8_lossy(
                        &payload
                    );

                let tx =
                    match crate::blockchain::transaction::Transaction::deserialize(
                        &data
                    ) {

                        Some(tx) =>
                            tx,

                        None => {

                            println!(
                                "Invalid transaction from {}",
                                peer_address
                            );

                            return;
                        }
                    };

                let result =
                    match node.lock() {

                        Ok(mut guard) => {

                            guard.on_transaction(
                                tx
                            )
                        }

                        Err(_) => {

                            println!(
                                "Node lock poisoned."
                            );

                            return;
                        }
                    };

                if let Err(error) =
                    result
                {

                    println!(
                        "Transaction processing failed from {}: {:?}",
                        peer_address,
                        error
                    );
                }
            }


            // =================================================
            // BALANCE REQUEST
            // =================================================

            Message::BalanceRequest => {

                let address_string =
                    String::from_utf8_lossy(
                        &payload
                    )
                    .trim()
                    .to_string();

                let address =
                    Address::new(
                        address_string
                    );

                let (
                    balances,
                    nonce,
                    stake,
                ) =
                    match node.lock() {

                        Ok(guard) =>
                            guard.get_balance(
                                &address
                            ),

                        Err(_) => {

                            println!(
                                "Node lock poisoned."
                            );

                            return;
                        }
                    };

                let balance_string =
                    balances
                        .iter()
                        .map(
                            |(asset, amount)|
                                format!(
                                    "{}={}",
                                    asset,
                                    amount
                                )
                        )
                        .collect::<Vec<_>>()
                        .join(";");

                let response =
                    format!(
                        "{}|{}|{}",
                        balance_string,
                        nonce,
                        stake
                    );

                if let Err(error) =
                    Message::BalanceResponse
                        .write_to(
                            stream,
                            response.as_bytes()
                        )
                {

                    println!(
                        "Failed BalanceResponse to {}: {}",
                        peer_address,
                        error
                    );
                }
            }


            // =================================================
            // BALANCE RESPONSE
            // =================================================

            Message::BalanceResponse => {

                println!(
                    "Received unexpected BalanceResponse from {}",
                    peer_address
                );
            }


            // =================================================
            // REGISTER ASSET
            // =================================================

            Message::RegisterAssetRequest => {

                let asset =
                    match Self::parse_asset(
                        &payload
                    ) {

                        Ok(asset) =>
                            asset,

                        Err(error) => {

                            let response =
                                format!(
                                    "ERROR|{}",
                                    error
                                );

                            let _ =
                                Message::RegisterAssetResponse
                                    .write_to(
                                        stream,
                                        response.as_bytes()
                                    );

                            return;
                        }
                    };

                match AssetRegistry::register(
                    asset.clone()
                ) {

                    Ok(()) => {

                        let response =
                            format!(
                                "OK|{}",
                                asset.id
                            );

                        let _ =
                            Message::RegisterAssetResponse
                                .write_to(
                                    stream,
                                    response.as_bytes()
                                );
                    }

                    Err(error) => {

                        let response =
                            format!(
                                "ERROR|{}",
                                error
                            );

                        let _ =
                            Message::RegisterAssetResponse
                                .write_to(
                                    stream,
                                    response.as_bytes()
                                );
                    }
                }
            }


            // =================================================
            // REGISTER ASSET RESPONSE
            // =================================================

            Message::RegisterAssetResponse => {

                println!(
                    "Received RegisterAssetResponse from {}",
                    peer_address
                );
            }


            // =================================================
            // RESERVED / FUTURE
            // =================================================

            Message::GetHeaders |
            Message::Headers |
            Message::GetBlock |
            Message::Block |
            Message::NewBlock |
            Message::NewTransaction => {

                println!(
                    "Received reserved P2P message {:?} from {}",
                    message,
                    peer_address
                );
            }
        }
    }
}