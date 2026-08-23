// ============================================================
// WALLET MODULE
// ============================================================

pub mod pep_chain;
pub mod ethereum;


// ============================================================
// WALLET CORE
// ============================================================

pub mod checksum;
pub mod entropy;
pub mod mnemonic;
pub mod seed;
pub mod hd;
pub mod crypto;


// ============================================================
// WALLET CLI
// ============================================================

pub mod cli;


// ============================================================
// PEP CHAIN WALLET EXPORTS
// ============================================================

pub use pep_chain::{
    Address,
    PrivateKey,
    PublicKey,
    Signature,
    Signer,
    Wallet,
};


// ============================================================
// ETHEREUM WALLET EXPORTS
// ============================================================

pub use ethereum::{
    Address as EthAddress,
    PrivateKey as EthPrivateKey,
    PublicKey as EthPublicKey,
    Signer as EthSigner,
    Wallet as EthWallet,
};