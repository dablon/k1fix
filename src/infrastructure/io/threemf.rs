//! 3MF reader / writer via the `threemf` crate.

use std::io::Cursor;

use nalgebra::Point3;
use threemf::model::{Mesh as ThreeMesh, Triangle, Triangles, Vertex, Vertices};

use crate::application::ports::{MeshReader, MeshWriter};
use crate::domain::error::{K1FixError, Result};
use crate::domain::mesh::IndexedMesh;

/// 3MF mesh reader.
#[derive(Debug, Default, Clone, Copy)]
pub struct ThreeMfReader;

impl MeshReader for ThreeMfReader {
    fn read_bytes(&self, bytes: &[u8]) -> Result<IndexedMesh> {
        let mut cursor = Cursor::new(bytes);
        let models =
            threemf::read(&mut cursor).map_err(|e| K1FixError::Parse(format!("3MF read: {e}")))?;
        let mesh = models
            .iter()
            .flat_map(|m| m.resources.object.iter())
            .find_map(|o| o.mesh.as_ref())
            .ok_or_else(|| K1FixError::Parse("3MF contains no mesh objects".into()))?;

        let verts = mesh
            .vertices
            .vertex
            .iter()
            .map(|v| Point3::new(v.x, v.y, v.z))
            .collect();
        let tris = mesh
            .triangles
            .triangle
            .iter()
            .map(|t| [t.v1 as u32, t.v2 as u32, t.v3 as u32])
            .collect();
        Ok(IndexedMesh::from_parts(verts, tris))
    }
}

/// 3MF mesh writer.
#[derive(Debug, Default, Clone, Copy)]
pub struct ThreeMfWriter;

impl MeshWriter for ThreeMfWriter {
    fn write_bytes(&self, mesh: &IndexedMesh) -> Result<Vec<u8>> {
        let three = ThreeMesh {
            vertices: Vertices {
                vertex: mesh
                    .verts
                    .iter()
                    .map(|v| Vertex {
                        x: v.x,
                        y: v.y,
                        z: v.z,
                    })
                    .collect(),
            },
            triangles: Triangles {
                triangle: mesh
                    .tris
                    .iter()
                    .map(|t| Triangle {
                        v1: t[0] as usize,
                        v2: t[1] as usize,
                        v3: t[2] as usize,
                    })
                    .collect(),
            },
        };
        let mut buf = Cursor::new(Vec::new());
        threemf::write(&mut buf, three)
            .map_err(|e| K1FixError::Parse(format!("3MF write: {e}")))?;
        Ok(buf.into_inner())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn threemf_round_trip_cube() {
        let cube = IndexedMesh::unit_cube();
        let bytes = ThreeMfWriter.write_bytes(&cube).expect("write");
        let back = ThreeMfReader.read_bytes(&bytes).expect("read");
        assert_eq!(back.triangle_count(), cube.triangle_count());
        assert!((back.volume() - 1.0).abs() < 1e-3);
    }
}
