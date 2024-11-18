#[cfg(test)]
mod tests {
    use chunk::DEFAULT_CHUNK_SIZE;
    use cloud_storage::*;
    use storage::disk::{DiskStorage, StorageBackend};
    use tempfile::TempDir;

    /// Helper to initialize DiskStorage with a temporary directory
    async fn create_test_storage() -> (DiskStorage, TempDir) {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let path = temp_dir.path();
        let storage = DiskStorage::new(path).await.expect("can't create disk storage");
        // storage.
        (storage, temp_dir)
    }

    /// Test storing and retrieving a file
    #[tokio::test]
    async fn test_store_and_retrieve_file() {
        let (storage, _temp_dir) = create_test_storage().await;

        // Test data
        let file_name = "test.txt";
        let file_data = b"Hello, world!";

        // Store file
        let metadata = storage
            .store_file(file_name, file_data)
            .await
            .expect("Failed to store file");

        assert_eq!(metadata.name, file_name);
        assert_eq!(metadata.size, file_data.len() as u64);

        // Retrieve file
        let retrieved_data = storage
            .get_file(&metadata.id)
            .await
            .expect("Failed to retrieve file");

        assert_eq!(retrieved_data, file_data);
    }

    /// Test file deletion
    #[tokio::test]
    async fn test_delete_file() {
        let (storage, _temp_dir) = create_test_storage().await;

        // Store file
        let file_name = "test.txt";
        let file_data = b"Temporary file data.";
        let metadata = storage
            .store_file(file_name, file_data)
            .await
            .expect("Failed to store file");

        // Delete file
        storage
            .delete_file(&metadata.id)
            .await
            .expect("Failed to delete file");

        // Try retrieving the deleted file
        let result = storage.get_file(&metadata.id).await;
        assert!(result.is_err());
    }

    /// Test chunk handling
    #[tokio::test]
    async fn test_chunk_handling() {
        let (storage, _temp_dir) = create_test_storage().await;

        let large_data = vec![0u8; DEFAULT_CHUNK_SIZE * 4]; // 10 KB data
        let metadata = storage
            .store_file("large_file", &large_data)
            .await
            .expect("Failed to store large file");

        // Verify chunking
        assert!(metadata.chunk_ids.len() > 1);

        // Verify retrieving large data
        let retrieved_data = storage
            .get_file(&metadata.id)
            .await
            .expect("Failed to retrieve large file");
        assert_eq!(retrieved_data, large_data);
    }

    /// Test compression and encryption
    #[tokio::test]
    async fn test_compression_and_encryption() {
        let (mut storage, _temp_dir) = create_test_storage().await;
        storage = storage
            .with_encryption([1; 32])
            .with_cache(100);

        let file_data = b"Sensitive data.";
        let file_name = "secure.txt";

        // Store file
        let metadata = storage
            .store_file(file_name, file_data)
            .await
            .expect("Failed to store file with encryption and compression");

        // Retrieve and check file integrity
        let retrieved_data = storage
            .get_file(&metadata.id)
            .await
            .expect("Failed to retrieve encrypted and compressed file");
        assert_eq!(retrieved_data, file_data);
    }

    /// Test listing files
    #[tokio::test]
    async fn test_list_files() {
        let (storage, _temp_dir) = create_test_storage().await;

        // Add files
        storage
            .store_file("file1.txt", b"Content of file1")
            .await
            .expect("Failed to store file1");
        storage
            .store_file("file2.txt", b"Content of file2")
            .await
            .expect("Failed to store file2");

        // List files
        let files = storage.list_files().await.expect("Failed to list files");

        assert_eq!(files.len(), 2);
        let file_names: Vec<_> = files.into_iter().map(|f| f.name).collect();
        assert!(file_names.contains(&"file1.txt".to_string()));
        assert!(file_names.contains(&"file2.txt".to_string()));
    }
}
