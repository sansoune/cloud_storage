use std::{collections::{HashMap, HashSet}, path::{Path, PathBuf}};
use tokio::fs;
use async_trait::async_trait;
use crate::{chunk::{ChunkManager, FileChunker}, crypto::encryption::EncryptionConfig};
use crate::{Result, StorageError, Chunk, ChunkId, FileType, FileTypeDetector, FileMetadata};
use sha2::{Sha256, Digest};
use uuid::Uuid;
use chrono::Utc;

use super::{cache::CacheManager, compression::CompressionManager};

#[async_trait]
pub trait StorageBackend: Send + Sync {
    async fn store_file(&self, name: &str, data: &[u8]) -> Result<FileMetadata>;
    async fn get_file(&self, id: &Uuid) -> Result<Vec<u8>>;
    async fn delete_file(&self, id: &Uuid) -> Result<()>;
}

pub struct DiskStorage {
    base_path: PathBuf,
    metadata_path: PathBuf,
    chunks_path: PathBuf,
    chunker: FileChunker,
    encryption: Option<EncryptionConfig>,
    cache: Option<CacheManager>,
    compression: Option<CompressionManager>,
}

impl DiskStorage {
    pub async fn new<P: AsRef<Path>>(base_path: P) -> Result<Self> {
        let base_path = base_path.as_ref().to_owned();
        let metadata_path = base_path.join("metadata");
        let chunks_path = base_path.join("chunks");

        fs::create_dir_all(&base_path).await.unwrap();
        fs::create_dir_all(&metadata_path).await.unwrap();
        fs::create_dir_all(&chunks_path).await?;

        let chunker = FileChunker::new(ChunkManager::default());
        Ok(Self {base_path, metadata_path, chunks_path, chunker, encryption: None, cache: None, compression: None} )
    }

    pub fn with_encryption(mut self, key: [u8; 32]) -> Self {
        self.encryption = Some(EncryptionConfig::new(key));
        self
    }

    pub fn with_cache(mut self, cache_size: usize) -> Self {
        self.cache = Some(CacheManager::new(cache_size));
        self
    }

    pub fn with_compression(mut self, enabled: bool) -> Self {
        self.compression = Some(CompressionManager::new(enabled));
        self
    }


    fn get_chunk_path(&self, chunk_id: &ChunkId) -> PathBuf {
        self.chunks_path.join(chunk_id.0.to_string())
    }

    async fn store_chunks(&self, chunks: Vec<Chunk>) -> Result<Vec<ChunkId>> {
        let mut chunk_ids = Vec::new();
        
        
        for chunk in chunks {
            let chunk_path = self.get_chunk_path(&chunk.id);
            fs::write(&chunk_path, &chunk.data).await?;
            chunk_ids.push(chunk.id);
        }
        
        Ok(chunk_ids)
    }

    async fn process_file_by_type(&self, file_type: FileType, data: &[u8]) -> Result<Vec<u8>> {
        match file_type {
            FileType::Image(_) => {
                // Here you could add image processing logic
                // For example, resizing, compression, format conversion
                Ok(data.to_vec())
            },
            FileType::Document(_) => {
                // Document processing logic
                // For example, text extraction, metadata parsing
                self.process_data(data).await
            },
            FileType::Video(_) => {
                // Video processing logic
                // For example, thumbnail generation, transcoding
                Ok(data.to_vec())
            },
            FileType::Audio(_) => {
                // Audio processing logic
                // For example, format conversion, metadata extraction
                Ok(data.to_vec())
            },
            FileType::Unknown => self.process_data(data).await,
        }
    }

    async fn deprocess_file_by_type(&self, file_type: FileType, data: &[u8]) -> Result<Vec<u8>> {
        match file_type {
            FileType::Image(_) => {
                // Here you could add image deprocessing logic
                // For example, resizing, compression, format conversion
                Ok(data.to_vec())
            },
            FileType::Document(_) => {
                // Document deprocessing logic
                // For example, text extraction, metadata parsing
                self.process_data(data).await
            },
            FileType::Video(_) => {
                // Video deprocessing logic
                // For example, thumbnail generation, transcoding
                Ok(data.to_vec())
            },
            FileType::Audio(_) => {
                // Audio deprocessing logic
                // For example, format conversion, metadata extraction
                Ok(data.to_vec())
            },
            FileType::Unknown => self.deprocess_data(data).await,
        }
    }

