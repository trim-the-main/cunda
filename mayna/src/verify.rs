use std::fs;
use std::path::Path;

use sha2::{Digest, Sha256};

use crate::error::{Error, Result};
use crate::manifest::Component;

/// Compute the SHA256 hex digest of a file.
pub fn sha256_file(path: &Path) -> Result<String> {
    let bytes = fs::read(path).map_err(|source| Error::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let hash = Sha256::digest(&bytes);
    let hex: String = hash.iter().map(|b| format!("{b:02x}")).collect();
    Ok(hex)
}

/// Verify a component file matches its expected size and checksum.
pub fn verify_component(path: &Path, expected: &Component) -> Result<()> {
    let metadata = fs::metadata(path).map_err(|source| Error::Io {
        path: path.to_path_buf(),
        source,
    })?;

    if metadata.len() != expected.size {
        return Err(Error::VerificationFailed {
            component: expected.file.clone(),
            reason: format!(
                "size mismatch: expected {} bytes, got {} bytes",
                expected.size,
                metadata.len()
            ),
        });
    }

    let actual_sha256 = sha256_file(path)?;
    if actual_sha256 != expected.sha256 {
        return Err(Error::VerificationFailed {
            component: expected.file.clone(),
            reason: format!(
                "SHA256 mismatch: expected {}, got {actual_sha256}",
                expected.sha256
            ),
        });
    }

    Ok(())
}
