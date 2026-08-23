use crate::blockchain::transaction::Transaction;
use super::{
    PrivateKey,
    PublicKey,
    Signature,
};

use k256::{
    ecdsa::{
        signature::{Signer as _, Verifier as _},
        Signature as EcdsaSignature,
        SigningKey,
        VerifyingKey,
    },
    SecretKey,
};

pub struct Signer;

impl Signer {

    pub fn sign(
        private: &PrivateKey,
        tx: &mut Transaction,
    ) {

        // 1. PrivateKey -> SigningKey
        let secret =
            SecretKey::from_slice(private.bytes())
                .unwrap();

        let signing_key =
            SigningKey::from(secret);

        // 2. Transaction bytes
        let message =
            tx.signing_message();

        // 3. ECDSA Sign
        let signature: EcdsaSignature =
            signing_key.sign(&message);

        // 4. 64-byte signature
        let bytes: [u8; 64] =
            signature.to_bytes().into();

        tx.set_signature(
            Signature::new(bytes)
        );
    }

    pub fn verify(
        public: &PublicKey,
        tx: &Transaction,
    ) -> bool {

        let Some(signature) = tx.signature() else {
            return false;
        };

        let verifying_key =
            VerifyingKey::from_sec1_bytes(
                public.bytes(),
            )
            .unwrap();

        let signature =
            EcdsaSignature::from_slice(
                signature.bytes(),
            )
            .unwrap();

        verifying_key
            .verify(
                &tx.signing_message(),
                &signature,
            )
            .is_ok()
    }
}