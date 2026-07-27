//! Typed errors for `k1fix-core`.

use std::path::PathBuf;

use thiserror::Error;

/// Domain error type for the k1fix library.
#[derive(Debug, Error)]
pub enum K1FixError {
    /// Filesystem failure.
    #[error("filesystem error: {0}")]
    Io(#[from] std::io::Error),

    /// Unsupported or unknown input format.
    #[error("unsupported format for `{path}`")]
    UnsupportedFormat { path: PathBuf },

    /// Mesh parse / decode failure.
    #[error("failed to parse mesh: {0}")]
    Parse(String),

    /// STEP feature is not available or the file cannot be tessellated.
    #[error("STEP input error: {0}")]
    Step(String),

    /// Unknown printer profile id.
    #[error("unknown printer profile `{0}`")]
    UnknownProfile(String),

    /// Mesh cannot be fitted into the build volume.
    #[error("mesh does not fit build volume: {detail}")]
    DoesNotFit { detail: String },

    /// Invalid mesh for the requested operation.
    #[error("invalid mesh: {0}")]
    InvalidMesh(String),

    /// Serialization failure.
    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),

    /// TOML profile parse failure.
    #[error("profile parse error: {0}")]
    Toml(#[from] toml::de::Error),

    /// Generic zipper / archive failure.
    #[error("archive error: {0}")]
    Archive(String),
}

/// Convenient result alias.
pub type Result<T> = std::result::Result<T, K1FixError>;
