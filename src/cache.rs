use std::collections::HashMap;
use std::hash::Hash;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Values kept `SECS` seconds, so repeated lookups don't hammer the Discord and Hypixel APIs
pub struct TtlCache<K, V, const SECS: u64>(Mutex<HashMap<K, (Instant, V)>>);

impl<K, V, const SECS: u64> Default for TtlCache<K, V, SECS> {
    fn default() -> Self {
        Self(Mutex::default())
    }
}

impl<K: Hash + Eq, V: Clone, const SECS: u64> TtlCache<K, V, SECS> {
    pub fn get(&self, key: &K) -> Option<V> {
        let cache = self.0.lock().unwrap_or_else(|e| e.into_inner());
        cache
            .get(key)
            .filter(|(stored_at, _)| stored_at.elapsed() < Duration::from_secs(SECS))
            .map(|(_, value)| value.clone())
    }

    pub fn insert(&self, key: K, value: V) {
        let mut cache = self.0.lock().unwrap_or_else(|e| e.into_inner());
        cache.retain(|_, (stored_at, _)| stored_at.elapsed() < Duration::from_secs(SECS));
        cache.insert(key, (Instant::now(), value));
    }

    pub fn remove(&self, key: &K) {
        let mut cache = self.0.lock().unwrap_or_else(|e| e.into_inner());
        cache.remove(key);
    }
}
