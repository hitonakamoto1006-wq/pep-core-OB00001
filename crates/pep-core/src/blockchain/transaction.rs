use crate::blockchain::{
    asset::AssetId,
    hash::Hash256,
};

use crate::wallet::{
    Address,
    PublicKey,
    Signature,
};

use crate::wallet::crypto;

#[derive(
    Clone,
    Debug,
)]
pub struct Transaction {
    pub transaction_type: TransactionType,

    pub from: Address,

    pub to: Address,

    /// Asset được chuyển / mint / burn.
    pub asset: AssetId,

    /// Amount tính theo đơn vị nguyên của asset.
    pub amount: u64,

    /// Account nonce.
    pub nonce: u64,

    /// Public key của sender.
    pub public_key: PublicKey,

    /// Signature của transaction.
    pub signature: Option<Signature>,
}

impl Transaction {

    // =========================================================
    // CONSTRUCTORS
    // =========================================================

    /// Tạo transaction PEP.
    pub fn new(
        from: Address,
        to: Address,
        amount: u64,
        nonce: u64,
        public_key: PublicKey,
        transaction_type: TransactionType,
    ) -> Self {

        Self::new_asset(
            from,
            to,
            AssetId::new(
                AssetId::PEP,
            ),
            amount,
            nonce,
            public_key,
            transaction_type,
        )
    }

    /// Tạo transaction cho asset cụ thể.
    pub fn new_asset(
        from: Address,
        to: Address,
        asset: AssetId,
        amount: u64,
        nonce: u64,
        public_key: PublicKey,
        transaction_type: TransactionType,
    ) -> Self {

        Self {
            transaction_type,
            from,
            to,
            asset,
            amount,
            nonce,
            public_key,
            signature: None,
        }
    }

    // =========================================================
    // SIGNING BYTES
    // =========================================================

    /// Dữ liệu được ký.
    ///
    /// Signature KHÔNG nằm trong message này.
    ///
    /// Mọi field có thể ảnh hưởng đến state transition
    /// đều phải nằm trong signing message.
    pub fn signing_message(
        &self,
    ) -> Vec<u8> {

        format!(
            "{}|{}|{}|{}|{}|{}|{}",
            self.transaction_type.code(),
            self.from,
            self.to,
            self.asset,
            self.amount,
            self.nonce,
            self.public_key.to_hex(),
        )
        .into_bytes()
    }

    // =========================================================
    // SERIALIZATION
    // =========================================================

    /// Canonical transaction serialization.
    ///
    /// Format:
    ///
    /// type|from|to|asset|amount|nonce|pubkey|signature
    ///
    /// Signature rỗng nếu transaction chưa ký.
    pub fn serialize(
        &self,
    ) -> String {

        let signature =
            match &self.signature {

                Some(signature) =>
                    signature.to_hex(),

                None =>
                    String::new(),
            };

        format!(
            "{}|{}|{}|{}|{}|{}|{}|{}",
            self.transaction_type.code(),
            self.from,
            self.to,
            self.asset,
            self.amount,
            self.nonce,
            self.public_key.to_hex(),
            signature,
        )
    }

    // =========================================================
    // TRANSACTION HASH / TXID
    // =========================================================

    /// Transaction hash / TXID.
    ///
    /// Khác signing_message():
    ///
    /// - signing_message() dùng để ký.
    /// - hash() dùng để định danh transaction.
    ///
    /// TXID commit toàn bộ serialized transaction,
    /// bao gồm signature nếu transaction đã ký.
    pub fn hash(
        &self,
    ) -> Hash256 {

        Hash256::new(
            crypto::sha256_bytes(
                self.serialize()
                    .as_bytes(),
            )
        )
    }

    pub fn id(
        &self,
    ) -> String {

        self.hash().to_string()
    }

    // =========================================================
    // SIGNATURE
    // =========================================================

    pub fn set_signature(
        &mut self,
        signature: Signature,
    ) {

        self.signature =
            Some(signature);
    }

    pub fn signature(
        &self,
    ) -> Option<&Signature> {

        self.signature.as_ref()
    }

    pub fn is_signed(
        &self,
    ) -> bool {

        self.signature.is_some()
    }

    // =========================================================
    // DESERIALIZATION
    // =========================================================

    pub fn deserialize(
        data: &str,
    ) -> Option<Self> {

        let parts:
            Vec<&str> =
            data.split('|')
                .collect();

        // =====================================================
        // CURRENT FORMAT
        // =====================================================
        //
        // type|from|to|asset|amount|nonce|pubkey|signature
        //

        if parts.len() == 8 {

            let transaction_type =
                Self::parse_transaction_type(
                    parts[0],
                )?;

            let amount =
                parts[4]
                    .parse::<u64>()
                    .ok()?;

            let nonce =
                parts[5]
                    .parse::<u64>()
                    .ok()?;

            let public_key =
                PublicKey::from_hex(
                    parts[6],
                )?;

            let signature =
                if parts[7].is_empty() {

                    None

                } else {

                    Some(
                        Signature::from_hex(
                            parts[7],
                        )?
                    )
                };

            return Some(
                Self {
                    transaction_type,

                    from:
                        Address::new(
                            parts[1]
                                .to_string(),
                        ),

                    to:
                        Address::new(
                            parts[2]
                                .to_string(),
                        ),

                    asset:
                        AssetId::new(
                            parts[3],
                        ),

                    amount,

                    nonce,

                    public_key,

                    signature,
                }
            );
        }

        // =====================================================
        // LEGACY FORMAT
        // =====================================================
        //
        // type|from|to|amount|nonce|pubkey|signature
        //
        // Legacy transaction được migrate thành PEP.
        //

        if parts.len() == 7 {

            let transaction_type =
                Self::parse_transaction_type(
                    parts[0],
                )?;

            let amount =
                parts[3]
                    .parse::<u64>()
                    .ok()?;

            let nonce =
                parts[4]
                    .parse::<u64>()
                    .ok()?;

            let public_key =
                PublicKey::from_hex(
                    parts[5],
                )?;

            let signature =
                if parts[6].is_empty() {

                    None

                } else {

                    Some(
                        Signature::from_hex(
                            parts[6],
                        )?
                    )
                };

            return Some(
                Self {
                    transaction_type,

                    from:
                        Address::new(
                            parts[1]
                                .to_string(),
                        ),

                    to:
                        Address::new(
                            parts[2]
                                .to_string(),
                        ),

                    asset:
                        AssetId::new(
                            AssetId::PEP,
                        ),

                    amount,

                    nonce,

                    public_key,

                    signature,
                }
            );
        }

        None
    }

    // =========================================================
    // TRANSACTION TYPE
    // =========================================================

    fn parse_transaction_type(
        value: &str,
    ) -> Option<TransactionType> {

        match value.parse::<u8>().ok()? {

            0 =>
                Some(
                    TransactionType::Transfer
                ),

            1 =>
                Some(
                    TransactionType::Mint
                ),

            2 =>
                Some(
                    TransactionType::Burn
                ),

            3 =>
                Some(
                    TransactionType::Stake
                ),

            4 =>
                Some(
                    TransactionType::Vote
                ),

            _ =>
                None,
        }
    }
}


// =============================================================
// TRANSACTION TYPE
// =============================================================

#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
)]
#[repr(u8)]
pub enum TransactionType {

    Transfer = 0,

    Mint = 1,

    Burn = 2,

    Stake = 3,

    Vote = 4,
}

impl TransactionType {

    pub fn code(
        &self,
    ) -> u8 {

        *self as u8
    }
}