use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("IO error at '{}': {source}", path.display())]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("ELF section '{section}' not found")]
    SectionNotFound { section: String },

    #[error("invalid ELF file: {reason}")]
    InvalidElf { reason: String },

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("verification failed for component '{component}': {reason}")]
    VerificationFailed { component: String, reason: String },

    #[error("defmt error: {reason}")]
    Defmt { reason: String },

    #[error("postcard serialization error: {0}")]
    Postcard(#[from] postcard::Error),

    #[error("espflash failed: {reason}")]
    Espflash { reason: String },

    #[error("package error: {reason}")]
    Package { reason: String },
}

pub type Result<T> = std::result::Result<T, Error>;
