use std::{
    collections::{
        HashMap,
        HashSet,
    },
    net::SocketAddr,
};


// ============================================================
// PEER
// ============================================================
//
// Peer đại diện cho một P2P endpoint.
//
// Hiện tại vẫn dùng SocketAddr làm identity tạm thời.
// Sau phase persistent connection sẽ đổi sang:
//
//     PeerId + Vec<SocketAddr>
//
// ============================================================

#[derive(
    Debug,
    Clone,
    Copy,
    Eq,
    PartialEq,
    Hash,
)]
pub struct Peer {

    pub address:
        SocketAddr,
}


impl Peer {

    pub fn new(
        address: SocketAddr,
    ) -> Self {

        Self {
            address,
        }
    }
}


// ============================================================
// PEER MANAGER
// ============================================================

pub struct PeerManager {

    peers:
        HashSet<Peer>,

    failures:
        HashMap<
            SocketAddr,
            u32,
        >,
}


impl PeerManager {

    // ========================================================
    // NEW
    // ========================================================

    pub fn new() -> Self {

        Self {

            peers:
                HashSet::new(),

            failures:
                HashMap::new(),
        }
    }


    // ========================================================
    // VALIDATE ADDRESS
    // ========================================================

    fn valid_address(
        address: SocketAddr,
    ) -> bool {

        if address
            .ip()
            .is_unspecified()
        {
            return false;
        }

        if address.port() == 0 {
            return false;
        }

        true
    }


    // ========================================================
    // ADD
    // ========================================================

    pub fn add(
        &mut self,
        address: SocketAddr,
    ) -> bool {

        if !Self::valid_address(
            address
        ) {
            return false;
        }

        self.peers.insert(
            Peer::new(
                address
            )
        )
    }


    // ========================================================
    // ADD MANY
    // ========================================================

    pub fn add_many<I>(
        &mut self,
        addresses: I,
    ) -> usize

    where
        I: IntoIterator<
            Item = SocketAddr
        >,
    {

        let mut added =
            0usize;

        for address
            in addresses
        {
            if self.add(
                address
            ) {
                added += 1;
            }
        }

        added
    }


    // ========================================================
    // REMOVE
    // ========================================================

    pub fn remove(
        &mut self,
        address: &SocketAddr,
    ) -> bool {

        let removed =
            self.peers.remove(
                &Peer::new(
                    *address
                )
            );

        if removed {
            self.failures.remove(
                address
            );
        }

        removed
    }


    // ========================================================
    // CONTAINS
    // ========================================================

    pub fn contains(
        &self,
        address: &SocketAddr,
    ) -> bool {

        self.peers.contains(
            &Peer::new(
                *address
            )
        )
    }


    // ========================================================
    // ALL
    // ========================================================

    pub fn all(
        &self,
    ) -> Vec<SocketAddr> {

        let mut peers =
            self.peers
                .iter()
                .map(
                    |peer|
                        peer.address
                )
                .collect::<Vec<_>>();

        peers.sort_by(
            |a, b| {
                a.to_string()
                    .cmp(
                        &b.to_string()
                    )
            }
        );

        peers
    }


    // ========================================================
    // CANDIDATES
    // ========================================================
    //
    // Persistent connection manager sử dụng hàm này.
    //
    // Peer có >= 3 failure sẽ tạm thời bị bỏ qua.
    //
    // ========================================================

    pub fn candidates(
        &self,
    ) -> Vec<SocketAddr> {

        const MAX_FAILURES:
            u32 = 3;

        let mut result =
            self.peers
                .iter()
                .filter_map(
                    |peer| {

                        let failures =
                            self.failures
                                .get(
                                    &peer.address
                                )
                                .copied()
                                .unwrap_or(0);

                        if failures <
                            MAX_FAILURES
                        {
                            Some(
                                peer.address
                            )
                        } else {
                            None
                        }
                    }
                )
                .collect::<Vec<_>>();

        result.sort_by(
            |a, b| {

                let af =
                    self.failures
                        .get(a)
                        .copied()
                        .unwrap_or(0);

                let bf =
                    self.failures
                        .get(b)
                        .copied()
                        .unwrap_or(0);

                af.cmp(&bf)
                    .then_with(
                        || {
                            a.to_string()
                                .cmp(
                                    &b.to_string()
                                )
                        }
                    )
            }
        );

        result
    }


    // ========================================================
    // MARK SUCCESS
    // ========================================================

    pub fn mark_success(
        &mut self,
        address: SocketAddr,
    ) {

        self.add(
            address
        );

        self.failures.remove(
            &address
        );
    }


    // ========================================================
    // MARK FAILURE
    // ========================================================

    pub fn mark_failure(
        &mut self,
        address: SocketAddr,
    ) {

        if !self.contains(
            &address
        ) {
            self.add(
                address
            );
        }

        let entry =
            self.failures
                .entry(
                    address
                )
                .or_insert(0);

        *entry =
            entry.saturating_add(1);
    }


    // ========================================================
    // FAILURE COUNT
    // ========================================================

    pub fn failure_count(
        &self,
        address: &SocketAddr,
    ) -> u32 {

        self.failures
            .get(address)
            .copied()
            .unwrap_or(0)
    }


    // ========================================================
    // RESET FAILURES
    // ========================================================

    pub fn reset_failures(
        &mut self,
    ) {

        self.failures.clear();
    }


    // ========================================================
    // LEN
    // ========================================================

    pub fn len(
        &self,
    ) -> usize {

        self.peers.len()
    }


    // ========================================================
    // IS EMPTY
    // ========================================================

    pub fn is_empty(
        &self,
    ) -> bool {

        self.peers.is_empty()
    }


    // ========================================================
    // CLEAR
    // ========================================================

    pub fn clear(
        &mut self,
    ) {

        self.peers.clear();
        self.failures.clear();
    }
}


impl Default for PeerManager {

    fn default() -> Self {

        Self::new()
    }
}