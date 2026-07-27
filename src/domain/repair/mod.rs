//! Mesh repair pipeline as a chain of `RepairStep`s.

use nalgebra::{Point3, Vector3};
use rustc_hash::FxHashMap;

use crate::domain::constants::{DEGENERATE_AREA_EPS, MAX_HOLE_LOOP_VERTS, SPECK_VOLUME_RATIO};
use crate::domain::mesh::IndexedMesh;
use crate::domain::spatial::weld_vertices;
use crate::domain::topology::{
    edge_key, extract_boundary_loops, face_connected_components, EdgeMap,
};

/// Outcome of a single repair step.
#[derive(Debug, Clone, PartialEq)]
pub struct StepOutcome {
    /// Short description of what changed.
    pub summary: String,
    /// Whether the mesh was modified.
    pub changed: bool,
}

/// A single repair operation (OCP).
pub trait RepairStep: Send + Sync {
    /// Stable name.
    fn name(&self) -> &'static str;
    /// Apply the step in place.
    fn apply(&self, mesh: &mut IndexedMesh) -> StepOutcome;
}

/// Options for the full repair pipeline.
#[derive(Debug, Clone)]
pub struct RepairOptions {
    /// Drop disconnected specks.
    pub drop_specks: bool,
}

impl Default for RepairOptions {
    fn default() -> Self {
        Self { drop_specks: true }
    }
}

/// Run the ordered repair pipeline and return step outcomes.
pub fn repair_mesh(mesh: &mut IndexedMesh, opts: &RepairOptions) -> Vec<StepOutcome> {
    let mut steps: Vec<Box<dyn RepairStep>> = vec![
        Box::new(WeldStep),
        Box::new(DegenerateStep),
        Box::new(NonManifoldStep),
        Box::new(OrientStep),
        Box::new(FillHolesStep),
        Box::new(InteriorFacesStep),
    ];
    if opts.drop_specks {
        steps.push(Box::new(DropSpecksStep));
    }
    steps.push(Box::new(RecalcNormalsStep));

    let mut outcomes = Vec::new();
    for step in steps {
        outcomes.push(step.apply(mesh));
    }
    outcomes
}

struct WeldStep;
impl RepairStep for WeldStep {
    fn name(&self) -> &'static str {
        "weld"
    }
    fn apply(&self, mesh: &mut IndexedMesh) -> StepOutcome {
        let before = mesh.vertex_count();
        *mesh = weld_vertices(mesh);
        let after = mesh.vertex_count();
        StepOutcome {
            summary: format!("welded vertices {before} -> {after}"),
            changed: before != after,
        }
    }
}

struct DegenerateStep;
impl RepairStep for DegenerateStep {
    fn name(&self) -> &'static str {
        "degenerate"
    }
    fn apply(&self, mesh: &mut IndexedMesh) -> StepOutcome {
        let before = mesh.triangle_count();
        let areas: Vec<f64> = mesh.tris.iter().map(|t| mesh.triangle_area(*t)).collect();
        let mut kept = Vec::with_capacity(mesh.tris.len());
        for (t, area) in mesh.tris.iter().zip(areas.iter()) {
            if t[0] != t[1] && t[1] != t[2] && t[0] != t[2] && *area >= DEGENERATE_AREA_EPS {
                kept.push(*t);
            }
        }
        mesh.tris = kept;
        let after = mesh.triangle_count();
        StepOutcome {
            summary: format!("removed {} degenerate triangles", before - after),
            changed: before != after,
        }
    }
}

