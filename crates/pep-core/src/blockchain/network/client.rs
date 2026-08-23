use std::net::{
    TcpStream,
    UdpSocket,
};
use std::time::Duration;

use crate::blockchain::network::message::Message;
use crate::blockchain::transaction::Transaction;

pub struct Client;

impl Client {

    // =========================
    // PEP Node Discovery
    // =========================

    pub fn discover_node() -> Option<String> {

        let discovery_port =
            6001;

        let discovery_address =
            format!(
                "255.255.255.255:{}",
                discovery_port,
            );

        let socket =
            match UdpSocket::bind(
                "0.0.0.0:0",
            ) {

                Ok(socket) =>
                    socket,

                Err(e) => {

                    println!(
                        "Failed to create discovery socket: {}",
                        e,
                    );

                    return None;
                }
            };

        if let Err(e) =
            socket.set_broadcast(true)
        {
            println!(
                "Failed to enable UDP broadcast: {}",
                e,
            );

            return None;
        }

        if let Err(e) =
            socket.set_read_timeout(
                Some(
                    Duration::from_millis(
                        1500,
                    )
                )
            )
        {
            println!(
                "Failed to configure discovery timeout: {}",
                e,
            );

            return None;
        }

        let request =
            b"PEP_DISCOVER";

        println!(
            "Searching for PEP Network..."
        );

        if let Err(e) =
            socket.send_to(
                request,
                &discovery_address,
            )
        {
            println!(
                "Failed to broadcast PEP discovery: {}",
                e,
            );

            return None;
        }

        let mut buffer =
            [0u8; 256];

        loop {

            let (
                size,
                sender,
            ) =
                match socket.recv_from(
                    &mut buffer,
                ) {

                    Ok(result) =>
                        result,

                    Err(e) => {

                        if e.kind()
                            == std::io::ErrorKind::WouldBlock
                            || e.kind()
                                == std::io::ErrorKind::TimedOut
                        {
                            break;
                        }

                        println!(
                            "PEP discovery receive error: {}",
                            e,
                        );

                        break;
                    }
                };

            let response =
                String::from_utf8_lossy(
                    &buffer[..size],
                )
                .trim()
                .to_string();

            if !response.starts_with(
                "PEP_NODE|"
            ) {
                continue;
            }

            let node_address =
                response
                    .strip_prefix(
                        "PEP_NODE|"
                    )
                    .unwrap_or("")
                    .trim();

            if node_address.is_empty() {
                continue;
            }

            println!(
                "PEP Node discovered: {}",
                node_address,
            );

            let address:
                std::net::SocketAddr =
                match node_address.parse() {

                    Ok(address) =>
                        address,

                    Err(e) => {

                        println!(
                            "Invalid discovered node address {}: {}",
                            node_address,
                            e,
                        );

                        continue;
                    }
                };

            // ====================================================
            // REAL TCP CONNECTION CHECK
            //
            // Không chỉ connect TCP rồi đóng socket.
            //
            // Wallet phải gửi PING.
            // Core phải trả PONG.
            // Chỉ khi nhận đúng PONG mới coi node là connected.
            // ====================================================

            let mut stream =
                match TcpStream::connect_timeout(
                    &address,
                    Duration::from_secs(2),
                ) {

                    Ok(stream) =>
                        stream,

                    Err(e) => {

                        println!(
                            "Discovered node {} is unreachable: {}",
                            node_address,
                            e,
                        );

                        continue;
                    }
                };

            if let Err(e) =
                stream.set_read_timeout(
                    Some(
                        Duration::from_secs(2)
                    )
                )
            {
                println!(
                    "Failed to configure PEP node read timeout {}: {}",
                    node_address,
                    e,
                );

                continue;
            }

            if let Err(e) =
                stream.set_write_timeout(
                    Some(
                        Duration::from_secs(2)
                    )
                )
            {
                println!(
                    "Failed to configure PEP node write timeout {}: {}",
                    node_address,
                    e,
                );

                continue;
            }

            println!(
                "TCP connection established with {}",
                node_address,
            );

            // =========================
            // PING
            // =========================

            if let Err(e) =
                Message::Ping
                    .write_to(
                        &mut stream,
                        &[],
                    )
            {
                println!(
                    "Failed to send PING to {}: {}",
                    node_address,
                    e,
                );

                continue;
            }

            println!(
                "PING sent to {}",
                node_address,
            );

            // =========================
            // PONG
            // =========================

            let (
                message,
                _payload,
            ) =
                match Message::read_from(
                    &mut stream,
                ) {

                    Ok(result) =>
                        result,

                    Err(e) => {

                        println!(
                            "Failed to receive PONG from {}: {}",
                            node_address,
                            e,
                        );

                        continue;
                    }
                };

            if !matches!(
                message,
                Message::Pong
            ) {

                println!(
                    "Unexpected response from {}: {:?}",
                    node_address,
                    message,
                );

                continue;
            }

            println!(
                "PONG received from {}",
                node_address,
            );

            println!(
                "✓ PEP Node handshake successful: {}",
                node_address,
            );

            println!(
                "✓ Connected to PEP Network"
            );

            // sender chỉ để debug.
            let _ =
                sender;

            return Some(
                node_address.to_string()
            );
        }

        println!(
            "No PEP Node found."
        );

        None
    }


