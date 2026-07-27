//! Auto-fit: orient and place a mesh into a printer build volume.

use nalgebra::{Matrix3, Point2, Rotation3, Vector3};

use crate::domain::constants::{BED_CONTACT_EPS_MM, CONTACT_Z_TOL_MM};
use crate::domain::error::{K1FixError, Result};
use crate::domain::mesh::IndexedMesh;
use crate::domain::profiles::PrinterProfile;

/// Options for auto-fit.
#[derive(Debug, Clone)]
pub struct AutofitOptions {
    /// Bed margin in millimetres.
    pub margin: f64,
    /// Allow uniform scale when rotation alone is insufficient.
    pub scale_to_fit: bool,
}

impl Default for AutofitOptions {
    fn default() -> Self {
        Self {
            margin: 3.0,
            scale_to_fit: false,
        }
    }
}

/// Result of a successful auto-fit.
#[derive(Debug, Clone)]
pub struct AutofitResult {
    /// Uniform scale applied (1.0 if none).
    pub scale: f64,
    /// Human summary.
    pub summary: String,
}

/// Orientation candidate generator (OCP).
pub trait OrientationGenerator: Send + Sync {
    /// Produce candidate rotation matrices (applied about origin).
    fn candidates(&self, mesh: &IndexedMesh) -> Vec<Matrix3<f64>>;
}

/// 24 axis-aligned orientations (6 faces down × 4 spins).
#[derive(Debug, Default, Clone, Copy)]
pub struct AxisAligned24;

impl OrientationGenerator for AxisAligned24 {
    fn candidates(&self, _mesh: &IndexedMesh) -> Vec<Matrix3<f64>> {
        let mut out = Vec::with_capacity(24);
        let bases = [
            Matrix3::identity(),
            Rotation3::from_axis_angle(&Vector3::x_axis(), std::f64::consts::FRAC_PI_2)
                .into_inner(),
            Rotation3::from_axis_angle(&Vector3::x_axis(), -std::f64::consts::FRAC_PI_2)
                .into_inner(),
            Rotation3::from_axis_angle(&Vector3::x_axis(), std::f64::consts::PI).into_inner(),
            Rotation3::from_axis_angle(&Vector3::y_axis(), std::f64::consts::FRAC_PI_2)
                .into_inner(),
            Rotation3::from_axis_angle(&Vector3::y_axis(), -std::f64::consts::FRAC_PI_2)
                .into_inner(),
        ];
        for base in bases {
            for k in 0..4 {
                let spin = Rotation3::from_axis_angle(
                    &Vector3::z_axis(),
                    k as f64 * std::f64::consts::FRAC_PI_2,
                )
                .into_inner();
                out.push(spin * base);
            }
        }
        out
    }
}

/// Extra orientations from PCA / OBB face normals.
#[derive(Debug, Default, Clone, Copy)]
pub struct PcaObbFaces;

impl OrientationGenerator for PcaObbFaces {
    fn candidates(&self, mesh: &IndexedMesh) -> Vec<Matrix3<f64>> {
        let axes = pca_axes(mesh);
        let mut out = Vec::new();
        // Map each PCA axis to +Z (down after placing on bed = -Z contact, we put min Z on bed).
        for axis in [axes.column(0), axes.column(1), axes.column(2)] {
            let z = Vector3::new(axis[0], axis[1], axis[2]);
            if let Some(r) = rotation_mapping_z(z) {
                out.push(r);
                out.push(
                    Rotation3::from_axis_angle(&Vector3::z_axis(), std::f64::consts::FRAC_PI_4)
                        .into_inner()
                        * r,
                );
            }
            if let Some(r) = rotation_mapping_z(-z) {
                out.push(r);
            }
        }
        out
    }
}

fn rotation_mapping_z(dir: Vector3<f64>) -> Option<Matrix3<f64>> {
    let d = dir.normalize();
    if d.norm() < 1e-12 {
        return None;
    }
    let z = Vector3::z();
    let axis = d.cross(&z);
    let axis_len = axis.norm();
    if axis_len < 1e-12 {
        if d.dot(&z) > 0.0 {
            return Some(Matrix3::identity());
        }
        return Some(
            Rotation3::from_axis_angle(&Vector3::x_axis(), std::f64::consts::PI).into_inner(),
        );
    }
    let angle = d.angle(&z);
    let unit = nalgebra::Unit::new_normalize(axis);
    Some(Rotation3::from_axis_angle(&unit, angle).into_inner())
}

