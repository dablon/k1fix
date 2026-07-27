//! File-backed mesh loader/store adapter.

use std::path::Path;

use crate::application::ports::{Fs, MeshLoader, MeshStore};
use crate::domain::error::Result;
use crate::domain::mesh::IndexedMesh;
use crate::infrastructure::fs::StdFsAdapter;
use crate::infrastructure::io::{read_mesh_with_fs, write_mesh_with_fs};

/// Mesh loader/store over an injectable [`Fs`].
#[derive(Debug, Clone, Copy)]
pub struct FileMeshRepository<F: Fs> {
    fs: F,
}

impl Default for FileMeshRepository<StdFsAdapter> {
    fn default() -> Self {
        Self { fs: StdFsAdapter }
    }
}

impl<F: Fs> FileMeshRepository<F> {
    /// Create a repository with a custom filesystem.
    #[must_use]
    pub fn new(fs: F) -> Self {
        Self { fs }
    }
}

impl<F: Fs> MeshLoader for FileMeshRepository<F> {
    fn load(&self, path: &Path, tess_tol: f64) -> Result<IndexedMesh> {
        read_mesh_with_fs(&self.fs, path, tess_tol)
    }
}

impl<F: Fs> MeshStore for FileMeshRepository<F> {
    fn store(&self, path: &Path, mesh: &IndexedMesh) -> Result<()> {
        write_mesh_with_fs(&self.fs, path, mesh)
    }
}