    // =========================
    // Send Transaction
    // =========================

    pub fn send_transaction(
        node_address: &str,
        tx: &Transaction,
    ) {

        match TcpStream::connect(
            node_address,
        ) {

            Ok(mut stream) => {

                println!(
                    "Connected to {}",
                    node_address,
                );

                let data =
                    tx.serialize();

                if let Err(e) =
                    Message::Transaction.write_to(
                        &mut stream,
                        data.as_bytes(),
                    )
                {
                    println!(
                        "Failed to send transaction: {}",
                        e,
                    );

                    return;
                }

                println!(
                    "Transaction sent."
                );

                println!(
                    "{}",
                    data
                );
            }

            Err(e) => {

                println!(
                    "Connection failed to {}: {}",
                    node_address,
                    e,
                );
            }
        }
    }
        // =========================
    // Register Asset
    // =========================

    pub fn register_asset(
        node_address: &str,
        asset_type: &str,
        decimals: u8,
        supply: u64,
        deploy_address: &str,
        transferable: bool,
        gas_eligible: bool,
        peg: &str,
    ) -> Option<String> {

        let mut stream =
            match TcpStream::connect(
                node_address,
            ) {

                Ok(stream) => stream,

                Err(e) => {

                    println!(
                        "Connection failed to {}: {}",
                        node_address,
                        e,
                    );

                    return None;
                }
            };

        let payload =
            format!(
                "{}|{}|{}|{}|{}|{}|{}",
                asset_type,
                decimals,
                supply,
                deploy_address,
                transferable,
                gas_eligible,
                peg,
            );

        if let Err(e) =
            Message::RegisterAssetRequest.write_to(
                &mut stream,
                payload.as_bytes(),
            )
        {
            println!(
                "Failed to send asset registration: {}",
                e,
            );

            return None;
        }

        let (
            message,
            response,
        ) =
            match Message::read_from(
                &mut stream,
            ) {

                Ok(result) => result,

                Err(e) => {

                    println!(
                        "Failed to read asset registration response: {}",
                        e,
                    );

                    return None;
                }
            };

        if !matches!(
            message,
            Message::RegisterAssetResponse
        ) {

            println!(
                "Unexpected asset registration response: {:?}",
                message,
            );

            return None;
        }

        Some(
            String::from_utf8_lossy(
                &response,
            )
            .trim()
            .to_string()
        )
    }

