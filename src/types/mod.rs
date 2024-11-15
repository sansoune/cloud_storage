mod metadata;
mod chunk;
mod file;

pub use chunk::{Chunk, ChunkId};
pub use file::{FileType, FileTypeDetector, ImageType, DocumentType, VideoType, AudioType};
pub use metadata::FileMetadata;

