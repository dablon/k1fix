//! Indexed triangle mesh and geometric queries.

use nalgebra::{Point3, Vector3};

use crate::domain::constants::DEGENERATE_AREA_EPS;

/// Axis-aligned bounding box.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Aabb {
    /// Minimum corner.
    pub min: Point3<f64>,
    /// Maximum corner.
    pub max: Point3<f64>,
}

impl Aabb {
    /// Empty / inverted AABB sentinel.
    #[must_use]
    pub fn empty() -> Self {
        Self {
            min: Point3::new(f64::INFINITY, f64::INFINITY, f64::INFINITY),
            max: Point3::new(f64::NEG_INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY),
        }
    }

    /// Expand to include `p`.
    pub fn include_point(&mut self, p: Point3<f64>) {
        self.min.x = self.min.x.min(p.x);
        self.min.y = self.min.y.min(p.y);
        self.min.z = self.min.z.min(p.z);
        self.max.x = self.max.x.max(p.x);
        self.max.y = self.max.y.max(p.y);
        self.max.z = self.max.z.max(p.z);
    }

    /// Size along each axis.
    #[must_use]
    pub fn extents(&self) -> Vector3<f64> {
        self.max - self.min
    }

    /// Diagonal length.
    #[must_use]
    pub fn diagonal(&self) -> f64 {
        self.extents().norm()
    }

    /// Centre point.
    #[must_use]
    pub fn center(&self) -> Point3<f64> {
        Point3::from((self.min.coords + self.max.coords) * 0.5)
    }

    /// Volume of the box (may be zero / negative if empty).
    #[must_use]
    pub fn volume(&self) -> f64 {
        let e = self.extents();
        e.x * e.y * e.z
    }
}

/// Indexed triangle mesh with `f64` vertices.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct IndexedMesh {
    /// Vertex positions.
    pub verts: Vec<Point3<f64>>,
    /// Triangle indices into `verts`.
    pub tris: Vec<[u32; 3]>,
}

impl IndexedMesh {
    /// Create an empty mesh.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Create from vertices and triangles.
    #[must_use]
    pub fn from_parts(verts: Vec<Point3<f64>>, tris: Vec<[u32; 3]>) -> Self {
        Self { verts, tris }
    }

    /// Number of triangles.
    #[must_use]
    pub fn triangle_count(&self) -> usize {
        self.tris.len()
    }

    /// Number of vertices.
    #[must_use]
    pub fn vertex_count(&self) -> usize {
        self.verts.len()
    }

    /// Compute the axis-aligned bounding box.
    #[must_use]
    pub fn aabb(&self) -> Aabb {
        let mut box_ = Aabb::empty();
        for v in &self.verts {
            box_.include_point(*v);
        }
        box_
    }

    /// Triangle area.
    #[must_use]
    pub fn triangle_area(&self, tri: [u32; 3]) -> f64 {
        let a = self.verts[tri[0] as usize];
        let b = self.verts[tri[1] as usize];
        let c = self.verts[tri[2] as usize];
        ((b - a).cross(&(c - a))).norm() * 0.5
    }

    /// Triangle normal (unnormalized cross product).
    #[must_use]
    pub fn triangle_normal(&self, tri: [u32; 3]) -> Vector3<f64> {
        let a = self.verts[tri[0] as usize];
        let b = self.verts[tri[1] as usize];
        let c = self.verts[tri[2] as usize];
        (b - a).cross(&(c - a))
    }

    /// Triangle centroid.
    #[must_use]
    pub fn triangle_centroid(&self, tri: [u32; 3]) -> Point3<f64> {
        let a = self.verts[tri[0] as usize];
        let b = self.verts[tri[1] as usize];
        let c = self.verts[tri[2] as usize];
        Point3::from((a.coords + b.coords + c.coords) / 3.0)
    }

    /// Total surface area.
    #[must_use]
    pub fn surface_area(&self) -> f64 {
        self.tris.iter().map(|t| self.triangle_area(*t)).sum()
    }

    /// Signed volume via divergence theorem (positive for outward CCW winding).
    #[must_use]
    pub fn signed_volume(&self) -> f64 {
        let mut vol = 0.0;
        for tri in &self.tris {
            let a = self.verts[tri[0] as usize];
            let b = self.verts[tri[1] as usize];
            let c = self.verts[tri[2] as usize];
            vol += a.coords.dot(&b.coords.cross(&c.coords));
        }
        vol / 6.0
    }

    /// Absolute volume.
    #[must_use]
    pub fn volume(&self) -> f64 {
        self.signed_volume().abs()
    }

    /// Translate all vertices by `delta`.
    pub fn translate(&mut self, delta: Vector3<f64>) {
        for v in &mut self.verts {
            *v += delta;
        }
    }

    /// Uniform scale about the origin.
    pub fn scale_uniform(&mut self, factor: f64) {
        for v in &mut self.verts {
            *v = Point3::from(v.coords * factor);
        }
    }

    /// Apply a 3x3 linear transform (rotation/scale) about the origin.
    pub fn transform_linear(&mut self, matrix: &nalgebra::Matrix3<f64>) {
        for v in &mut self.verts {
            *v = Point3::from(matrix * v.coords);
        }
    }

    /// Flip winding of every triangle.
    pub fn flip_winding(&mut self) {
        for tri in &mut self.tris {
            tri.swap(1, 2);
        }
    }

    /// Returns true when the triangle has repeated indices or near-zero area.
    #[must_use]
    pub fn is_degenerate(&self, tri: [u32; 3]) -> bool {
        tri[0] == tri[1]
            || tri[1] == tri[2]
            || tri[0] == tri[2]
            || self.triangle_area(tri) < DEGENERATE_AREA_EPS
    }

