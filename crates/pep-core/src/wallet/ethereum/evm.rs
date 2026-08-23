use anyhow::Result;

use crate::wallet::hd::master::MasterKey;

use super::{
    Address,
    PrivateKey,
    PublicKey,
};

pub struct Evm;

impl Evm {

    /// m/44'/60'/account'/0/index
    pub fn derive(
        master: &MasterKey,
        account: u32,
        index: u32,
    ) -> Result<(PrivateKey, PublicKey, Address)> {

        let secret =
            master.derive_evm(
                account,
                index,
            )?;

        let private =
            PrivateKey::from_bytes(
                secret,
            )?;

        let public =
            private.public_key();

        let address =
            public.address();

        Ok((
            private,
            public,
            address,
        ))

    }

    pub fn derive_private(
        master: &MasterKey,
        account: u32,
        index: u32,
    ) -> Result<PrivateKey> {

        let secret =
            master.derive_evm(
                account,
                index,
            )?;

        PrivateKey::from_bytes(
            secret,
        )

    }

    pub fn derive_public(
        master: &MasterKey,
        account: u32,
        index: u32,
    ) -> Result<PublicKey> {

        Ok(
            Self::derive_private(
                master,
                account,
                index,
            )?
            .public_key()
        )

    }

    pub fn derive_address(
        master: &MasterKey,
        account: u32,
        index: u32,
    ) -> Result<Address> {

        Ok(
            Self::derive_public(
                master,
                account,
                index,
            )?
            .address()
        )

    }

}