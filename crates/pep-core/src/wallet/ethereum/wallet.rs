use anyhow::Result;

use crate::wallet::hd::master::MasterKey;

use super::{
    Address,
    Evm,
    PrivateKey,
    Provider,
    PublicKey,
    Signature,
    Signer,
    Transaction,
};

#[derive(Clone)]
pub struct Wallet {
    private_key: PrivateKey,
    public_key: PublicKey,
    address: Address,
}

impl Wallet {

    /// Derive wallet from master key.
    pub fn from_master(
        master: &MasterKey,
        account: u32,
        index: u32,
    ) -> Result<Self> {

        let (
            private_key,
            public_key,
            address,
        ) = Evm::derive(
            master,
            account,
            index,
        )?;

        Ok(Self {
            private_key,
            public_key,
            address,
        })

    }

    /// Create from private key.
    pub fn from_private_key(
        private_key: PrivateKey,
    ) -> Self {

        let public_key =
            private_key.public_key();

        let address =
            public_key.address();

        Self {
            private_key,
            public_key,
            address,
        }

    }

    /// Address.
    pub fn address(
        &self,
    ) -> &Address {

        &self.address

    }

    /// Address string.
    pub fn address_string(
        &self,
    ) -> String {

        self.address.to_string()

    }

    /// Private key.
    pub fn private_key(
        &self,
    ) -> &PrivateKey {

        &self.private_key

    }

    /// Private key hex.
    pub fn private_key_hex(
        &self,
    ) -> String {

        self.private_key.to_hex()

    }

    /// Public key.
    pub fn public_key(
        &self,
    ) -> &PublicKey {

        &self.public_key

    }

    /// Public key hex.
    pub fn public_key_hex(
        &self,
    ) -> String {

        self.public_key.to_hex()

    }

    /// Balance.
    pub fn balance(
        &self,
        provider: &Provider,
    ) -> Result<primitive_types::U256> {

        provider.balance(
            &self.address,
        )

    }

    /// Nonce.
    pub fn nonce(
        &self,
        provider: &Provider,
    ) -> Result<u64> {

        provider.nonce(
            &self.address,
        )

    }

    /// Sign arbitrary message.
    pub fn sign_message(
        &self,
        message: &[u8],
    ) -> Result<Signature> {

        Signer::sign_message(
            &self.private_key,
            message,
        )

    }

    /// Sign transaction.
    pub fn sign_transaction(
        &self,
        tx: &Transaction,
    ) -> Result<Signature> {

        Signer::sign_transaction(
            &self.private_key,
            tx,
        )

    }

    /// Sign raw hash.
    pub fn sign_hash(
        &self,
        hash: &[u8; 32],
    ) -> Result<Signature> {

        Signer::sign_hash(
            &self.private_key,
            hash,
        )

    }

}