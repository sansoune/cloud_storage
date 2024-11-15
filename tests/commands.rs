use chunk::DEFAULT_CHUNK_SIZE;
use cloud_storage::*;
use storage::disk::{DiskStorage, StorageBackend};
use tempfile::TempDir;
use cloud_storage::{FileType, ImageType};
use std::sync::Arc;

#[tokio::test]
async fn test_disk_storage_basic_operations() {
    let temp_dir = TempDir::new().unwrap();
    let storage = DiskStorage::new(temp_dir.path()).await.unwrap();

    // Test file storage
    let test_data = b"Hello, World!".to_vec();
    let metadata = storage.store_file("test.txt", &test_data).await.unwrap();

    // Verify metadata
    assert_eq!(metadata.name, "test.txt");
    assert_eq!(metadata.size, test_data.len() as u64);
    
    // Test file retrieval
    let retrieved_data = storage.get_file(&metadata.id).await.unwrap();
    assert_eq!(retrieved_data, test_data);

    // Test file deletion
    storage.delete_file(&metadata.id).await.unwrap();
    assert!(storage.get_file(&metadata.id).await.is_err());
}

#[tokio::test]
async fn test_chunking_large_file() {
    let temp_dir = TempDir::new().unwrap();
    let storage = DiskStorage::new(temp_dir.path()).await.unwrap();

    // Create a large test file that will span multiple chunks
    let large_data = vec![0u8; DEFAULT_CHUNK_SIZE * 2 + 500]; // 2.5 chunks
    let metadata = storage.store_file("large.bin", &large_data).await.unwrap();

    // Verify chunk count
    assert_eq!(metadata.chunk_ids.len(), 3);

    // Verify data integrity
    let retrieved_data = storage.get_file(&metadata.id).await.unwrap();
    assert_eq!(retrieved_data, large_data);
}

#[tokio::test]
async fn test_file_type_detection() {
    let temp_dir = TempDir::new().unwrap();
    let storage = DiskStorage::new(temp_dir.path()).await.unwrap();

    // PNG file header
    let png_data = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
    let metadata = storage.store_file("test.png", &png_data).await.unwrap();

    match metadata.file_type {
        FileType::Image(ImageType::Png) => (),
        _ => panic!("Failed to detect PNG file type"),
    }
}

#[tokio::test]
async fn test_deduplication() {
    let temp_dir = TempDir::new().unwrap();
    let storage = DiskStorage::new(temp_dir.path()).await.unwrap();

    // Store the same data twice
    let test_data = b"Hello, World!".to_vec();
    let metadata1 = storage.store_file("file1.txt", &test_data).await.unwrap();
    let metadata2 = storage.store_file("file2.txt", &test_data).await.unwrap();

    // Verify checksums match
    assert_eq!(metadata1.checksum, metadata2.checksum);
}

#[tokio::test]
async fn test_name_based_operations() {
    let temp_dir = TempDir::new().unwrap();
    let storage = DiskStorage::new(temp_dir.path()).await.unwrap();

    // Store a file
    let test_data = b"Hello, World!".to_vec();
    let _original_metadata = storage.store_file("test.txt", &test_data).await.unwrap();

    // List files and verify
    let files = storage.list_files().await.unwrap();
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].name, "test.txt");
}

#[tokio::test]
async fn test_concurrent_operations() {
    use tokio::task;
    
    let temp_dir = TempDir::new().unwrap();
    let storage = Arc::new(DiskStorage::new(temp_dir.path()).await.unwrap());
    
    let mut handles = vec![];
    
    // Spawn multiple concurrent uploads
    for i in 0..5 {
        let storage = storage.clone();
        let handle = task::spawn(async move {
            let data = format!("Data {}", i).into_bytes();
            storage.store_file(&format!("file{}.txt", i), &data).await
        });
        handles.push(handle);
    }

    // Wait for all operations to complete
    for handle in handles {
        handle.await.unwrap().unwrap();
    }

    // Verify all files were stored
    let files = storage.list_files().await.unwrap();
    assert_eq!(files.len(), 5);
}

// Test utilities
// #[cfg(test)]
// mod test_utils {
//     pub fn create_test_file(size: usize) -> Vec<u8> {
//         let mut data = Vec::with_capacity(size);
//         for i in 0..size {
//             data.push((i % 256) as u8);
//         }
//         data
//     }
// }

// Property-based tests using proptest
#[cfg(test)]
mod property_tests {
    use cloud_storage::chunk::{ChunkManager, FileChunker, DEFAULT_CHUNK_SIZE};
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn test_chunk_size_properties(data in prop::collection::vec(any::<u8>(), 0..1024*1024)) {
            let chunker = FileChunker::new(ChunkManager::default());
            let chunks = chunker.chunk_data(&data);
            
            // Properties that should hold
            assert!(chunks.iter().all(|chunk| chunk.size <= DEFAULT_CHUNK_SIZE));
            let total_size: usize = chunks.iter().map(|chunk| chunk.size).sum();
            assert_eq!(total_size, data.len());
        }
    }
}