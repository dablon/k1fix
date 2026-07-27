//! Spatial hashing and BVH helpers.

use nalgebra::{Point3, Vector3};
use parry3d_f64::bounding_volume::Aabb as ParryAabb;
use parry3d_f64::math::{Isometry, Point as ParryPoint, Real, Vector as ParryVector};
use parry3d_f64::query::{Ray, RayCast};
use parry3d_f64::shape::{TriMesh, Triangle};
use rustc_hash::FxHashMap;

use crate::domain::constants::{WELD_TOL_FLOOR_MM, WELD_TOL_RATIO};
use crate::domain::mesh::IndexedMesh;

/// Compute the weld tolerance from the mesh diagonal.
#[must_use]
pub fn weld_tolerance(mesh: &IndexedMesh) -> f64 {
    let diag = mesh.aabb().diagonal();
    if !diag.is_finite() || diag == 0.0 {
        return WELD_TOL_FLOOR_MM;
    }
    WELD_TOL_FLOOR_MM.max(WELD_TOL_RATIO * diag)
}

/// Spatial hash used to weld near-duplicate vertices.
#[derive(Debug)]
pub struct SpatialHash {
    cell: f64,
    buckets: FxHashMap<(i64, i64, i64), Vec<u32>>,
}

impl SpatialHash {
    /// Create an empty hash with the given cell size.
    #[must_use]
    pub fn new(cell: f64) -> Self {
        Self {
            cell: cell.max(WELD_TOL_FLOOR_MM),
            buckets: FxHashMap::default(),
        }
    }

    fn key(&self, p: Point3<f64>) -> (i64, i64, i64) {
        let inv = 1.0 / self.cell;
        (
            (p.x * inv).floor() as i64,
            (p.y * inv).floor() as i64,
            (p.z * inv).floor() as i64,
        )
    }

    /// Insert vertex index `id` at position `p`.
    pub fn insert(&mut self, id: u32, p: Point3<f64>) {
        self.buckets.entry(self.key(p)).or_default().push(id);
    }

    /// Find an existing vertex within `tol` of `p`, if any.
    #[must_use]
    pub fn find_near(&self, p: Point3<f64>, verts: &[Point3<f64>], tol: f64) -> Option<u32> {
        let base = self.key(p);
        let tol2 = tol * tol;
        for dx in -1..=1 {
            for dy in -1..=1 {
                for dz in -1..=1 {
                    let key = (base.0 + dx, base.1 + dy, base.2 + dz);
                    if let Some(ids) = self.buckets.get(&key) {
                        for &id in ids {
                            if (verts[id as usize] - p).norm_squared() <= tol2 {
                                return Some(id);
                            }
                        }
                    }
                }
            }
        }
        None
    }
}

/// Weld duplicate vertices, remapping triangle indices. Returns the welded mesh.
#[must_use]
pub fn weld_vertices(mesh: &IndexedMesh) -> IndexedMesh {
    let tol = weld_tolerance(mesh);
    let mut hash = SpatialHash::new(tol);
    let mut new_verts: Vec<Point3<f64>> = Vec::new();
    let mut remap = vec![0u32; mesh.verts.len()];

    for (i, &p) in mesh.verts.iter().enumerate() {
        if let Some(existing) = hash.find_near(p, &new_verts, tol) {
            remap[i] = existing;
        } else {
            let id = new_verts.len() as u32;
            new_verts.push(p);
            hash.insert(id, p);
            remap[i] = id;
        }
    }

    let tris = mesh
        .tris
        .iter()
        .map(|t| {
            [
                remap[t[0] as usize],
                remap[t[1] as usize],
                remap[t[2] as usize],
            ]
        })
        .collect();
    IndexedMesh::from_parts(new_verts, tris)
}

/// Acceleration structure wrapping a `parry` triangle mesh.
#[derive(Debug)]
pub struct MeshBvh {
    trimesh: TriMesh,
}