    // =========================
// Get Balance
// =========================

pub fn get_balance(
    node_address: &str,
    address: &crate::wallet::Address,
) -> Option<(Vec<(String, u64)>, u64, u64)> {

    println!(
        "Connecting to PEP Node {} for balance...",
        node_address
    );

    let mut stream =
        match TcpStream::connect(
            node_address,
        ) {

            Ok(stream) => {

                println!(
                    "Balance connection established with {}",
                    node_address
                );

                stream
            }

            Err(e) => {

                println!(
                    "Connection failed to {}: {}",
                    node_address,
                    e,
                );

                return None;
            }
        };


    // =========================
    // Send Balance Request
    // =========================

    if let Err(e) =
        Message::BalanceRequest.write_to(
            &mut stream,
            address.as_str().as_bytes(),
        )
    {
        println!(
            "Failed to send balance request to {}: {}",
            node_address,
            e,
        );

        return None;
    }

    println!(
        "Balance request sent for {}",
        address.as_str()
    );


    // =========================
    // Read Balance Response
    // =========================

    let (
        message,
        payload,
    ) =
        match Message::read_from(
            &mut stream,
        ) {

            Ok(result) => result,

            Err(e) => {

                println!(
                    "Failed to read balance response from {}: {}",
                    node_address,
                    e,
                );

                return None;
            }
        };


    // =========================
    // Verify Message Type
    // =========================

    if !matches!(
        message,
        Message::BalanceResponse
    ) {

        println!(
            "Unexpected balance response from {}: {:?}",
            node_address,
            message,
        );

        return None;
    }


    // =========================
    // Decode Response
    //
    // Format:
    //
    // asset=amount;asset=amount|nonce|stake
    //
    // Example:
    //
    // PEP=1000;USDT=500|3|0
    //
    // Empty portfolio:
    //
    // |0|0
    // =========================

    let response =
        String::from_utf8_lossy(
            &payload,
        )
        .trim()
        .to_string();


    println!(
        "Balance response: {}",
        response
    );


    let parts:
        Vec<&str> =
        response
            .split('|')
            .collect();


    if parts.len() != 3 {

        println!(
            "Invalid balance response format: {}",
            response
        );

        return None;
    }


    // =========================
    // Parse Portfolio
    // =========================

    let mut balances:
        Vec<(String, u64)> =
        Vec::new();


    if !parts[0]
        .trim()
        .is_empty()
    {

        for entry in
            parts[0]
                .split(';')
        {

            let entry =
                entry.trim();


            if entry.is_empty() {
                continue;
            }


            let mut pair =
                entry.splitn(
                    2,
                    '='
                );


            let asset =
                match pair.next() {

                    Some(value) =>
                        value.trim(),

                    None =>
                        continue,
                };


            let amount_string =
                match pair.next() {

                    Some(value) =>
                        value.trim(),

                    None => {

                        println!(
                            "Invalid asset balance entry: {}",
                            entry
                        );

                        return None;
                    }
                };


            let amount:
                u64 =
                match amount_string.parse() {

                    Ok(value) =>
                        value,

                    Err(e) => {

                        println!(
                            "Invalid balance amount '{}': {}",
                            amount_string,
                            e
                        );

                        return None;
                    }
                };


            balances.push(
                (
                    asset.to_string(),
                    amount,
                )
            );
        }
    }


    // =========================
    // Parse Nonce
    // =========================

    let nonce:
        u64 =
        match parts[1]
            .trim()
            .parse()
        {

            Ok(value) =>
                value,

            Err(e) => {

                println!(
                    "Invalid nonce '{}': {}",
                    parts[1],
                    e
                );

                return None;
            }
        };


    // =========================
    // Parse Stake
    // =========================

    let stake:
        u64 =
        match parts[2]
            .trim()
            .parse()
        {

            Ok(value) =>
                value,

            Err(e) => {

                println!(
                    "Invalid stake '{}': {}",
                    parts[2],
                    e
                );

                return None;
            }
        };


    println!(
        "Balance received successfully: {} asset(s), nonce={}, stake={}",
        balances.len(),
        nonce,
        stake
    );


    Some(
        (
            balances,
            nonce,
            stake,
        )
    )
}


    // =========================
    // Get Peers
    // =========================

    pub fn get_peers(
        node_address: &str,
    ) -> Vec<String> {

        let mut stream =
            match TcpStream::connect(
                node_address,
            ) {

                Ok(stream) => stream,

                Err(e) => {

                    println!(
                        "Connection failed to {}: {}",
                        node_address,
                        e,
                    );

                    return Vec::new();
                }
            };

        if let Err(e) =
            Message::GetPeers.write_to(
                &mut stream,
                &[],
            )
        {
            println!(
                "Failed to request peers from {}: {}",
                node_address,
                e,
            );

            return Vec::new();
        }

        let (
            message,
            payload,
        ) =
            match Message::read_from(
                &mut stream,
            ) {

                Ok(result) => result,

                Err(e) => {

                    println!(
                        "Failed to read peer list from {}: {}",
                        node_address,
                        e,
                    );

                    return Vec::new();
                }
            };

        if !matches!(
            message,
            Message::Peers
        ) {
            println!(
                "Unexpected peer response from {}: {:?}",
                node_address,
                message,
            );

            return Vec::new();
        }

        let data =
            String::from_utf8_lossy(
                &payload,
            );

        data
            .split(',')
            .map(str::trim)
            .filter(|address| {
                !address.is_empty()
            })
            .map(String::from)
            .collect()
    }
}