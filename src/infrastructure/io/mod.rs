//! Mesh format adapters (STL, 3MF, STEP).

mod step;
mod stl;
mod threemf;

pub use step::{mesh_to_faceted_step, StepReader};
pub use stl::{StlReader, StlWriter};
pub use threemf::{ThreeMfReader, ThreeMfWriter};

use std::path::Path;

use crate::application::ports::{Fs, MeshReader, MeshWriter};
use crate::domain::error::{K1FixError, Result};
use crate::domain::mesh::IndexedMesh;
use crate::infrastructure::fs::StdFsAdapter;

/// Detected mesh file format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeshFormat {
    /// Stereolithography.
    Stl,
    /// 3D Manufacturing Format.
    ThreeMf,
    /// STEP CAD.
    Step,
}

impl MeshFormat {
    /// Infer format from a file path extension.
    #[must_use]
    pub fn from_path(path: &Path) -> Option<Self> {
        let ext = path.extension()?.to_str()?.to_ascii_lowercase();
        match ext.as_str() {
            "stl" => Some(Self::Stl),
            "3mf" => Some(Self::ThreeMf),
            "step" | "stp" => Some(Self::Step),
            _ => None,
        }
    }
}

/// Read a mesh from a path using the real filesystem.
///
/// # Errors
/// Propagates IO / parse errors.
pub fn read_mesh_file(path: &Path, tess_tol: f64) -> Result<IndexedMesh> {
    read_mesh_with_fs(&StdFsAdapter, path, tess_tol)
}

/// Read a mesh using an injectable [`Fs`].
///
/// # Errors
/// Propagates IO / parse errors.
pub fn read_mesh_with_fs(fs: &impl Fs, path: &Path, tess_tol: f64) -> Result<IndexedMesh> {
    let format = MeshFormat::from_path(path).ok_or_else(|| K1FixError::UnsupportedFormat {
        path: path.to_path_buf(),
    })?;
    let bytes = fs.read(path)?;
    read_mesh_bytes(format, &bytes, tess_tol)
}

/// Decode mesh bytes for a known format.
///
/// # Errors
/// Propagates parse errors.
pub fn read_mesh_bytes(format: MeshFormat, bytes: &[u8], tess_tol: f64) -> Result<IndexedMesh> {
    match format {
        MeshFormat::Stl => StlReader.read_bytes(bytes),
        MeshFormat::ThreeMf => ThreeMfReader.read_bytes(bytes),
        MeshFormat::Step => StepReader { tess_tol }.read_bytes(bytes),
    }
}

/// Write a mesh to a path; format is inferred from the extension.
///
/// # Errors
/// Propagates IO / encode errors.
pub fn write_mesh_file(path: &Path, mesh: &IndexedMesh) -> Result<()> {
    write_mesh_with_fs(&StdFsAdapter, path, mesh)
}

/// Write a mesh using an injectable [`Fs`].
///
/// # Errors
/// Propagates IO / encode errors.
pub fn write_mesh_with_fs(fs: &impl Fs, path: &Path, mesh: &IndexedMesh) -> Result<()> {
    let format = MeshFormat::from_path(path).ok_or_else(|| K1FixError::UnsupportedFormat {
        path: path.to_path_buf(),
    })?;
    let bytes = write_mesh_bytes(format, mesh)?;
    fs.write(path, &bytes)
}

/// Encode a mesh to bytes.
///
/// # Errors
/// Propagates encode errors. STEP is write-unsupported.
pub fn write_mesh_bytes(format: MeshFormat, mesh: &IndexedMesh) -> Result<Vec<u8>> {
    match format {
        MeshFormat::Stl => StlWriter.write_bytes(mesh),
        MeshFormat::ThreeMf => ThreeMfWriter.write_bytes(mesh),
        MeshFormat::Step => Err(K1FixError::UnsupportedFormat {
            path: Path::new("out.step").to_path_buf(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_from_path() {
        assert_eq!(
            MeshFormat::from_path(Path::new("a.STL")),
            Some(MeshFormat::Stl)
        );
        assert_eq!(
            MeshFormat::from_path(Path::new("a.3mf")),
            Some(MeshFormat::ThreeMf)
        );
        assert_eq!(
            MeshFormat::from_path(Path::new("a.stp")),
            Some(MeshFormat::Step)
        );
        assert_eq!(MeshFormat::from_path(Path::new("a.obj")), None);
    }
}
