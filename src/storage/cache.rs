use lru::LruCache;
use std::num::NonZeroUsize;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

pub struct CacheManager {
    cache: Arc<Mutex<LruCache<Uuid, Vec<u8>>>>,
}

impl CacheManager {
    pub fn new(cache_size: usize) -> Self {
        Self {
            cache: Arc::new(Mutex::new(LruCache::new(NonZeroUsize::new(cache_size).unwrap())))
        }
    }

    pub async fn get(&self, id: &Uuid) -> Option<Vec<u8>> {
        let mut cache = self.cache.lock().await;
        cache.get(id).cloned()
    }

    pub async fn put(&self, id: Uuid, data: Vec<u8>) {
        let mut cache = self.cache.lock().await;
        cache.put(id, data);
    }

    pub async fn invalidate(&self, id: &Uuid) {
        let mut cache = self.cache.lock().await;
        cache.pop(id);
    }
    
}