fn pca_axes(mesh: &IndexedMesh) -> Matrix3<f64> {
    let n = mesh.verts.len().max(1) as f64;
    let mut mean = Vector3::zeros();
    for v in &mesh.verts {
        mean += v.coords;
    }
    mean /= n;
    let mut cov = Matrix3::zeros();
    for v in &mesh.verts {
        let d = v.coords - mean;
        cov += d * d.transpose();
    }
    cov /= n;
    let eig = cov.symmetric_eigen();
    eig.eigenvectors
}

/// Place mesh on bed (min Z = 0) and centre in XY within the bed.
pub fn place_on_bed(mesh: &mut IndexedMesh, profile: &PrinterProfile) {
    let box_ = mesh.aabb();
    let center = box_.center();
    let dx = profile.bed_x_mm * 0.5 - center.x;
    let dy = profile.bed_y_mm * 0.5 - center.y;
    let dz = -box_.min.z;
    mesh.translate(Vector3::new(dx, dy, dz));
}

/// Returns true when the mesh fits the usable volume.
#[must_use]
pub fn fits(mesh: &IndexedMesh, profile: &PrinterProfile, margin: f64) -> bool {
    let e = mesh.aabb().extents();
    let (ux, uy) = profile.usable_bed_area(margin);
    e.x <= ux + 1e-6 && e.y <= uy + 1e-6 && e.z <= profile.build_z_mm + 1e-6
}

/// Score a placed mesh: higher is better.
fn score(mesh: &IndexedMesh, profile: &PrinterProfile, margin: f64) -> Option<(f64, f64, f64)> {
    if !fits(mesh, profile, margin) {
        return None;
    }
    let contact = bed_contact_area(mesh);
    let height = mesh.aabb().extents().z;
    // lexicographic via weighted tuple: contact desc, height asc
    Some((contact, -height, 0.0))
}

fn bed_contact_area(mesh: &IndexedMesh) -> f64 {
    let min_z = mesh.aabb().min.z;
    mesh.tris
        .iter()
        .filter(|t| {
            let c = mesh.triangle_centroid(**t);
            (c.z - min_z).abs() <= CONTACT_Z_TOL_MM && mesh.triangle_normal(**t).z < -1e-6
        })
        .map(|t| mesh.triangle_area(*t))
        .sum()
}

/// Best yaw (radians) so the XY AABB fits a usable bed of size `(ux, uy)`.
///
/// Minimises `max(width, height)` among sampled angles; min-area rectangles are
/// the wrong objective for a square build plate (a 45° spin grows area but shrinks
/// the axis-aligned footprint of a thin part).
#[must_use]
pub fn best_yaw_for_bed(mesh: &IndexedMesh, ux: f64, uy: f64) -> f64 {
    let pts: Vec<Point2<f64>> = mesh.verts.iter().map(|v| Point2::new(v.x, v.y)).collect();
    if pts.is_empty() {
        return 0.0;
    }
    let mut best_angle = 0.0;
    let mut best_key = f64::INFINITY;
    // 1° sampling is enough for print beds; include hull-edge angles too.
    let mut angles: Vec<f64> = (0..180).map(|d| (d as f64).to_radians()).collect();
    let mut hull_pts = pts.clone();
    let hull = convex_hull_2d(&mut hull_pts);
    for i in 0..hull.len() {
        let a = hull[i];
        let b = hull[(i + 1) % hull.len()];
        let edge = b - a;
        angles.push(-edge.y.atan2(edge.x));
        angles.push(-edge.y.atan2(edge.x) + std::f64::consts::FRAC_PI_4);
    }
    for angle in angles {
        let (w, h) = extents_at_angle(&pts, angle);
        let fits_bed = w <= ux + 1e-6 && h <= uy + 1e-6;
        let key = if fits_bed { w.max(h) } else { 1.0e9 + w.max(h) };
        if key < best_key {
            best_key = key;
            best_angle = angle;
        }
    }
    best_angle
}

