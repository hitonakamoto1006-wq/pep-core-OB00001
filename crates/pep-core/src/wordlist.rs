use std::io;

pub struct WordList {
    words: Vec<String>,
}

impl WordList {
    pub fn new() -> Self {
        // PEP Chain root:
        // C:\Users\MYPC\desktop\pep-chain\wordlist.txt
        //
        // CARGO_MANIFEST_DIR khi compile pep-core:
        // C:\Users\MYPC\desktop\pep-chain\crates\pep-core
        //
        // => ../../../wordlist.txt

        let path = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../Wordlist.txt"
);

        let content = std::fs::read_to_string(path)
            .unwrap_or_else(|error| {
                panic!(
                    "Cannot read PEP wordlist at '{}': {}",
                    path,
                    error
                );
            });

        let words: Vec<String> = content
            .lines()
            .map(str::trim)
            .filter(|word| !word.is_empty())
            .map(ToOwned::to_owned)
            .collect();

        if words.is_empty() {
            panic!("PEP wordlist is empty");
        }

        Self { words }
    }

    pub fn lookup(&self, index: u16) -> &str {
        self.words
            .get(index as usize)
            .unwrap_or_else(|| {
                panic!(
                    "PEP wordlist index {} is out of range ({} words)",
                    index,
                    self.words.len()
                )
            })
    }

    pub fn find(&self, word: &str) -> Option<u16> {
        self.words
            .iter()
            .position(|w| w == word)
            .map(|index| index as u16)
    }

    pub fn len(&self) -> usize {
        self.words.len()
    }
}