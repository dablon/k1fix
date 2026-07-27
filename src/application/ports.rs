//! Outbound / inbound ports (Dependency Inversion).
//! Implementations live in [`crate::infrastructure`].

use std::path::Path;
use std::time::{Duration, SystemTime};

use crate::domain::error::Result;
use crate::domain::mesh::IndexedMesh;

/// Reads bytes from a path (injectable for tests).
pub trait Fs {
    /// Read the full contents of `path`.
    fn read(&self, path: &Path) -> Result<Vec<u8>>;
    /// Write `bytes` to `path`.
    fn write(&self, path: &Path, bytes: &[u8]) -> Result<()>;
    /// Returns true when `path` exists.
    fn exists(&self, path: &Path) -> bool;
}

/// Loads a mesh from a path (format inferred by adapter).
pub trait MeshLoader {
    /// Load and decode a mesh.
    fn load(&self, path: &Path, tess_tol: f64) -> Result<IndexedMesh>;
}

/// Persists a mesh to a path (format inferred by adapter).
pub trait MeshStore {
    /// Encode and write a mesh.
    fn store(&self, path: &Path, mesh: &IndexedMesh) -> Result<()>;
}

/// Progress reporting sink.
pub trait ProgressSink {
    /// Report a human-readable progress message.
    fn message(&self, msg: &str);
    /// Report fractional progress in `[0.0, 1.0]`.
    fn fraction(&self, value: f64);
}

/// Silent progress sink (usable from tests and headless runs).
#[derive(Debug, Default, Clone, Copy)]
pub struct NullProgress;

impl ProgressSink for NullProgress {
    fn message(&self, _msg: &str) {}
    fn fraction(&self, _value: f64) {}
}

/// Clock abstraction for reproducible tests.
pub trait Clock {
    /// Current wall-clock time.
    fn now(&self) -> SystemTime;
    /// Elapsed duration marker for timings.
    fn elapsed(&self) -> Duration;
}

/// System clock.
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> SystemTime {
        SystemTime::now()
    }

    fn elapsed(&self) -> Duration {
        Duration::from_secs(0)
    }
}

/// Reads a mesh from a byte buffer.
pub trait MeshReader {
    /// Decode a mesh from raw bytes.
    fn read_bytes(&self, bytes: &[u8]) -> Result<IndexedMesh>;
}

/// Writes a mesh to a byte buffer.
pub trait MeshWriter {
    /// Encode a mesh into bytes.
    fn write_bytes(&self, mesh: &IndexedMesh) -> Result<Vec<u8>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn null_progress_and_clock_cover() {
        let p = NullProgress;
        p.message("hi");
        p.fraction(0.5);
        let c = SystemClock;
        let _ = c.now();
        let _ = c.elapsed();
    }
}
