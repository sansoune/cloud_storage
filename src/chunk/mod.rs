    use sha2::{Sha256, Digest};
    use crate::{Chunk, ChunkId};
    use uuid::Uuid;

    pub struct ChunkManager {
        chunk_size: usize,
    }

    pub const DEFAULT_CHUNK_SIZE: usize = 1024 * 1024;

    impl Default for ChunkManager {
        fn default() -> Self {
            Self {
                chunk_size: DEFAULT_CHUNK_SIZE,
            }
        }
    }

    pub struct FileChunker {
        config: ChunkManager,
    }

    impl FileChunker {
        pub fn new(config: ChunkManager) -> Self {
            Self { config }
        }

        pub fn chunk_data(&self, data: &[u8]) -> Vec<Chunk> {
            let mut chunks = Vec::new();
            let mut position = 0;

            while position < data.len() {
                let end = (position + self.config.chunk_size).min(data.len());
                let chunk_data = &data[position..end];
                
                chunks.push(Chunk {
                    id: ChunkId(Uuid::new_v4()),
                    data: chunk_data.to_vec(),
                    checksum: self.calculate_checksum(chunk_data),
                    size: chunk_data.len(),
                });

                position = end;
            }

            chunks
        }

        fn calculate_checksum(&self, data: &[u8]) -> String {
            let mut hasher = Sha256::new();
            hasher.update(data);
            format!("{:x}", hasher.finalize())
        }
    }