struct NonManifoldStep;
impl RepairStep for NonManifoldStep {
    fn name(&self) -> &'static str {
        "nonmanifold"
    }
    fn apply(&self, mesh: &mut IndexedMesh) -> StepOutcome {
        let map = EdgeMap::build(mesh);
        let nm = map.nonmanifold_edges();
        if nm.is_empty() {
            return StepOutcome {
                summary: "no non-manifold edges".into(),
                changed: false,
            };
        }
        // Duplicate vertices along non-manifold edges for all but the first two faces.
        let mut changed = false;
        for (a, b) in nm {
            let Some(faces) = map.edges.get(&(a, b)) else {
                continue;
            };
            for &face in faces.iter().skip(2) {
                let tri = &mut mesh.tris[face as usize];
                for idx in tri.iter_mut() {
                    if *idx == a || *idx == b {
                        let p = mesh.verts[*idx as usize];
                        let new_id = mesh.verts.len() as u32;
                        mesh.verts.push(p);
                        *idx = new_id;
                        changed = true;
                    }
                }
            }
        }
        StepOutcome {
            summary: format!("separated non-manifold edges"),
            changed,
        }
    }
}

struct OrientStep;
impl RepairStep for OrientStep {
    fn name(&self) -> &'static str {
        "orient"
    }
    fn apply(&self, mesh: &mut IndexedMesh) -> StepOutcome {
        orient_consistent(mesh);
        if mesh.signed_volume() < 0.0 {
            mesh.flip_winding();
        }
        StepOutcome {
            summary: "oriented winding consistently".into(),
            changed: true,
        }
    }
}

fn orient_consistent(mesh: &mut IndexedMesh) {
    let n = mesh.tris.len();
    if n == 0 {
        return;
    }
    let map = EdgeMap::build(mesh);
    // adjacency: face -> (neighbour, need_flip)
    let mut adj: Vec<Vec<(u32, bool)>> = vec![Vec::new(); n];
    for (&(a, b), faces) in &map.edges {
        if faces.len() != 2 {
            continue;
        }
        let f0 = faces[0] as usize;
        let f1 = faces[1] as usize;
        let same =
            same_edge_direction(&mesh.tris[f0], a, b) == same_edge_direction(&mesh.tris[f1], a, b);
        // If both walk the edge the same way, one must flip.
        adj[f0].push((faces[1], same));
        adj[f1].push((faces[0], same));
    }

    let mut visited = vec![false; n];
    for start in 0..n {
        if visited[start] {
            continue;
        }
        let mut stack = vec![start];
        visited[start] = true;
        while let Some(f) = stack.pop() {
            for &(nb, need_flip) in &adj[f] {
                if visited[nb as usize] {
                    continue;
                }
                visited[nb as usize] = true;
                if need_flip {
                    mesh.tris[nb as usize].swap(1, 2);
                }
                stack.push(nb as usize);
            }
        }
    }
}

fn same_edge_direction(tri: &[u32; 3], a: u32, b: u32) -> bool {
    for (i, j) in [(0usize, 1), (1, 2), (2, 0)] {
        if tri[i] == a && tri[j] == b {
            return true;
        }
        if tri[i] == b && tri[j] == a {
            return false;
        }
    }
    true
}

struct FillHolesStep;
impl RepairStep for FillHolesStep {
    fn name(&self) -> &'static str {
        "fill_holes"
    }
    fn apply(&self, mesh: &mut IndexedMesh) -> StepOutcome {
        let loops = extract_boundary_loops(mesh);
        let mut filled = 0usize;
        for loop_verts in loops {
            if loop_verts.len() < 3 || loop_verts.len() > MAX_HOLE_LOOP_VERTS {
                continue;
            }
            if let Some(new_tris) = ear_clip_loop(mesh, &loop_verts) {
                filled += new_tris.len();
                mesh.tris.extend(new_tris);
            }
        }
        StepOutcome {
            summary: format!("filled holes with {filled} triangles"),
            changed: filled > 0,
        }
    }
}

