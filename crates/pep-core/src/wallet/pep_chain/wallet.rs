use crate::{
    wordlist::WordList,
};

use crate::wallet::{
    checksum,
    entropy::MasterEntropy,
    mnemonic::{
        Mnemonic,
        Payload,
    },
    seed::Seed,
};

use crate::blockchain::{
    network::client::Client,
    transaction::{
        Transaction,
        TransactionType,
    },
};

use crate::wallet::{
    Address,
    PrivateKey,
    PublicKey,
    Signer,
};

pub struct Wallet {
    mnemonic: Mnemonic,
    seed: Seed,

    private_key: PrivateKey,
    public_key: PublicKey,

    address: Address,
}

impl Wallet {

    // =========================
    // Create Wallet
    // =========================

    pub fn new() -> Self {

        let wordlist = WordList::new();

        let entropy = MasterEntropy::new();

        let checksum =
            checksum::checksum8(entropy.bytes());

        let payload =
            Payload::new(
                &entropy,
                checksum,
            );

        let mnemonic =
            Mnemonic::new(
                &payload,
                &wordlist,
            );

        let seed =
            Seed::from_mnemonic(
                &mnemonic,
                "",
            );

        let private_key =
            PrivateKey::from_seed(
                &seed,
            );

        let public_key =
            PublicKey::from_private(
                &private_key,
            );

        let address =
            Address::from_public_key(
                &public_key,
            );

        Self {
            mnemonic,
            seed,
            private_key,
            public_key,
            address,
        }
    }

    // =========================
    // Import Wallet
    // =========================

    pub fn from_phrase(
        phrase: &str,
    ) -> Result<Self, String> {

        let mnemonic =
            Mnemonic::from_phrase(
                phrase,
            )?;

        let seed =
            Seed::from_mnemonic(
                &mnemonic,
                "",
            );

        let private_key =
            PrivateKey::from_seed(
                &seed,
            );

        let public_key =
            PublicKey::from_private(
                &private_key,
            );

        let address =
            Address::from_public_key(
                &public_key,
            );

        Ok(
            Self {
                mnemonic,
                seed,
                private_key,
                public_key,
                address,
            }
        )
    }

    // =========================
    // Getter
    // =========================

    pub fn mnemonic(&self) -> &Mnemonic {
        &self.mnemonic
    }

    pub fn seed(&self) -> &Seed {
        &self.seed
    }

    pub fn private_key(&self) -> &PrivateKey {
        &self.private_key
    }

    pub fn public_key(&self) -> &PublicKey {
        &self.public_key
    }

    pub fn address(&self) -> &Address {
        &self.address
    }

    // =========================
    // Transaction
    // =========================

    pub fn create_transaction(
        &self,
        receiver: &Address,
        amount: u64,
        nonce: u64,
        transaction_type: TransactionType,
    ) -> Transaction {

        let mut tx = Transaction::new(
            self.address.clone(),
            receiver.clone(),
            amount,
            nonce,
            self.public_key.clone(),
            transaction_type,
        );

        Signer::sign(
            &self.private_key,
            &mut tx,
        );

        tx
    }

    pub fn send(
        &self,
        node_address: &str,
        receiver: &Address,
        amount: u64,
        transaction_type: TransactionType,
    ) {

        let (_, nonce, _) =
            Client::get_balance(
                node_address,
                self.address(),
            )
            .expect("Cannot connect to node");

        let tx =
            self.create_transaction(
                receiver,
                amount,
                nonce,
                transaction_type,
            );

        Client::send_transaction(
            node_address,
            &tx,
        );
    }
}