    /// Unit cube `[0,1]³` as a closed manifold mesh (12 triangles).
    #[must_use]
    pub fn unit_cube() -> Self {
        let verts = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(1.0, 1.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
            Point3::new(0.0, 0.0, 1.0),
            Point3::new(1.0, 0.0, 1.0),
            Point3::new(1.0, 1.0, 1.0),
            Point3::new(0.0, 1.0, 1.0),
        ];
        let tris = vec![
            // bottom (outward -Z)
            [0, 2, 1],
            [0, 3, 2],
            // top (outward +Z)
            [4, 5, 6],
            [4, 6, 7],
            // front y=0 (outward -Y)
            [0, 1, 5],
            [0, 5, 4],
            // right x=1 (outward +X)
            [1, 2, 6],
            [1, 6, 5],
            // back y=1 (outward +Y)
            [2, 3, 7],
            [2, 7, 6],
            // left x=0 (outward -X)
            [3, 0, 4],
            [3, 4, 7],
        ];
        Self { verts, tris }
    }

    /// Axis-aligned box of the given size starting at the origin.
    #[must_use]
    pub fn box_mesh(sx: f64, sy: f64, sz: f64) -> Self {
        let mut m = Self::unit_cube();
        for v in &mut m.verts {
            v.x *= sx;
            v.y *= sy;
            v.z *= sz;
        }
        m
    }

    /// Thin tray matching the Küchenablage footprint for autofit tests.
    #[must_use]
    pub fn kitchen_tray() -> Self {
        Self::box_mesh(221.64, 239.64, 40.0)
    }
}

/// Bounding-box trait (ISP).
pub trait Bounds {
    /// Axis-aligned bounds.
    fn aabb(&self) -> Aabb;
}

impl Bounds for IndexedMesh {
    fn aabb(&self) -> Aabb {
        IndexedMesh::aabb(self)
    }
}

/// Volume trait (ISP).
pub trait Volume {
    /// Signed volume.
    fn signed_volume(&self) -> f64;
}

impl Volume for IndexedMesh {
    fn signed_volume(&self) -> f64 {
        IndexedMesh::signed_volume(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_cube_volume_and_area() {
        let cube = IndexedMesh::unit_cube();
        assert!((cube.signed_volume() - 1.0).abs() < 1e-9);
        assert!((cube.surface_area() - 6.0).abs() < 1e-9);
        assert_eq!(cube.triangle_count(), 12);
        assert_eq!(cube.vertex_count(), 8);
    }

    #[test]
    fn aabb_extents() {
        let cube = IndexedMesh::unit_cube();
        let box_ = cube.aabb();
        assert_eq!(box_.extents(), Vector3::new(1.0, 1.0, 1.0));
        assert!((box_.diagonal() - 3.0_f64.sqrt()).abs() < 1e-12);
        assert_eq!(box_.center(), Point3::new(0.5, 0.5, 0.5));
    }

    #[test]
    fn translate_and_scale() {
        let mut cube = IndexedMesh::unit_cube();
        cube.translate(Vector3::new(1.0, 2.0, 3.0));
        assert_eq!(cube.aabb().min, Point3::new(1.0, 2.0, 3.0));
        cube.scale_uniform(2.0);
        assert_eq!(cube.aabb().extents(), Vector3::new(2.0, 2.0, 2.0));
    }

    #[test]
    fn flip_winding_negates_volume() {
        let mut cube = IndexedMesh::unit_cube();
        let v = cube.signed_volume();
        cube.flip_winding();
        assert!((cube.signed_volume() + v).abs() < 1e-12);
    }

    #[test]
    fn degenerate_detection() {
        let mesh = IndexedMesh::from_parts(
            vec![
                Point3::origin(),
                Point3::new(1.0, 0.0, 0.0),
                Point3::new(0.5, 0.0, 0.0),
            ],
            vec![[0, 1, 2], [0, 0, 1]],
        );
        assert!(mesh.is_degenerate([0, 1, 2]));
        assert!(mesh.is_degenerate([0, 0, 1]));
    }

    #[test]
    fn kitchen_tray_dimensions() {
        let tray = IndexedMesh::kitchen_tray();
        let e = tray.aabb().extents();
        assert!((e.x - 221.64).abs() < 1e-9);
        assert!((e.y - 239.64).abs() < 1e-9);
        assert!((e.z - 40.0).abs() < 1e-9);
    }

    #[test]
    fn empty_aabb() {
        let a = Aabb::empty();
        assert!(a.volume().is_infinite() || a.volume() < 0.0 || a.min.x.is_infinite());
        let mesh = IndexedMesh::new();
        assert_eq!(mesh.triangle_count(), 0);
        assert_eq!(mesh.surface_area(), 0.0);
        assert_eq!(mesh.signed_volume(), 0.0);
    }

    #[test]
    fn transform_linear_identity() {
        let mut cube = IndexedMesh::unit_cube();
        cube.transform_linear(&nalgebra::Matrix3::identity());
        assert!((cube.signed_volume() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn bounds_and_volume_traits() {
        let cube = IndexedMesh::unit_cube();
        let _: &dyn Bounds = &cube;
        let _: &dyn Volume = &cube;
        assert!((Bounds::aabb(&cube).diagonal() - 3.0_f64.sqrt()).abs() < 1e-12);
        assert!((Volume::signed_volume(&cube) - 1.0).abs() < 1e-9);
    }
}
