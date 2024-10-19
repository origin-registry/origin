use std::collections::HashMap;
use std::sync::{Arc, Weak};
use tokio::sync::{Mutex as AsyncMutex, RwLock};

#[derive(Debug, Default)]
pub struct InMemoryRwLock {
    locks: AsyncMutex<HashMap<String, Weak<RwLock<()>>>>,
}

impl InMemoryRwLock {
    pub fn new() -> Self {
        Self {
            locks: AsyncMutex::new(HashMap::new()),
        }
    }

    pub async fn read_lock(&self, key: String) -> InMemoryReadLockGuard {
        let lock = self.get_lock_for_key(&key).await;
        lock.clone().read_owned().await
    }

    pub async fn write_lock(&self, key: String) -> InMemoryWriteLockGuard {
        let lock = self.get_lock_for_key(&key).await;
        lock.clone().write_owned().await
    }

    async fn get_lock_for_key(&self, key: &str) -> Arc<RwLock<()>> {
        let mut locks = self.locks.lock().await;
        if let Some(weak_lock) = locks.get(key) {
            if let Some(lock) = weak_lock.upgrade() {
                return lock;
            } else {
                locks.remove(key);
            }
        }
        let lock = Arc::new(RwLock::new(()));
        locks.insert(key.to_string(), Arc::downgrade(&lock));
        lock
    }
}

pub type InMemoryReadLockGuard = tokio::sync::OwnedRwLockReadGuard<()>;
pub type InMemoryWriteLockGuard = tokio::sync::OwnedRwLockWriteGuard<()>;