/// Minimum-area bounding rectangle yaw (radians) for the XY projection.
#[must_use]
pub fn min_area_rect_yaw(mesh: &IndexedMesh) -> f64 {
    let mut pts: Vec<Point2<f64>> = mesh.verts.iter().map(|v| Point2::new(v.x, v.y)).collect();
    if pts.is_empty() {
        return 0.0;
    }
    let hull = convex_hull_2d(&mut pts);
    if hull.len() < 2 {
        return 0.0;
    }
    let mut best_angle = 0.0;
    let mut best_area = f64::INFINITY;
    for i in 0..hull.len() {
        let a = hull[i];
        let b = hull[(i + 1) % hull.len()];
        let edge = b - a;
        let angle = -edge.y.atan2(edge.x);
        let (w, h) = extents_at_angle(&hull, angle);
        let area = w * h;
        if area < best_area {
            best_area = area;
            best_angle = angle;
        }
    }
    best_angle
}

fn extents_at_angle(pts: &[Point2<f64>], angle: f64) -> (f64, f64) {
    let (s, c) = angle.sin_cos();
    let mut min_x = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    for p in pts {
        let x = p.x * c - p.y * s;
        let y = p.x * s + p.y * c;
        min_x = min_x.min(x);
        max_x = max_x.max(x);
        min_y = min_y.min(y);
        max_y = max_y.max(y);
    }
    (max_x - min_x, max_y - min_y)
}

fn convex_hull_2d(points: &mut [Point2<f64>]) -> Vec<Point2<f64>> {
    if points.len() <= 1 {
        return points.to_vec();
    }
    points.sort_by(
        |a, b| match (a.x.partial_cmp(&b.x), a.y.partial_cmp(&b.y)) {
            (Some(o), _) if o != std::cmp::Ordering::Equal => o,
            (_, Some(o)) => o,
            _ => std::cmp::Ordering::Equal,
        },
    );
    let mut lower = Vec::new();
    for &p in points.iter() {
        while lower.len() >= 2 && cross(lower[lower.len() - 2], lower[lower.len() - 1], p) <= 0.0 {
            lower.pop();
        }
        lower.push(p);
    }
    let mut upper = Vec::new();
    for &p in points.iter().rev() {
        while upper.len() >= 2 && cross(upper[upper.len() - 2], upper[upper.len() - 1], p) <= 0.0 {
            upper.pop();
        }
        upper.push(p);
    }
    lower.pop();
    upper.pop();
    lower.extend(upper);
    lower
}

fn cross(o: Point2<f64>, a: Point2<f64>, b: Point2<f64>) -> f64 {
    (a.x - o.x) * (b.y - o.y) - (a.y - o.y) * (b.x - o.x)
}

/// Auto-fit the mesh into the printer volume.
///
/// # Errors
/// Returns [`K1FixError::DoesNotFit`] when no orientation (and optional scale) works.
pub fn autofit(
    mesh: &mut IndexedMesh,
    profile: &PrinterProfile,
    opts: &AutofitOptions,
) -> Result<AutofitResult> {
    let generators: Vec<Box<dyn OrientationGenerator>> =
        vec![Box::new(AxisAligned24), Box::new(PcaObbFaces)];

    let original = mesh.clone();
    let mut best: Option<(IndexedMesh, f64, f64, f64)> = None; // mesh, scale, contact, -height

    for gen in &generators {
        for rot in gen.candidates(&original) {
            let mut candidate = original.clone();
            candidate.transform_linear(&rot);
            let (ux, uy) = profile.usable_bed_area(opts.margin);
            let yaw = best_yaw_for_bed(&candidate, ux, uy);
            let yaw_m = Rotation3::from_axis_angle(&Vector3::z_axis(), yaw).into_inner();
            candidate.transform_linear(&yaw_m);
            place_on_bed(&mut candidate, profile);
            if let Some((contact, neg_h, _)) = score(&candidate, profile, opts.margin) {
                let replace = match &best {
                    None => true,
                    Some((_, _, bc, bh)) => {
                        contact > *bc + 1e-9 || ((contact - *bc).abs() < 1e-9 && neg_h > *bh)
                    }
                };
                if replace {
                    best = Some((candidate, 1.0, contact, neg_h));
                }
            }
        }
    }

    if let Some((fitted, scale, _, _)) = best {
        *mesh = fitted;
        return Ok(AutofitResult {
            scale,
            summary: "oriented and placed on bed".into(),
        });
    }

    if opts.scale_to_fit {
        // Pick the orientation with minimal required uniform scale.
        let mut best_scaled: Option<(IndexedMesh, f64)> = None;
        for gen in &generators {
            for rot in gen.candidates(&original) {
                let mut candidate = original.clone();
                candidate.transform_linear(&rot);
                let (ux, uy) = profile.usable_bed_area(opts.margin);
                let yaw = best_yaw_for_bed(&candidate, ux, uy);
                candidate.transform_linear(
                    &Rotation3::from_axis_angle(&Vector3::z_axis(), yaw).into_inner(),
                );
                let e = candidate.aabb().extents();
                let sx = if e.x > 1e-12 { ux / e.x } else { 1.0 };
                let sy = if e.y > 1e-12 { uy / e.y } else { 1.0 };
                let sz = if e.z > 1e-12 {
                    profile.build_z_mm / e.z
                } else {
                    1.0
                };
                let s = sx.min(sy).min(sz);
                if s <= 0.0 {
                    continue;
                }
                candidate.scale_uniform(s);
                place_on_bed(&mut candidate, profile);
                if fits(&candidate, profile, opts.margin) {
                    let replace = match &best_scaled {
                        None => true,
                        Some((_, bs)) => s > *bs,
                    };
                    if replace {
                        best_scaled = Some((candidate, s));
                    }
                }
            }
        }
        if let Some((fitted, scale)) = best_scaled {
            *mesh = fitted;
            return Ok(AutofitResult {
                scale,
                summary: format!("scaled to {:.2}% and placed on bed", scale * 100.0),
            });
        }
    }

    let e = original.aabb().extents();
    let (ux, uy) = profile.usable_bed_area(opts.margin);
    Err(K1FixError::DoesNotFit {
        detail: format!(
            "footprint {:.2}x{:.2}x{:.2} mm vs usable {:.2}x{:.2}x{:.2} mm — split the part or pass --scale-to-fit",
            e.x, e.y, e.z, ux, uy, profile.build_z_mm
        ),
    })
}

