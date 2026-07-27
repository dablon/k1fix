//! STL reader / writer via `stl_io`.

use std::io::Cursor;

use nalgebra::Point3;

use crate::application::ports::{MeshReader, MeshWriter};
use crate::domain::error::{K1FixError, Result};
use crate::domain::mesh::IndexedMesh;

/// STL mesh reader.
#[derive(Debug, Default, Clone, Copy)]
pub struct StlReader;

impl MeshReader for StlReader {
    fn read_bytes(&self, bytes: &[u8]) -> Result<IndexedMesh> {
        let mut cursor = Cursor::new(bytes);
        let mesh = stl_io::read_stl(&mut cursor)
            .map_err(|e| K1FixError::Parse(format!("STL parse failed: {e}")))?;
        let verts = mesh
            .vertices
            .iter()
            .map(|v| Point3::new(f64::from(v[0]), f64::from(v[1]), f64::from(v[2])))
            .collect();
        let tris = mesh
            .faces
            .iter()
            .map(|f| {
                [
                    f.vertices[0] as u32,
                    f.vertices[1] as u32,
                    f.vertices[2] as u32,
                ]
            })
            .collect();
        Ok(IndexedMesh::from_parts(verts, tris))
    }
}

/// Binary STL writer.
#[derive(Debug, Default, Clone, Copy)]
pub struct StlWriter;

impl MeshWriter for StlWriter {
    fn write_bytes(&self, mesh: &IndexedMesh) -> Result<Vec<u8>> {
        let mut triangles = Vec::with_capacity(mesh.tris.len());
        for tri in &mesh.tris {
            let a = mesh.verts[tri[0] as usize];
            let b = mesh.verts[tri[1] as usize];
            let c = mesh.verts[tri[2] as usize];
            let n = (b - a).cross(&(c - a));
            let normal = if n.norm() > 0.0 {
                n.normalize()
            } else {
                nalgebra::Vector3::z()
            };
            triangles.push(stl_io::Triangle {
                normal: stl_io::Normal::new([normal.x as f32, normal.y as f32, normal.z as f32]),
                vertices: [
                    stl_io::Vertex::new([a.x as f32, a.y as f32, a.z as f32]),
                    stl_io::Vertex::new([b.x as f32, b.y as f32, b.z as f32]),
                    stl_io::Vertex::new([c.x as f32, c.y as f32, c.z as f32]),
                ],
            });
        }
        let mut out = Vec::new();
        stl_io::write_stl(&mut out, triangles.iter())
            .map_err(|e| K1FixError::Parse(format!("STL write failed: {e}")))?;
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::ports::{MeshReader, MeshWriter};

    #[test]
    fn stl_round_trip_cube() {
        let cube = IndexedMesh::unit_cube();
        let bytes = StlWriter.write_bytes(&cube).expect("write");
        let back = StlReader.read_bytes(&bytes).expect("read");
        assert_eq!(back.triangle_count(), cube.triangle_count());
        assert!((back.volume() - cube.volume()).abs() < 1e-3);
    }

    #[test]
    fn stl_bad_bytes_error() {
        assert!(StlReader.read_bytes(b"not an stl").is_err());
    }
}