fn ear_clip_loop(mesh: &IndexedMesh, loop_verts: &[u32]) -> Option<Vec<[u32; 3]>> {
    // Project to best-fit plane and ear-clip in 2D.
    let pts: Vec<Point3<f64>> = loop_verts.iter().map(|&i| mesh.verts[i as usize]).collect();
    let (origin, axis_u, axis_v) = best_fit_plane(&pts)?;
    let mut poly: Vec<(u32, f64, f64)> = loop_verts
        .iter()
        .map(|&id| {
            let p = mesh.verts[id as usize];
            let d = p - origin;
            (id, d.dot(&axis_u), d.dot(&axis_v))
        })
        .collect();

    let mut tris = Vec::new();
    let mut guard = 0;
    while poly.len() > 3 && guard < MAX_HOLE_LOOP_VERTS * 3 {
        guard += 1;
        let n = poly.len();
        let mut clipped = false;
        for i in 0..n {
            let prev = (i + n - 1) % n;
            let next = (i + 1) % n;
            if is_ear(&poly, prev, i, next) {
                tris.push([poly[prev].0, poly[i].0, poly[next].0]);
                poly.remove(i);
                clipped = true;
                break;
            }
        }
        if !clipped {
            // fallback fan
            for i in 1..poly.len() - 1 {
                tris.push([poly[0].0, poly[i].0, poly[i + 1].0]);
            }
            break;
        }
    }
    if poly.len() == 3 {
        tris.push([poly[0].0, poly[1].0, poly[2].0]);
    }
    Some(tris)
}

fn best_fit_plane(pts: &[Point3<f64>]) -> Option<(Point3<f64>, Vector3<f64>, Vector3<f64>)> {
    if pts.len() < 3 {
        return None;
    }
    let mut c = Vector3::zeros();
    for p in pts {
        c += p.coords;
    }
    c /= pts.len() as f64;
    let origin = Point3::from(c);
    // Newell's method for normal
    let mut n: Vector3<f64> = Vector3::zeros();
    for i in 0..pts.len() {
        let cur = pts[i];
        let nxt = pts[(i + 1) % pts.len()];
        n.x += (cur.y - nxt.y) * (cur.z + nxt.z);
        n.y += (cur.z - nxt.z) * (cur.x + nxt.x);
        n.z += (cur.x - nxt.x) * (cur.y + nxt.y);
    }
    if n.norm() < 1e-12 {
        return None;
    }
    let normal = n.normalize();
    let axis_u = if normal.x.abs() < 0.9 {
        normal.cross(&Vector3::x_axis()).normalize()
    } else {
        normal.cross(&Vector3::y_axis()).normalize()
    };
    let axis_v = normal.cross(&axis_u);
    Some((origin, axis_u, axis_v))
}

fn is_ear(poly: &[(u32, f64, f64)], prev: usize, i: usize, next: usize) -> bool {
    let a = poly[prev];
    let b = poly[i];
    let c = poly[next];
    // cross z > 0 for CCW ear
    let cross = (b.1 - a.1) * (c.2 - a.2) - (b.2 - a.2) * (c.1 - a.1);
    if cross <= 0.0 {
        return false;
    }
    for (j, p) in poly.iter().enumerate() {
        if j == prev || j == i || j == next {
            continue;
        }
        if point_in_tri(p.1, p.2, a.1, a.2, b.1, b.2, c.1, c.2) {
            return false;
        }
    }
    true
}

fn point_in_tri(px: f64, py: f64, ax: f64, ay: f64, bx: f64, by: f64, cx: f64, cy: f64) -> bool {
    let v0x = cx - ax;
    let v0y = cy - ay;
    let v1x = bx - ax;
    let v1y = by - ay;
    let v2x = px - ax;
    let v2y = py - ay;
    let dot00 = v0x * v0x + v0y * v0y;
    let dot01 = v0x * v1x + v0y * v1y;
    let dot02 = v0x * v2x + v0y * v2y;
    let dot11 = v1x * v1x + v1y * v1y;
    let dot12 = v1x * v2x + v1y * v2y;
    let denom = dot00 * dot11 - dot01 * dot01;
    if denom.abs() < 1e-18 {
        return false;
    }
    let inv = 1.0 / denom;
    let u = (dot11 * dot02 - dot01 * dot12) * inv;
    let v = (dot00 * dot12 - dot01 * dot02) * inv;
    u >= 0.0 && v >= 0.0 && (u + v) < 1.0
}

