use bitvec::prelude::*;
use std::fmt;

use crate::wallet::entropy::MasterEntropy;
use crate::wordlist::WordList;

pub struct Mnemonic {
    words: [String; 25],
}

impl Mnemonic {

    pub fn new(
        payload: &Payload,
        wordlist: &WordList,
    ) -> Self {

        let indices = payload.to_indices();

        let mut words =
            std::array::from_fn(|_| String::new());

        for (i, index) in indices.indices().iter().enumerate() {
            words[i] =
                wordlist.lookup(*index).to_string();
        }

        Self {
            words,
        }
    }

    pub fn from_words(
        words: [String; 25],
    ) -> Self {

        Self {
            words,
        }
    }

    pub fn from_phrase(
        phrase: &str,
    ) -> Result<Self, String> {

        let parts: Vec<&str> =
            phrase
                .split_whitespace()
                .collect();

        if parts.len() != 25 {
            return Err(
                format!(
                    "Expected 25 words, got {}",
                    parts.len()
                )
            );
        }

        let mut words =
            std::array::from_fn(|_| String::new());

        for i in 0..25 {
            words[i] = parts[i].to_string();
        }

        Ok(
            Self {
                words,
            }
        )
    }

    pub fn words(
        &self,
    ) -> &[String; 25] {
        &self.words
    }

    pub fn phrase(
        &self,
    ) -> String {

        self.words.join(" ")
    }

    pub fn to_indices(
        &self,
        wordlist: &WordList,
    ) -> MnemonicIndex {

        let mut indices = [0u16; 25];

        for (i, word) in self.words.iter().enumerate() {

            let index =
                wordlist
                    .find(word)
                    .expect("Invalid mnemonic word");

            indices[i] = index;
        }

        MnemonicIndex {
            indices,
        }
    }
}

impl fmt::Display for Mnemonic {

    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {

        write!(
            f,
            "{}",
            self.words.join(" ")
        )
    }
}

pub struct MnemonicIndex {
    indices: [u16; 25],
}

impl MnemonicIndex {

    pub(crate) fn indices(
        &self,
    ) -> &[u16; 25] {

        &self.indices
    }
}

pub struct Payload {

    version: u8,

    entropy: [u8; 32],

    checksum: u8,

    reserved: u32,
}

impl Payload {

    pub fn new(
        entropy: &MasterEntropy,
        checksum: u8,
    ) -> Self {

        Self {

            version: 1,

            entropy: *entropy.bytes(),

            checksum,

            reserved: 0,
        }
    }

    pub fn to_indices(
        &self,
    ) -> MnemonicIndex {

        let bits =
            self.to_bits();

        let mut indices =
            [0u16; 25];

        for group in 0..25 {

            let start =
                group * 12;

            let mut value =
                0u16;

            for i in 0..12 {

                value <<= 1;

                if bits[start + i] {
                    value |= 1;
                }
            }

            indices[group] =
                value;
        }

        MnemonicIndex {
            indices,
        }
    }

    pub fn to_bits(
        &self,
    ) -> BitVec<u8, Msb0> {

        let mut bits =
            BitVec::<u8, Msb0>::with_capacity(300);

        for i in (0..8).rev() {
            bits.push(
                ((self.version >> i) & 1) == 1
            );
        }

        for byte in &self.entropy {

            for i in (0..8).rev() {

                bits.push(
                    ((*byte >> i) & 1) == 1
                );
            }
        }

        for i in (0..8).rev() {
            bits.push(
                ((self.checksum >> i) & 1) == 1
            );
        }

        for i in (0..28).rev() {
            bits.push(
                ((self.reserved >> i) & 1) == 1
            );
        }

        bits
    }

    pub fn from_indices(
        indices: &MnemonicIndex,
    ) -> Self {

        let mut bits =
            BitVec::<u8, Msb0>::new();

        for index in indices.indices() {

            for i in (0..12).rev() {

                bits.push(
                    ((*index >> i) & 1) == 1
                );
            }
        }

        assert_eq!(
            bits.len(),
            300,
        );

        let mut pos = 0;

        let mut version = 0u8;

        for _ in 0..8 {

            version <<= 1;

            if bits[pos] {
                version |= 1;
            }

            pos += 1;
        }

        let mut entropy =
            [0u8; 32];

        for byte in &mut entropy {

            let mut value =
                0u8;

            for _ in 0..8 {

                value <<= 1;

                if bits[pos] {
                    value |= 1;
                }

                pos += 1;
            }

            *byte = value;
        }

        let mut checksum =
            0u8;

        for _ in 0..8 {

            checksum <<= 1;

            if bits[pos] {
                checksum |= 1;
            }

            pos += 1;
        }

        let mut reserved =
            0u32;

        for _ in 0..28 {

            reserved <<= 1;

            if bits[pos] {
                reserved |= 1;
            }

            pos += 1;
        }

        Self {

            version,

            entropy,

            checksum,

            reserved,
        }
    }
}