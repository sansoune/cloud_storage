use thiserror::Error;

#[derive(Error, Debug)]
pub enum StorageError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("File not found: {0}")]
    NotFound(String),
    #[error("Storage error: {0}")]
    Storage(String),
}

#[derive(Error, Debug)]
pub enum DaemonError {
    #[error("Daemon task failed: {0}")]
    TaskFailure(String),
    #[error("Daemon already exists")]
    DaemonAlreadyExists,
    #[error("Daemon not found")]
    DaemonNotFound,
}

#[derive(Error, Debug)]
pub enum AppError { // New encompassing error type
    #[error(transparent)] // Use transparent to avoid double wrapping
    Storage(#[from] StorageError),
    #[error(transparent)]
    Daemon(#[from] DaemonError),
    #[error("Other application error: {0}")]
    Other(String), // For other non-storage, non-daemon errors
}

pub type Result<T> = std::result::Result<T, AppError>;