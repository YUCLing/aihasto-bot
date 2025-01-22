use dashmap::DashMap;
use std::sync::Arc;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct CacheKey {
    pub id: u64,
    pub name: String,
}

struct CacheEntry {
    value: String,
    frequency: usize,
}

struct IdCache {
    data: DashMap<String, CacheEntry>,
    capacity: usize,
}

impl IdCache {
    pub fn new(capacity: usize) -> Self {
        IdCache {
            data: DashMap::new(),
            capacity,
        }
    }

    pub fn get(&self, name: &str) -> Option<String> {
        if let Some(mut entry) = self.data.get_mut(name) {
            entry.frequency += 1;
            Some(entry.value().value.clone())
        } else {
            None
        }
    }

    pub fn insert(&self, name: String, value: String) {
        if self.data.len() >= self.capacity && !self.data.contains_key(&name) {
            let mut min_freq = usize::MAX;
            let mut lfu_key = None;
            for entry in self.data.iter() {
                if entry.value().frequency < min_freq {
                    min_freq = entry.value().frequency;
                    lfu_key = Some(entry.key().clone());
                }
            }
            if let Some(k) = lfu_key {
                self.data.remove(&k);
            }
        }

        if self.data.contains_key(&name) {
            if let Some(mut entry) = self.data.get_mut(&name) {
                entry.value = value;
            }
        } else {
            self.data.insert(
                name,
                CacheEntry {
                    value,
                    frequency: 0,
                },
            );
        }
    }

    pub fn invalidate(&self, name: &str) {
        self.data.remove(name);
    }
}


pub struct GuildSettingsCache {
    id_caches: Arc<DashMap<u64, Arc<IdCache>>>,
    capacity: usize,
}

impl GuildSettingsCache {
    pub fn new(capacity: usize) -> Self {
        GuildSettingsCache {
            id_caches: Arc::new(DashMap::new()),
            capacity,
        }
    }

    fn get_id_cache(&self, id: u64) -> Arc<IdCache> {
        if let Some(cache) = self.id_caches.get(&id) {
            cache.clone()
        } else {
            let new_cache = Arc::new(IdCache::new(self.capacity));
            self.id_caches.insert(id, new_cache.clone());
            new_cache
        }
    }

    pub fn get(&self, key: &CacheKey) -> Option<String> {
        let id_cache = self.get_id_cache(key.id);
        id_cache.get(&key.name)
    }

    pub fn insert(&self, key: CacheKey, value: String) {
        let id_cache = self.get_id_cache(key.id);
        id_cache.insert(key.name, value);
    }