    async fn process_data(&self, data: &[u8]) -> Result<Vec<u8>> {
        let compressed_data = if let Some(compression) = &self.compression {
            compression.compress(data)?
        }else {
            data.to_vec()
        };

        let encrypted_data = if let Some(encryption) = &self.encryption {
            encryption.encrypt(&compressed_data)?
        }else {
            compressed_data
        };

        Ok(encrypted_data)
    }

    pub async fn deprocess_data(&self, data: &[u8]) -> Result<Vec<u8>> {
        let decrypted_data = if let Some(encryption) = &self.encryption {
            encryption.decrypt(data)?
        } else {
            data.to_vec()
        };

        let decompressed_data = if let Some(compression) = &self.compression {
            compression.decompress(&decrypted_data)?
        } else {
            decrypted_data
        };

        Ok(decompressed_data)
    }
    
    async fn is_chunk_used_by_others(&self, chunk_id: &ChunkId, current_file_id: &Uuid) -> Result<bool> {
        let mut entries = fs::read_dir(&self.metadata_path).await?;
        while let Some(entry) = entries.next_entry().await? {
            if entry.file_type().await?.is_file() {
                if let Some(ext) = entry.path().extension() {
                    if ext == "json" {
                        let metadata_content = fs::read_to_string(entry.path()).await?;
                        let metadata: FileMetadata = serde_json::from_str(&metadata_content)
                            .map_err(|e| StorageError::Storage(format!("Failed to parse metadata: {}", e)))?;
                        
                        // Skip the current file being deleted
                        if metadata.id != *current_file_id && metadata.chunk_ids.contains(chunk_id) {
                            return Ok(true);
                        }
                    }
                }
            }
        }
        Ok(false)
    }
    
    async fn cleanup_orphaned_chunks(&self) -> Result<()> {
        // Get all existing chunk files
        let mut chunk_files = HashSet::new();
        let mut entries = fs::read_dir(&self.chunks_path).await?;
        while let Some(entry) = entries.next_entry().await? {
            if let Some(file_name) = entry.file_name().to_str() {
                chunk_files.insert(file_name.to_string());
            }
        }

        // Get all chunks referenced in metadata
        let mut referenced_chunks = HashSet::new();
        let mut metadata_entries = fs::read_dir(&self.metadata_path).await?;
        while let Some(entry) = metadata_entries.next_entry().await? {
            if let Some(ext) = entry.path().extension() {
                if ext == "json" {
                    let metadata_content = fs::read_to_string(entry.path()).await?;
                    if let Ok(metadata) = serde_json::from_str::<FileMetadata>(&metadata_content) {
                        for chunk_id in metadata.chunk_ids {
                            referenced_chunks.insert(chunk_id.0.to_string());
                        }
                    }
                }
            }
        }

        // Delete orphaned chunks
        for chunk_file in chunk_files {
            if !referenced_chunks.contains(&chunk_file) {
                let chunk_path = self.chunks_path.join(&chunk_file);
                if let Err(e) = fs::remove_file(&chunk_path).await {
                    eprintln!("Failed to delete orphaned chunk {}: {}", chunk_file, e);
                }
            }
        }

        Ok(())
    }

    fn get_metadata_path(&self, id: &Uuid) -> PathBuf {
        self.metadata_path.join(format!("{}.json", id))
    }

