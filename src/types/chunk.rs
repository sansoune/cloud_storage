use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct ChunkId(pub Uuid);

#[derive(Debug, Clone, Serialize, Deserialize, )]
pub struct Chunk {
    pub id: ChunkId,
    pub data: Vec<u8>,
    pub checksum: String,
    pub size: usize,
}