use primitive_types::U256;

use super::{
    transaction::AccessListItem,
    Address,
};

pub struct Rlp;

impl Rlp {

    /* ===========================
            Bytes
    =========================== */

    pub fn encode_bytes(
        data: &[u8],
    ) -> Vec<u8> {

        match data.len() {

            0 => {

                vec![0x80]

            }

            1 if data[0] < 0x80 => {

                vec![data[0]]

            }

            len if len <= 55 => {

                let mut out =
                    Vec::with_capacity(
                        len + 1
                    );

                out.push(
                    0x80 + len as u8
                );

                out.extend_from_slice(
                    data,
                );

                out

            }

            len => {

                let len_bytes =
                    Self::length_bytes(
                        len,
                    );

                let mut out =
                    Vec::new();

                out.push(
                    0xb7 + len_bytes.len() as u8
                );

                out.extend_from_slice(
                    &len_bytes,
                );

                out.extend_from_slice(
                    data,
                );

                out

            }

        }

    }


    /* ===========================
            Integer
    =========================== */

    pub fn encode_u64(
        value: u64,
    ) -> Vec<u8> {

        if value == 0 {

            return vec![0x80];

        }

        let bytes =
            value.to_be_bytes();

        let first =
            bytes.iter()
                .position(
                    |x| *x != 0
                )
                .unwrap();

        Self::encode_bytes(
            &bytes[first..],
        )

    }


    pub fn encode_u256(
        value: U256,
    ) -> Vec<u8> {

        if value.is_zero() {

            return vec![0x80];

        }

        let mut bytes =
            [0u8; 32];

        value.to_big_endian(
            &mut bytes,
        );

        let first =
            bytes.iter()
                .position(
                    |x| *x != 0
                )
                .unwrap();

        Self::encode_bytes(
            &bytes[first..],
        )

    }


    /* ===========================
            Address
    =========================== */

    pub fn encode_address(
        address: &Option<Address>,
    ) -> Vec<u8> {

        match address {

            Some(addr) => {

                Self::encode_bytes(
                    addr.bytes(),
                )

            }

            None => {

                vec![0x80]

            }

        }

    }


    /* ===========================
            Access List
    =========================== */

    pub fn encode_access_list(
        list: &[AccessListItem],
    ) -> Vec<u8> {

        let mut items =
            Vec::new();


        for item in list {

            let address =
                Self::encode_bytes(
                    item.address()
                        .bytes(),
                );


            let mut keys =
                Vec::new();


            for key in item.storage_keys() {

                keys.push(
                    Self::encode_bytes(
                        key,
                    ),
                );

            }


            let keys =
                Self::encode_list(
                    keys,
                );


            let entry =
                Self::encode_list(
                    vec![
                        address,
                        keys,
                    ],
                );


            items.push(
                entry,
            );

        }


        Self::encode_list(
            items,
        )

    }


    /* ===========================
            List
    =========================== */

    pub fn encode_list(
        items: Vec<Vec<u8>>,
    ) -> Vec<u8> {

        let payload_len =
            items.iter()
                .map(
                    |x| x.len()
                )
                .sum::<usize>();


        let mut payload =
            Vec::new();


        for item in items {

            payload.extend(
                item,
            );

        }


        if payload_len <= 55 {

            let mut out =
                Vec::new();

            out.push(
                0xc0 + payload_len as u8
            );

            out.extend(
                payload,
            );

            out

        } else {

            let len_bytes =
                Self::length_bytes(
                    payload_len,
                );

            let mut out =
                Vec::new();

            out.push(
                0xf7 + len_bytes.len() as u8
            );

            out.extend(
                len_bytes,
            );

            out.extend(
                payload,
            );

            out

        }

    }


    /* ===========================
            Internal
    =========================== */

    fn length_bytes(
        mut len: usize,
    ) -> Vec<u8> {

        let mut out =
            Vec::new();


        while len > 0 {

            out.push(
                (len & 0xff) as u8
            );

            len >>= 8;

        }


        out.reverse();

        out

    }

}
use sha3::{Digest, Keccak256};

use super::Transaction;

impl Rlp {

    /* ===========================
        EIP-1559 Unsigned
    =========================== */

    /// Returns:
    ///
    /// 0x02 || RLP([
    ///     chain_id,
    ///     nonce,
    ///     max_priority_fee_per_gas,
    ///     max_fee_per_gas,
    ///     gas_limit,
    ///     destination,
    ///     value,
    ///     data,
    ///     access_list
    /// ])
    pub fn encode_unsigned(
        tx: &Transaction,
    ) -> Vec<u8> {

        let payload =
            Self::encode_list(

                vec![

                    Self::encode_u64(
                        tx.chain_id(),
                    ),

                    Self::encode_u64(
                        tx.nonce(),
                    ),

                    Self::encode_u256(
                        tx.max_priority_fee_per_gas(),
                    ),

                    Self::encode_u256(
                        tx.max_fee_per_gas(),
                    ),

                    Self::encode_u64(
                        tx.gas_limit(),
                    ),

                    Self::encode_address(
                        &tx.to().cloned(),
                    ),

                    Self::encode_u256(
                        tx.value(),
                    ),

                    Self::encode_bytes(
                        tx.data(),
                    ),

                    Self::encode_access_list(
                        tx.access_list(),
                    ),

                ]

            );

        let mut out =
            Vec::with_capacity(
                payload.len() + 1,
            );

        // Typed Transaction (EIP-2718)
        out.push(0x02);

        out.extend_from_slice(
            &payload,
        );

        out

    }


    /* ===========================
            Signing Hash
    =========================== */

    /// Keccak256(
    ///     0x02 || RLP(unsigned)
    /// )
    pub fn signing_hash(
        tx: &Transaction,
    ) -> [u8;32] {

        let payload =
            Self::encode_unsigned(
                tx,
            );

        let hash =
            Keccak256::digest(
                payload,
            );

        let mut out =
            [0u8;32];

        out.copy_from_slice(
            &hash,
        );

        out

    }


    /* ===========================
            Helper
    =========================== */

    pub fn unsigned_hex(
        tx: &Transaction,
    ) -> String {

        hex::encode(

            Self::encode_unsigned(
                tx,
            )

        )

    }
pub fn encode_signed(
    tx: &Transaction,
) -> Vec<u8> {

    let sig =
        tx.signature()
            .expect("missing signature");

    let payload =
        Self::encode_list(

            vec![

                Self::encode_u64(
                    tx.chain_id(),
                ),

                Self::encode_u64(
                    tx.nonce(),
                ),

                Self::encode_u256(
                    tx.max_priority_fee_per_gas(),
                ),

                Self::encode_u256(
                    tx.max_fee_per_gas(),
                ),

                Self::encode_u64(
                    tx.gas_limit(),
                ),

                Self::encode_address(
                    &tx.to().cloned(),
                ),

                Self::encode_u256(
                    tx.value(),
                ),

                Self::encode_bytes(
                    tx.data(),
                ),

                Self::encode_access_list(
                    tx.access_list(),
                ),

                Self::encode_u64(
                    sig.v() as u64,
                ),

                Self::encode_bytes(
                    sig.r(),
                ),
                Self::encode_bytes(
                    sig.s(),
                ),

            ],

        );

    let mut out =
        Vec::with_capacity(
            payload.len() + 1,
        );

    out.push(0x02);

    out.extend_from_slice(
        &payload,
    );

    out

}
}