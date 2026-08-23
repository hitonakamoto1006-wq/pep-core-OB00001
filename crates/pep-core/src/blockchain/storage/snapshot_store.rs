use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::blockchain::{
    state::State,
    storage::state_store::StateStore,
};

pub struct SnapshotStore;

impl SnapshotStore {

    const DIR: &'static str = "data/snapshots";

    /// Tạo snapshot
    pub fn create(
        state: &State,
    ) {
        StateStore::save_snapshot(state);
    }

    /// Load snapshot theo height
    pub fn load(
        height: u64,
    ) -> Option<State> {
        StateStore::load_snapshot(height)
    }

    /// Snapshot mới nhất
    pub fn latest() -> Option<State> {
        StateStore::latest_snapshot()
    }

    /// Rollback
    pub fn rollback(
        height: u64,
    ) -> Option<State> {
        StateStore::rollback(height)
    }

    /// Xóa snapshot cũ
    pub fn prune(
        keep_after: u64,
    ) {
        StateStore::prune(keep_after);
    }

    /// Snapshot có tồn tại không
    pub fn exists(
        height: u64,
    ) -> bool {

        Self::path(height).exists()
    }

    /// Danh sách snapshot
    pub fn list() -> Vec<u64> {

        let mut snapshots = Vec::new();

        let Ok(entries) = fs::read_dir(Self::DIR) else {
            return snapshots;
        };

        for entry in entries {

            let Ok(entry) = entry else {
                continue;
            };

            // ===== SỬA LỖI E0716 =====
            let path = entry.path();

            let Some(stem) = path.file_stem() else {
                continue;
            };

            let Some(name) = stem.to_str() else {
                continue;
            };

            if let Ok(height) = name.parse::<u64>() {
                snapshots.push(height);
            }
        }

        snapshots.sort_unstable();

        snapshots
    }

    /// Số snapshot
    pub fn count() -> usize {
        Self::list().len()
    }

    /// Đường dẫn snapshot
    pub fn path(
        height: u64,
    ) -> PathBuf {

        PathBuf::from(format!(
            "{}/{}.dat",
            Self::DIR,
            height,
        ))
    }

    /// Xóa toàn bộ snapshot
    pub fn clear() {

        if Path::new(Self::DIR).exists() {
            let _ = fs::remove_dir_all(Self::DIR);
        }
    }
}