/// Ensure mesh rests on Z=0; used by diagnostics helpers.
pub fn drop_to_bed(mesh: &mut IndexedMesh) {
    let min_z = mesh.aabb().min.z;
    if min_z.abs() > BED_CONTACT_EPS_MM {
        mesh.translate(Vector3::new(0.0, 0.0, -min_z));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::profiles::load_profile;

    #[test]
    fn unit_cube_fits_without_rotation() {
        let mut mesh = IndexedMesh::unit_cube();
        let profile = load_profile("k1").expect("p");
        let res = autofit(&mut mesh, &profile, &AutofitOptions::default()).expect("fit");
        assert!((res.scale - 1.0).abs() < 1e-12);
        assert!(fits(&mesh, &profile, 3.0));
        assert!(mesh.aabb().min.z.abs() < 1e-6);
    }

    #[test]
    fn kitchen_tray_fits_after_autofit() {
        let mut mesh = IndexedMesh::kitchen_tray();
        let profile = load_profile("k1").expect("p");
        autofit(&mut mesh, &profile, &AutofitOptions::default()).expect("tray should fit");
        assert!(fits(&mesh, &profile, 3.0));
        let e = mesh.aabb().extents();
        assert!(e.x <= 214.0 + 1e-3);
        assert!(e.y <= 214.0 + 1e-3);
        assert!(e.z <= 250.0 + 1e-3);
    }

    #[test]
    fn impossible_without_scale_errors() {
        let mut mesh = IndexedMesh::box_mesh(500.0, 500.0, 500.0);
        let profile = load_profile("k1").expect("p");
        let err = autofit(&mut mesh, &profile, &AutofitOptions::default());
        assert!(err.is_err());
    }

    #[test]
    fn scale_to_fit_works() {
        let mut mesh = IndexedMesh::box_mesh(400.0, 100.0, 50.0);
        let profile = load_profile("k1").expect("p");
        let res = autofit(
            &mut mesh,
            &profile,
            &AutofitOptions {
                margin: 3.0,
                scale_to_fit: true,
            },
        )
        .expect("scale");
        assert!(res.scale < 1.0);
        assert!(fits(&mesh, &profile, 3.0));
    }

    #[test]
    fn generators_produce_candidates() {
        let mesh = IndexedMesh::unit_cube();
        assert_eq!(AxisAligned24.candidates(&mesh).len(), 24);
        assert!(!PcaObbFaces.candidates(&mesh).is_empty());
    }
}
