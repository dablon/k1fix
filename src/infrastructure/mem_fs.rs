//! In-memory filesystem fake for tests (DIP).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};

use crate::application::ports::Fs;
use crate::domain::error::{K1FixError, Result};

/// Thread-safe in-memory [`Fs`].
#[derive(Clone, Default)]
pub struct MemFs {
    files: Arc<Mutex<HashMap<PathBuf, Vec<u8>>>>,
    fail_read: Arc<Mutex<bool>>,
    fail_write: Arc<Mutex<bool>>,
}

impl MemFs {
    /// Create an empty filesystem.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Force the next reads to fail.
    pub fn set_fail_read(&self, fail: bool) {
        *self.flag(&self.fail_read) = fail;
    }

    /// Force the next writes to fail.
    pub fn set_fail_write(&self, fail: bool) {
        *self.flag(&self.fail_write) = fail;
    }

    fn files(&self) -> MutexGuard<'_, HashMap<PathBuf, Vec<u8>>> {
        self.files.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn flag<'a>(&self, m: &'a Mutex<bool>) -> MutexGuard<'a, bool> {
        m.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

impl Fs for MemFs {
    fn read(&self, path: &Path) -> Result<Vec<u8>> {
        if *self.flag(&self.fail_read) {
            return Err(K1FixError::Io(std::io::Error::other("forced read failure")));
        }
        self.files().get(path).cloned().ok_or_else(|| {
            K1FixError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "missing",
            ))
        })
    }

    fn write(&self, path: &Path, bytes: &[u8]) -> Result<()> {
        if *self.flag(&self.fail_write) {
            return Err(K1FixError::Io(std::io::Error::other("forced write failure")));
        }
        self.files().insert(path.to_path_buf(), bytes.to_vec());
        Ok(())
    }

    fn exists(&self, path: &Path) -> bool {
        self.files().contains_key(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mem_fs_round_trip_and_failures() {
        let fs = MemFs::new();
        let path = Path::new("a.stl");
        assert!(fs.write(path, b"hello").is_ok());
        assert!(fs.exists(path));
        assert_eq!(fs.read(path).expect("read"), b"hello");
        fs.set_fail_write(true);
        assert!(fs.write(path, b"x").is_err());
        fs.set_fail_write(false);
        fs.set_fail_read(true);
        assert!(fs.read(path).is_err());
    }
}
