use std::io::{self, Read, Write};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Message {
    Ping = 1,
    Pong = 2,

    Transaction = 3,

    BalanceRequest = 4,
    BalanceResponse = 5,

    GetPeers = 6,
    Peers = 7,

    NodeAddress = 8,

    RegisterAssetRequest = 9,
    RegisterAssetResponse = 10,

    /*
     * ========================================================
     * CHAIN SYNCHRONIZATION
     * ========================================================
     */

    GetStatus = 11,
    Status = 12,

    GetBlocks = 13,
    Blocks = 14,

    /*
     * ========================================================
     * P2P HANDSHAKE
     * ========================================================
     *
     * Hello
     *     Node introduces itself.
     *
     * HelloAck
     *     Peer accepts the connection.
     *
     * Payload:
     *
     *     network|protocol|chain_id|height|tip
     *
     * Example:
     *
     *     mainnet|1|1|123|abcdef...
     *
     * ========================================================
     */

    Hello = 15,
    HelloAck = 16,

    /*
     * ========================================================
     * FUTURE HEADER SYNCHRONIZATION
     * ========================================================
     *
     * These messages are reserved now so the protocol can
     * evolve without renumbering existing messages.
     *
     * ========================================================
     */

    GetHeaders = 17,
    Headers = 18,

    /*
     * ========================================================
     * FUTURE SINGLE BLOCK PROPAGATION
     * ========================================================
     */

    GetBlock = 19,
    Block = 20,

    /*
     * ========================================================
     * FUTURE REAL-TIME PROPAGATION
     * ========================================================
     */

    NewBlock = 21,
    NewTransaction = 22,
}


impl Message {

    /*
     * ========================================================
     * CONSTANTS
     * ========================================================
     */

    pub const MAX_PAYLOAD_SIZE:
        usize = 64 * 1024 * 1024;


    /*
     * ========================================================
     * MESSAGE TYPE → BYTE
     * ========================================================
     */

    pub fn to_bytes(
        &self,
    ) -> [u8; 1] {

        [*self as u8]
    }


    /*
     * ========================================================
     * BYTE → MESSAGE TYPE
     * ========================================================
     */

    pub fn from_byte(
        byte: u8,
    ) -> Option<Self> {

        match byte {

            1 =>
                Some(Self::Ping),

            2 =>
                Some(Self::Pong),

            3 =>
                Some(Self::Transaction),

            4 =>
                Some(Self::BalanceRequest),

            5 =>
                Some(Self::BalanceResponse),

            6 =>
                Some(Self::GetPeers),

            7 =>
                Some(Self::Peers),

            8 =>
                Some(Self::NodeAddress),

            9 =>
                Some(Self::RegisterAssetRequest),

            10 =>
                Some(Self::RegisterAssetResponse),

            11 =>
                Some(Self::GetStatus),

            12 =>
                Some(Self::Status),

            13 =>
                Some(Self::GetBlocks),

            14 =>
                Some(Self::Blocks),

            15 =>
                Some(Self::Hello),

            16 =>
                Some(Self::HelloAck),

            17 =>
                Some(Self::GetHeaders),

            18 =>
                Some(Self::Headers),

            19 =>
                Some(Self::GetBlock),

            20 =>
                Some(Self::Block),

            21 =>
                Some(Self::NewBlock),

            22 =>
                Some(Self::NewTransaction),

            _ =>
                None,
        }
    }


    /*
     * ========================================================
     * WRITE MESSAGE
     * ========================================================
     *
     * Frame:
     *
     *     [1 byte message type]
     *     [4 byte payload length]
     *     [payload]
     *
     * Length:
     *
     *     big-endian u32
     *
     * ========================================================
     */

    pub fn write_to<W: Write>(
        &self,
        writer: &mut W,
        payload: &[u8],
    ) -> io::Result<()> {

        let length =
            u32::try_from(
                payload.len()
            )
            .map_err(
                |_| {
                    io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "Payload too large",
                    )
                }
            )?;


        /*
         * Message type.
         */

        writer.write_all(
            &self.to_bytes()
        )?;


        /*
         * Payload length.
         */

        writer.write_all(
            &length.to_be_bytes()
        )?;


        /*
         * Payload.
         */

        writer.write_all(
            payload
        )?;


        writer.flush()?;


        Ok(())
    }


    /*
     * ========================================================
     * READ MESSAGE
     * ========================================================
     *
     * Reads exactly one complete PEP message.
     *
     * ========================================================
     */

    pub fn read_from<R: Read>(
    reader: &mut R,
) -> io::Result<(Self, Vec<u8>)> {

    // ========================================================
    // MESSAGE TYPE
    // ========================================================

    let mut type_buffer = [0u8; 1];

    reader.read_exact(&mut type_buffer)
        .map_err(|error| {
            io::Error::new(
                error.kind(),
                format!(
                    "PEP frame: failed reading message type: {}",
                    error
                ),
            )
        })?;

    let message =
        Self::from_byte(type_buffer[0])
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "PEP frame: unknown message type byte {}",
                        type_buffer[0]
                    ),
                )
            })?;

    // ========================================================
    // PAYLOAD LENGTH
    // ========================================================

    let mut length_buffer = [0u8; 4];

    reader.read_exact(&mut length_buffer)
        .map_err(|error| {
            io::Error::new(
                error.kind(),
                format!(
                    "PEP frame: message {:?} received, \
                     but payload length was incomplete: {}",
                    message,
                    error
                ),
            )
        })?;

    let length =
        u32::from_be_bytes(length_buffer) as usize;

    // ========================================================
    // SAFETY LIMIT
    // ========================================================

    if length > Self::MAX_PAYLOAD_SIZE {
        return Err(
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "PEP frame: payload too large: {} bytes",
                    length
                ),
            )
        );
    }

    // ========================================================
    // PAYLOAD
    // ========================================================

    let mut payload = vec![0u8; length];

    reader.read_exact(&mut payload)
        .map_err(|error| {
            io::Error::new(
                error.kind(),
                format!(
                    "PEP frame: message {:?} declared \
                     payload {} bytes, but payload was incomplete: {}",
                    message,
                    length,
                    error
                ),
            )
        })?;

    Ok((message, payload))
}


    /*
     * ========================================================
     * HELLO PAYLOAD
     * ========================================================
     *
     * Standard format:
     *
     *     network|protocol|chain_id|height|tip
     *
     * Example:
     *
     *     mainnet|1|1|123|abcdef
     *
     * ========================================================
     */

    pub fn hello_payload(
        network: &str,
        protocol: u32,
        chain_id: u64,
        height: u64,
        tip: &str,
    ) -> Vec<u8> {

        format!(
            "{}|{}|{}|{}|{}",
            network,
            protocol,
            chain_id,
            height,
            tip,
        )
        .into_bytes()
    }


    /*
     * ========================================================
     * HELLO ACK PAYLOAD
     * ========================================================
     *
     * Uses exactly the same structure as HELLO.
     *
     * This allows the receiving node to immediately know
     * the peer's current chain state.
     * ========================================================
     */

    pub fn hello_ack_payload(
        network: &str,
        protocol: u32,
        chain_id: u64,
        height: u64,
        tip: &str,
    ) -> Vec<u8> {

        Self::hello_payload(
            network,
            protocol,
            chain_id,
            height,
            tip,
        )
    }
}