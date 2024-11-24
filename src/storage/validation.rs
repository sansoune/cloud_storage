use crate::{Result, StorageError, FileMetadata};
use tokio::fs;
use std::path::PathBuf;

pub struct ValidationManager {
    base_path: PathBuf,
}

impl ValidationManager {
    pub fn new(base_path: PathBuf) -> Self {
        Self {base_path}
    }

    pub async fn validate_file(&self, metadata: &FileMetadata) -> Result<()> {
        for chunk_id in &metadata.chunk_ids {
            let chunk_path = self.base_path.join("chunks").join(chunk_id.0.to_string());
            if !chunk_path.exists() {
                return Err(StorageError::Storage(format!("chunk {} is missing", chunk_id.0)));
            }
        }

        let mut total_size = 0;
        for chunk_id in &metadata.chunk_ids {
            let chunk_path = self.base_path.join("chunks").join(chunk_id.0.to_string());
            let metadata = fs::metadata(chunk_path).await?;
            total_size += metadata.len();
        }

        if total_size as u64 != metadata.size {
            return Err(StorageError::Storage(format!("File size mismatch. Expected: {}, Got: {}", metadata.size, total_size)));
        }

        Ok(())
    }
}