use anyhow::{anyhow, Result};

use super::super::{
    PrivateKey,
    Provider,
    Rpc,
    Rlp,
    Transaction,
};

pub struct EvmClient {
    rpc: Rpc,
}

impl EvmClient {

    pub fn new(
        url: impl Into<String>,
    ) -> Self {

        Self {
            rpc: Rpc::new(url),
        }

    }

    pub fn send_transaction(
        &self,
        key: &PrivateKey,
        tx: &Transaction,
    ) -> Result<String> {

        let mut tx =
            tx.clone();

        let sig =
            key.sign_transaction(
                &tx,
            )?;

        tx.sign(sig);

        let raw =
            Rlp::encode_signed(
                &tx,
            );

        self.rpc
            .send_raw_transaction(
                &raw,
            )

    }

}