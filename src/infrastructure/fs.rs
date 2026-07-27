//! Real filesystem adapter.

use std::path::Path;

use crate::application::ports::Fs;
use crate::domain::error::Result;

/// Production [`Fs`] backed by `std::fs`.
#[derive(Debug, Default, Clone, Copy)]
pub struct StdFsAdapter;

impl Fs for StdFsAdapter {
    fn read(&self, path: &Path) -> Result<Vec<u8>> {
        Ok(std::fs::read(path)?)
    }

    fn write(&self, path: &Path, bytes: &[u8]) -> Result<()> {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
        Ok(std::fs::write(path, bytes)?)
    }

    fn exists(&self, path: &Path) -> bool {
        path.exists()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn std_fs_round_trip() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("nested").join("file.bin");
        let fs = StdFsAdapter;
        fs.write(&path, b"data").expect("write");
        assert!(fs.exists(&path));
        assert_eq!(fs.read(&path).expect("read"), b"data");
    }
}
