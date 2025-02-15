use flate2::{write::GzEncoder, read::GzDecoder, Compression};
use std::io::prelude::*;
use crate::{AppError, Result};

pub struct CompressionManager {
    enabled: bool,
}

impl CompressionManager {
    pub fn new(enabled: bool) -> Self {
        Self {
            enabled,
        }
    }

    pub fn compress(&self, data: &[u8]) -> Result<Vec<u8>> {
        if !self.enabled {
            return  Ok(data.to_vec());
        }

        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(data).map_err(|e| AppError::Storage(crate::StorageError::Storage(e.to_string())));
        encoder.finish().map_err(|e| crate::AppError::Storage(crate::StorageError::Storage(e.to_string())))

    }

    pub fn decompress(&self, data: &[u8]) -> Result<Vec<u8>> {
        if !self.enabled {
            return Ok(data.to_vec());
        }

        let mut decoder = GzDecoder::new(data);
        let mut decompressed = Vec::new();
        decoder.read_to_end(&mut decompressed).map_err(|e| AppError::Storage(crate::StorageError::Storage(e.to_string())));
        Ok(decompressed)
    }
}