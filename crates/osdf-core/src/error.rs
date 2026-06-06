use thiserror::Error;

#[derive(Debug, Error)]
pub enum OsdfError {
    #[error("container: {0}")]
    Container(String),
    #[error("manifest: {0}")]
    Manifest(String),
    #[error("integrity: {0}")]
    Integrity(String),
    #[error("revision: {0}")]
    Revision(String),
    #[error("signature: {0}")]
    Signature(String),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("zip: {0}")]
    Zip(#[from] zip::result::ZipError),
}

pub type Result<T> = std::result::Result<T, OsdfError>;
