use primitive_types::U256;

use super::{
    Address,
    Signature,
    Rlp,
};

#[derive(Clone, Debug)]
pub struct AccessListItem {

    address: Address,

    storage_keys: Vec<[u8; 32]>,

}

impl AccessListItem {

    pub fn new(
        address: Address,
    ) -> Self {

        Self {

            address,

            storage_keys: Vec::new(),

        }

    }

    pub fn push_storage_key(

        &mut self,

        key: [u8; 32],

    ) {

        self.storage_keys.push(
            key,
        );

    }

    pub fn address(
        &self,
    ) -> &Address {

        &self.address

    }

    pub fn storage_keys(
        &self,
    ) -> &[[u8; 32]] {

        &self.storage_keys

    }

}

#[derive(Clone, Debug)]
pub struct Transaction {

    /// EIP-2718 typed transaction.
    tx_type: u8,

    /// Network.
    chain_id: u64,

    /// Sender nonce.
    nonce: u64,

    /// EIP-1559.
    max_priority_fee_per_gas: U256,

    max_fee_per_gas: U256,

    gas_limit: u64,

    /// Receiver.
    to: Option<Address>,

    /// Native value.
    value: U256,

    /// Calldata.
    data: Vec<u8>,

    /// EIP-2930.
    access_list: Vec<AccessListItem>,

    /// Signature.
    signature: Option<Signature>,

}

impl Default for Transaction {

    fn default() -> Self {

        Self::new()

    }

}

impl Transaction {

    /* ===========================
            Constructor
    =========================== */

    pub fn new() -> Self {

        Self {

            tx_type: 0x02,

            chain_id: 1,

            nonce: 0,

            max_priority_fee_per_gas:
                U256::zero(),

            max_fee_per_gas:
                U256::zero(),

            gas_limit: 21_000,

            to: None,

            value: U256::zero(),

            data: Vec::new(),

            access_list: Vec::new(),

            signature: None,

        }

    }

    /* ===========================
            Builder
    =========================== */

    pub fn set_chain_id(

        mut self,

        id: u64,

    ) -> Self {

        self.chain_id = id;

        self

    }

    pub fn set_nonce(

        mut self,

        nonce: u64,

    ) -> Self {

        self.nonce = nonce;

        self

    }

    pub fn set_max_priority_fee_per_gas(

        mut self,

        fee: U256,

    ) -> Self {

        self.max_priority_fee_per_gas =
            fee;

        self

    }

    pub fn set_max_fee_per_gas(

        mut self,

        fee: U256,

    ) -> Self {

        self.max_fee_per_gas =
            fee;

        self

    }

    pub fn set_gas_limit(

        mut self,

        gas: u64,

    ) -> Self {

        self.gas_limit = gas;

        self

    }

    pub fn set_to(

        mut self,

        address: Address,

    ) -> Self {

        self.to =
            Some(address);

        self

    }

    pub fn set_value(

        mut self,

        value: U256,

    ) -> Self {

        self.value = value;

        self

    }

    pub fn set_data(

        mut self,

        data: Vec<u8>,

    ) -> Self {

        self.data = data;

        self

    }

    pub fn push_access_list(

        mut self,

        item: AccessListItem,

    ) -> Self {

        self.access_list.push(
            item,
        );

        self

    }

    pub fn set_signature(

        mut self,

        signature: Signature,

    ) -> Self {

        self.signature =
            Some(signature);

        self

    }
        /* ===========================
            Getter
    =========================== */

    pub fn tx_type(
        &self,
    ) -> u8 {

        self.tx_type

    }

    pub fn chain_id(
        &self,
    ) -> u64 {

        self.chain_id

    }

    pub fn nonce(
        &self,
    ) -> u64 {

        self.nonce

    }

    pub fn max_priority_fee_per_gas(
        &self,
    ) -> U256 {

        self.max_priority_fee_per_gas

    }

    pub fn max_fee_per_gas(
        &self,
    ) -> U256 {

        self.max_fee_per_gas

    }

    pub fn gas_limit(
        &self,
    ) -> u64 {

        self.gas_limit

    }

    pub fn to(
        &self,
    ) -> Option<&Address> {

        self.to.as_ref()

    }

    pub fn value(
        &self,
    ) -> U256 {

        self.value

    }

    pub fn data(
        &self,
    ) -> &[u8] {

        &self.data

    }

    pub fn access_list(
        &self,
    ) -> &[AccessListItem] {

        &self.access_list

    }

    pub fn signature(
        &self,
    ) -> Option<&Signature> {

        self.signature.as_ref()

    }

    /* ===========================
            Utility
    =========================== */

    pub fn is_signed(
        &self,
    ) -> bool {

        self.signature.is_some()

    }

    pub fn has_receiver(
        &self,
    ) -> bool {

        self.to.is_some()

    }

    pub fn has_data(
        &self,
    ) -> bool {

        !self.data.is_empty()

    }

    pub fn is_contract_creation(
        &self,
    ) -> bool {

        self.to.is_none()

    }

    pub fn sign(
        &mut self,
        signature: Signature,
    ) {

        self.signature =
            Some(signature);

    }

    pub fn clear_signature(
        &mut self,
    ) {

        self.signature = None;

    }

    pub fn take_signature(
        &mut self,
    ) -> Option<Signature> {

        self.signature.take()

    }

    pub fn unsigned(
        &self,
    ) -> Self {

        let mut tx =
            self.clone();

        tx.signature = None;

        tx

    }

    /* ===========================
            Hash
    =========================== */

    /// EIP-1559 signing hash.
pub fn signing_hash(
    &self,
) -> [u8; 32] {

    Rlp::signing_hash(self)

}

    /// Transaction hash.
    ///
    /// Sau khi có RLP encoder,
    /// hàm này sẽ hash raw signed tx.
    pub fn tx_hash(
        &self,
    ) -> [u8; 32] {

        self.signing_hash()

    }

}