    pub fn invalidate(&self, key: &CacheKey) {
        if let Some(id_cache) = self.id_caches.get(&key.id) {
            id_cache.invalidate(&key.name);
            if id_cache.data.is_empty() {
                std::mem::drop(id_cache);
                self.id_caches.remove(&key.id);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_insert_and_get() {
        let cache = GuildSettingsCache::new(2);
        let key1 = CacheKey { id: 1, name: "setting1".to_string() };
        let key2 = CacheKey { id: 1, name: "setting2".to_string() };

        cache.insert(key1.clone(), "value1".to_string());
        cache.insert(key2.clone(), "value2".to_string());

        assert_eq!(cache.get(&key1), Some("value1".to_string()));
        assert_eq!(cache.get(&key2), Some("value2".to_string()));
    }

    #[test]
    fn test_lfu_eviction_same_id() {
        let cache = GuildSettingsCache::new(2);
        let key1 = CacheKey { id: 1, name: "setting1".to_string() };
        let key2 = CacheKey { id: 1, name: "setting2".to_string() };
        let key3 = CacheKey { id: 1, name: "setting3".to_string() };

        cache.insert(key1.clone(), "value1".to_string()); // freq 0
        cache.insert(key2.clone(), "value2".to_string()); // freq 0
        cache.get(&key1); // freq 1
        cache.insert(key3.clone(), "value3".to_string()); // capacity reached, setting2 should be evicted (LFU)

        assert_eq!(cache.get(&key1), Some("value1".to_string()));
        assert_eq!(cache.get(&key2), None); // evicted
        assert_eq!(cache.get(&key3), Some("value3".to_string()));
    }

    #[test]
    fn test_explicit_lfu_eviction_order() {
        let cache = GuildSettingsCache::new(3); // Capacity 3 for easier control
        let key1 = CacheKey { id: 1, name: "setting1".to_string() };
        let key2 = CacheKey { id: 1, name: "setting2".to_string() };
        let key3 = CacheKey { id: 1, name: "setting3".to_string() };
        let key4 = CacheKey { id: 1, name: "setting4".to_string() };

        cache.insert(key1.clone(), "value1".to_string()); // freq 0
        cache.insert(key2.clone(), "value2".to_string()); // freq 0
        cache.insert(key3.clone(), "value3".to_string()); // freq 0

        cache.get(&key1); // freq 1
        cache.get(&key1); // freq 2
        cache.get(&key2); // freq 1

        // Now frequencies are: key1: 2, key2: 1, key3: 0

        cache.insert(key4.clone(), "value4".to_string()); // Capacity reached, key3 (freq 0) should be evicted

        assert_eq!(cache.get(&key3), None); // key3 should be evicted
        assert_eq!(cache.get(&key1), Some("value1".to_string()));
        assert_eq!(cache.get(&key2), Some("value2".to_string()));
        assert_eq!(cache.get(&key4), Some("value4".to_string()));
    }

    #[test]
    fn test_invalidate() {
        let cache = GuildSettingsCache::new(2);
        let key1 = CacheKey { id: 1, name: "setting1".to_string() };
        let key2 = CacheKey { id: 1, name: "setting2".to_string() };

        cache.insert(key1.clone(), "value1".to_string());
        cache.insert(key2.clone(), "value2".to_string());

        cache.invalidate(&key1);

        assert_eq!(cache.get(&key1), None);
        assert_eq!(cache.get(&key2), Some("value2".to_string()));
    }

    #[test]
    fn test_invalidate_and_reinsert() {
        let cache = GuildSettingsCache::new(2);
        let key1 = CacheKey { id: 1, name: "setting1".to_string() };

        cache.insert(key1.clone(), "value1".to_string());
        cache.invalidate(&key1);
        assert_eq!(cache.get(&key1), None);

        cache.insert(key1.clone(), "value_new".to_string());
        assert_eq!(cache.get(&key1), Some("value_new".to_string()));
    }

    #[test]
    fn test_multithreaded_access() {
        let cache = Arc::new(GuildSettingsCache::new(5)); // Shared cache
        let mut handles = vec![];

        for i in 0..10 {
            let cache_clone = cache.clone();
            handles.push(thread::spawn(move || {
                let id = (i % 2) as u64 + 1; // Two IDs: 1 and 2
                for j in 0..10 {
                    let key = CacheKey { id, name: format!("setting{}", j) };
                    cache_clone.insert(key.clone(), format!("value_{}_{}", id, j));
                    cache_clone.get(&key); // Simulate access
                    thread::sleep(Duration::from_millis(5)); // Introduce some delay
                }
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }

        // After threads finish, check cache state (basic verification)
        for id in 1..=2 {
            for j in 0..10 {
                let key = CacheKey { id, name: format!("setting{}", j) };
                if cache.get(&key).is_some() {
                    // Basic check - some entries should likely remain, but eviction might happen
                    println!("Entry for id={} name={} still in cache (or was re-inserted)", id, j);
                }
            }
        }
        // More detailed assertions in a multithreaded scenario would require more deterministic control
        // or synchronization to predict the exact final state due to LFU eviction and concurrency.
        // However, this test at least checks for panics and basic concurrent operation.
    }
}