struct InteriorFacesStep;
impl RepairStep for InteriorFacesStep {
    fn name(&self) -> &'static str {
        "interior"
    }
    fn apply(&self, mesh: &mut IndexedMesh) -> StepOutcome {
        // Remove exact duplicate faces (same three indices, any winding).
        let before = mesh.triangle_count();
        let mut seen: FxHashMap<[u32; 3], ()> = FxHashMap::default();
        mesh.tris.retain(|t| {
            let mut key = *t;
            key.sort_unstable();
            seen.insert(key, ()).is_none()
        });
        let after = mesh.triangle_count();
        StepOutcome {
            summary: format!("removed {} duplicate faces", before - after),
            changed: before != after,
        }
    }
}

struct DropSpecksStep;
impl RepairStep for DropSpecksStep {
    fn name(&self) -> &'static str {
        "specks"
    }
    fn apply(&self, mesh: &mut IndexedMesh) -> StepOutcome {
        let comps = face_connected_components(mesh);
        if comps.len() <= 1 {
            return StepOutcome {
                summary: "no specks".into(),
                changed: false,
            };
        }
        let total = mesh.volume().max(1e-18);
        let mut keep = vec![true; mesh.tris.len()];
        let mut dropped = 0usize;
        for comp in &comps {
            let mut vol = 0.0;
            for &f in comp {
                let tri = mesh.tris[f as usize];
                let a = mesh.verts[tri[0] as usize];
                let b = mesh.verts[tri[1] as usize];
                let c = mesh.verts[tri[2] as usize];
                vol += a.coords.dot(&b.coords.cross(&c.coords)) / 6.0;
            }
            if vol.abs() / total < SPECK_VOLUME_RATIO {
                for &f in comp {
                    keep[f as usize] = false;
                    dropped += 1;
                }
            }
        }
        if dropped == 0 {
            return StepOutcome {
                summary: "no specks".into(),
                changed: false,
            };
        }
        let mut new_tris = Vec::new();
        for (i, t) in mesh.tris.iter().enumerate() {
            if keep[i] {
                new_tris.push(*t);
            }
        }
        mesh.tris = new_tris;
        StepOutcome {
            summary: format!("dropped {dropped} speck faces"),
            changed: true,
        }
    }
}

struct RecalcNormalsStep;
impl RepairStep for RecalcNormalsStep {
    fn name(&self) -> &'static str {
        "normals"
    }
    fn apply(&self, mesh: &mut IndexedMesh) -> StepOutcome {
        // Normals are derived on write; ensure positive volume.
        if mesh.signed_volume() < 0.0 {
            mesh.flip_winding();
        }
        let _ = edge_key(0, 1); // keep helper referenced for coverage of topology re-exports usage
        StepOutcome {
            summary: "normals recalculated".into(),
            changed: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::topology::cube_missing_top;

    #[test]
    fn repair_fills_single_boundary_loop_of_a_cube_missing_one_face() {
        let mut mesh = cube_missing_top();
        assert!(EdgeMap::build(&mesh).open_edge_count() > 0);
        let outcomes = repair_mesh(&mut mesh, &RepairOptions::default());
        assert!(outcomes.iter().any(|o| o.summary.contains("filled")));
        assert_eq!(EdgeMap::build(&mesh).open_edge_count(), 0);
        assert!(mesh.signed_volume() > 0.0);
    }

    #[test]
    fn repair_removes_degenerate() {
        let mut mesh = IndexedMesh::from_parts(
            vec![
                Point3::origin(),
                Point3::new(1.0, 0.0, 0.0),
                Point3::new(0.0, 1.0, 0.0),
            ],
            vec![[0, 1, 2], [0, 0, 1]],
        );
        repair_mesh(&mut mesh, &RepairOptions { drop_specks: false });
        assert!(mesh.tris.iter().all(|t| t[0] != t[1]));
    }

    #[test]
    fn repair_closed_cube_idempotent_enough() {
        let mut mesh = IndexedMesh::unit_cube();
        repair_mesh(&mut mesh, &RepairOptions::default());
        assert_eq!(EdgeMap::build(&mesh).open_edge_count(), 0);
        assert!(mesh.signed_volume() > 0.0);
    }
}
