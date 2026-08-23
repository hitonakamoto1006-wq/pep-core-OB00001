use anyhow::Result;
use primitive_types::U256;

use super::{
    Address,
    Rpc,
};

#[derive(Clone)]
pub struct Provider {
    rpc: Rpc,
}

impl Provider {

    /// Create provider from RPC endpoint.
    pub fn new(
        url: impl Into<String>,
    ) -> Self {

        Self {
            rpc: Rpc::new(url),
        }

    }

    /// Borrow RPC.
    pub fn rpc(
        &self,
    ) -> &Rpc {

        &self.rpc

    }

    /// Current chain id.
    pub fn chain_id(
        &self,
    ) -> Result<u64> {

        self.rpc.chain_id()

    }

    /// Account balance.
    pub fn balance(
        &self,
        address: &Address,
    ) -> Result<U256> {

        self.rpc.balance(address)

    }

    /// Account nonce.
    pub fn nonce(
        &self,
        address: &Address,
    ) -> Result<u64> {

        self.rpc.nonce(address)

    }

    /// Current gas price.
    pub fn gas_price(
        &self,
    ) -> Result<U256> {

        self.rpc.gas_price()

    }

    /// Latest block number.
    pub fn block_number(
        &self,
    ) -> Result<u64> {

        self.rpc.block_number()

    }

    /// Current network version.
    pub fn network_version(
        &self,
    ) -> Result<String> {

        self.rpc.net_version()

    }

    /// Client version.
    pub fn client_version(
        &self,
    ) -> Result<String> {

        self.rpc.client_version()

    }

}