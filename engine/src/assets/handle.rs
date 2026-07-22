use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

/// A unique handle to an asset
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AssetHandle(u64);

impl AssetHandle {
    pub fn new(id: u64) -> Self {
        Self(id)
    }

    pub fn generate() -> Self {
        Self(NEXT_ID.fetch_add(1, Ordering::Relaxed))
    }

    pub fn id(&self) -> u64 {
        self.0
    }
}