    fn calculate_checksum(data: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data);
        format!("{:x}", hasher.finalize())
    }

    async fn update_name_index(&self, name: &str, id: &Uuid) -> Result<()> {
        let index_path = self.base_path.join("name_to_id.json");

        let mut index: HashMap<String, Uuid> = if index_path.exists() {
            let content = fs::read_to_string(&index_path).await?;
            serde_json::from_str(&content).unwrap_or_default()
        }else {
            HashMap::new()
        };

        index.insert(name.to_string(), *id);
        let updated_index = serde_json::to_string(&index).unwrap();
        fs::write(index_path, updated_index).await?;
        Ok(())
    }

    pub async fn list_files(&self) -> Result<Vec<FileMetadata>> {
        let metadata_dir = self.base_path.join("metadata");
        let mut files = Vec::new();

        let mut entries = fs::read_dir(&metadata_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            if entry.file_type().await?.is_file() {
                if let Some(ext) = entry.path().extension() {
                    if ext == "json" {
                        let metadata_content = fs::read_to_string(entry.path()).await?;
                        let metadata: FileMetadata = serde_json::from_str(&metadata_content)
                            .map_err(|e| StorageError::Storage(format!("Failed to parse metadata: {}", e)))?;
                        files.push(metadata);
                    }
                }
            }
        }

        Ok(files)
    }
}

#[async_trait]
impl StorageBackend for DiskStorage {
    async fn store_file(&self, name: &str, data: &[u8]) -> Result<FileMetadata> {
        let id = Uuid::new_v4();
        let file_type = FileTypeDetector::detect(data);
        
        let final_data = self.process_file_by_type(file_type.clone(), data).await?;

        

        let chunks = self.chunker.chunk_data(&final_data);
        let chunk_ids = self.store_chunks(chunks).await?;

        // Create and store metadata
        let metadata = FileMetadata {
            id,
            name: name.to_string(),
            size: final_data.len() as u64,
            created_at: Utc::now(),
            modified_at: Utc::now(),
            checksum: Self::calculate_checksum(&final_data),
            file_type,
            chunk_ids
        };

        // Write metadata to file
        let metadata_json = serde_json::to_string(&metadata)
            .map_err(|e| StorageError::Storage(e.to_string()))?;
        fs::write(self.get_metadata_path(&id), metadata_json).await?;

        self.update_name_index(name, &id).await?;

        if let Some(cache) = &self.cache {
            cache.put(id, final_data.clone()).await;
        }

        Ok(metadata)
    }

    async fn get_file(&self, id: &Uuid) -> Result<Vec<u8>> {
        let metadata_path = self.get_metadata_path(id);
        
        if !metadata_path.exists() {
            return Err(StorageError::NotFound(id.to_string()));
        }

        let metadata_content = fs::read_to_string(&metadata_path).await?;
        let metadata: FileMetadata = serde_json::from_str(&metadata_content)
            .map_err(|e| StorageError::Storage(format!("Failed to parse metadata: {}", e)))?;

        // Read and combine chunks
        let mut data = Vec::new();
        for chunk_id in metadata.chunk_ids {
            let chunk_path = self.get_chunk_path(&chunk_id);
            let chunk_data = fs::read(&chunk_path).await?;
            data.extend(chunk_data);
        }

        let final_data = self.deprocess_file_by_type(metadata.file_type, &data).await?;

        if let Some(cache) = &self.cache {
            cache.put(*id, final_data.clone()).await; // Store the data in cache
        }

        Ok(final_data)
    }

    async fn delete_file(&self, id: &Uuid) -> Result<()> {
        let metadata_path = self.get_metadata_path(id);
        
        // Check if file exists
        if !metadata_path.exists() {
            return Err(StorageError::NotFound(id.to_string()));
        }

        // Read metadata to get chunk information
        let metadata_content = fs::read_to_string(&metadata_path).await?;
        let metadata: FileMetadata = serde_json::from_str(&metadata_content)
            .map_err(|e| StorageError::Storage(format!("Failed to parse metadata: {}", e)))?;

        // Delete chunks that aren't used by other files
        for chunk_id in &metadata.chunk_ids {
            if !self.is_chunk_used_by_others(chunk_id, id).await? {
                let chunk_path = self.get_chunk_path(chunk_id);
                if chunk_path.exists() {
                    if let Err(e) = fs::remove_file(&chunk_path).await {
                        eprintln!("Failed to delete chunk {}: {}", chunk_id.0, e);
                    }
                }
            }
        }

        // Delete metadata file
        fs::remove_file(&metadata_path).await?;

        // Clean up any orphaned chunks
        self.cleanup_orphaned_chunks().await?;

        if let Some(cache) = &self.cache {
            cache.invalidate(id).await; // Invalidate cache entry
        }

        Ok(())
    }
}

