use storage_engine::storage::disk::{DiskStorage, StorageBackend};
use storage_engine::FileMetadata;
use storage_engine::Result;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct StorageManager {
    inner: Arc<Mutex<DiskStorage>>
}

impl StorageManager {
    pub async fn new(storage_path: &str) -> Result<Self> {
        let storage = DiskStorage::new(storage_path)
        .await?
        .with_encryption([0u8; 32])
        .with_cache(100)
        .with_compression(true);

        Ok(Self { inner: Arc::new(Mutex::new(storage)) })
    }

    pub fn get_arc_mutex(&self) -> Arc<Mutex<DiskStorage>> {
        Arc::clone(&self.inner)
    }

    pub async fn upload_file(&self, filename: &str, data: &[u8]) -> Result<FileMetadata> {
        let storage = self.inner.lock().await;
        storage.store_file(filename, data).await
    }

    pub async fn download_file(&self, file_id: &uuid::Uuid) -> Result<Vec<u8>> {
        let storage = self.inner.lock().await;
        storage.get_file(file_id).await
    }

    pub async fn list_files(&self) -> Result<Vec<FileMetadata>> {
        let storage = self.inner.lock().await;
        storage.list_files().await
    }

    pub async fn delete_file(&self, file_id: &uuid::Uuid) -> Result<()> {
        let storage = self.inner.lock().await;
        storage.delete_file(file_id).await
    }
}