impl MeshBvh {
    /// Build a BVH for `mesh`.
    ///
    /// # Errors
    /// Returns an error when the mesh has no triangles.
    pub fn build(mesh: &IndexedMesh) -> crate::domain::error::Result<Self> {
        if mesh.tris.is_empty() {
            return Err(crate::domain::error::K1FixError::InvalidMesh(
                "cannot build BVH for empty mesh".into(),
            ));
        }
        let vertices: Vec<ParryPoint<Real>> = mesh
            .verts
            .iter()
            .map(|v| ParryPoint::new(v.x, v.y, v.z))
            .collect();
        let indices: Vec<[u32; 3]> = mesh.tris.clone();
        let trimesh = TriMesh::new(vertices, indices);
        Ok(Self { trimesh })
    }

    /// Cast a ray and return the closest hit distance, if any.
    #[must_use]
    pub fn cast_ray(&self, origin: Point3<f64>, dir: Vector3<f64>, max_toi: f64) -> Option<f64> {
        let ray = Ray::new(
            ParryPoint::new(origin.x, origin.y, origin.z),
            ParryVector::new(dir.x, dir.y, dir.z),
        );
        self.trimesh
            .cast_ray(&Isometry::identity(), &ray, max_toi, true)
    }

    /// Axis-aligned bounds of the trimesh.
    #[must_use]
    pub fn aabb(&self) -> ParryAabb {
        *self.trimesh.local_aabb()
    }
}

/// Test whether two triangles intersect (excluding shared vertices of a mesh edge).
#[must_use]
pub fn triangles_intersect(
    a0: Point3<f64>,
    a1: Point3<f64>,
    a2: Point3<f64>,
    b0: Point3<f64>,
    b1: Point3<f64>,
    b2: Point3<f64>,
) -> bool {
    let ta = Triangle::new(
        ParryPoint::new(a0.x, a0.y, a0.z),
        ParryPoint::new(a1.x, a1.y, a1.z),
        ParryPoint::new(a2.x, a2.y, a2.z),
    );
    let tb = Triangle::new(
        ParryPoint::new(b0.x, b0.y, b0.z),
        ParryPoint::new(b1.x, b1.y, b1.z),
        ParryPoint::new(b2.x, b2.y, b2.z),
    );
    parry3d_f64::query::intersection_test(&Isometry::identity(), &ta, &Isometry::identity(), &tb)
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weld_collapses_duplicates() {
        let mesh = IndexedMesh::from_parts(
            vec![
                Point3::origin(),
                Point3::new(1e-9, 0.0, 0.0),
                Point3::new(0.0, 1.0, 0.0),
                Point3::new(1.0, 0.0, 0.0),
            ],
            vec![[0, 2, 3], [1, 2, 3]],
        );
        let welded = weld_vertices(&mesh);
        assert!(welded.verts.len() < mesh.verts.len());
    }

    #[test]
    fn bvh_ray_hits_cube() {
        let cube = IndexedMesh::unit_cube();
        let bvh = MeshBvh::build(&cube).expect("bvh");
        let hit = bvh.cast_ray(
            Point3::new(0.5, 0.5, -1.0),
            Vector3::new(0.0, 0.0, 1.0),
            10.0,
        );
        assert!(hit.is_some());
        assert!((hit.expect("hit") - 1.0).abs() < 1e-6);
    }

    #[test]
    fn bvh_empty_mesh_errors() {
        let empty = IndexedMesh::new();
        assert!(MeshBvh::build(&empty).is_err());
    }

    #[test]
    fn weld_tolerance_nonzero() {
        let cube = IndexedMesh::unit_cube();
        assert!(weld_tolerance(&cube) > 0.0);
        let empty = IndexedMesh::new();
        assert!(weld_tolerance(&empty) > 0.0);
    }

    #[test]
    fn triangle_intersection_smoke() {
        assert!(triangles_intersect(
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
            Point3::new(0.2, 0.2, -1.0),
            Point3::new(0.2, 0.2, 1.0),
            Point3::new(1.0, 1.0, 0.0),
        ));
    }
}
