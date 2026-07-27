//! Diagnostic engine: registry of `Check` implementations.

use serde::{Deserialize, Serialize};

use crate::domain::constants::{
    BED_CONTACT_EPS_MM, EXCESSIVE_TRIANGLE_COUNT, OVERHANG_LIMIT_DEG, SUSPICIOUS_MAX_DIAG_MM,
    SUSPICIOUS_MIN_DIAG_MM,
};
use crate::domain::mesh::IndexedMesh;
use crate::domain::profiles::PrinterProfile;
use crate::domain::spatial::MeshBvh;
use crate::domain::topology::{EdgeMap, Topology};

/// Stable diagnostic identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum CheckId {
    Fit001,
    Fit002,
    Fit003,
    Fit004,
    Mesh001,
    Mesh002,
    Mesh003,
    Mesh004,
    Mesh005,
    Mesh006,
    Mesh007,
    Prt001,
    Prt002,
    Prt003,
    Prt004,
    Prt005,
}

impl CheckId {
    /// Wire format used in JSON reports.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Fit001 => "FIT001",
            Self::Fit002 => "FIT002",
            Self::Fit003 => "FIT003",
            Self::Fit004 => "FIT004",
            Self::Mesh001 => "MESH001",
            Self::Mesh002 => "MESH002",
            Self::Mesh003 => "MESH003",
            Self::Mesh004 => "MESH004",
            Self::Mesh005 => "MESH005",
            Self::Mesh006 => "MESH006",
            Self::Mesh007 => "MESH007",
            Self::Prt001 => "PRT001",
            Self::Prt002 => "PRT002",
            Self::Prt003 => "PRT003",
            Self::Prt004 => "PRT004",
            Self::Prt005 => "PRT005",
        }
    }
}

impl std::fmt::Display for CheckId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Finding severity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Info,
    Warning,
    Error,
}

/// A single diagnostic finding.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Finding {
    /// Stable id.
    pub id: String,
    /// Severity.
    pub severity: Severity,
    /// Human message.
    pub message: String,
}

impl Finding {
    /// Construct a finding from a check id.
    #[must_use]
    pub fn new(id: CheckId, severity: Severity, message: impl Into<String>) -> Self {
        Self {
            id: id.as_str().to_string(),
            severity,
            message: message.into(),
        }
    }
}

/// Shared context passed to checks.
pub struct MeshContext<'a> {
    /// Mesh under inspection.
    pub mesh: &'a IndexedMesh,
    /// Printer profile.
    pub profile: &'a PrinterProfile,
    /// Bed margin override.
    pub margin: f64,
}

/// Open/closed diagnostic check (OCP).
pub trait Check: Send + Sync {
    /// Stable id.
    fn id(&self) -> CheckId;
    /// Default severity.
    fn severity(&self) -> Severity;
    /// Run the check.
    fn run(&self, ctx: &MeshContext<'_>) -> Vec<Finding>;
}

/// Run the default check registry.
#[must_use]
pub fn run_all_checks(ctx: &MeshContext<'_>) -> Vec<Finding> {
    let checks: Vec<Box<dyn Check>> = default_registry();
    let mut out = Vec::new();
    for check in checks {
        out.extend(check.run(ctx));
    }
    out
}

/// Default registry of all built-in checks.
#[must_use]
pub fn default_registry() -> Vec<Box<dyn Check>> {
    vec![
        Box::new(FitXy),
        Box::new(FitZ),
        Box::new(FitBedContact),
        Box::new(FitMargin),
        Box::new(MeshOpenEdges),
        Box::new(MeshNonManifold),
        Box::new(MeshWinding),
        Box::new(MeshDegenerate),
        Box::new(MeshDuplicates),
        Box::new(MeshShells),
        Box::new(MeshSelfIntersect),
        Box::new(PrtThinWalls),
        Box::new(PrtOverhangs),
        Box::new(PrtFineDetail),
        Box::new(PrtSuspiciousScale),
        Box::new(PrtTooManyTris),
    ]
}

struct FitXy;
impl Check for FitXy {
    fn id(&self) -> CheckId {
        CheckId::Fit001
    }
    fn severity(&self) -> Severity {
        Severity::Error
    }
    fn run(&self, ctx: &MeshContext<'_>) -> Vec<Finding> {
        let box_ = ctx.mesh.aabb();
        let (ux, uy) = ctx.profile.usable_bed_area(ctx.margin);
        let e = box_.extents();
        let mut out = Vec::new();
        if e.x > ux + 1e-6 || e.y > uy + 1e-6 {
            out.push(Finding::new(
                self.id(),
                self.severity(),
                format!(
                    "XY footprint {:.2}x{:.2} mm exceeds usable bed {:.2}x{:.2} mm",
                    e.x, e.y, ux, uy
                ),
            ));
        }
        // Also flag if currently placed outside the bed origin box
        if box_.min.x < -1e-6
            || box_.min.y < -1e-6
            || box_.max.x > ctx.profile.bed_x_mm + 1e-6
            || box_.max.y > ctx.profile.bed_y_mm + 1e-6
        {
            if out.is_empty() {
                out.push(Finding::new(
                    self.id(),
                    self.severity(),
                    "object crosses the plate boundary",
                ));
            }
        }
        out
    }
}

struct FitZ;
impl Check for FitZ {
    fn id(&self) -> CheckId {
        CheckId::Fit002
    }
    fn severity(&self) -> Severity {
        Severity::Error
    }
    fn run(&self, ctx: &MeshContext<'_>) -> Vec<Finding> {
        let h = ctx.mesh.aabb().extents().z;
        if h > ctx.profile.build_z_mm + 1e-6 {
            vec![Finding::new(
                self.id(),
                self.severity(),
                format!(
                    "height {:.2} mm exceeds build volume Z {:.2} mm",
                    h, ctx.profile.build_z_mm
                ),
            )]
        } else {
            vec![]
        }
    }
}

struct FitBedContact;
impl Check for FitBedContact {
    fn id(&self) -> CheckId {
        CheckId::Fit003
    }
    fn severity(&self) -> Severity {
        Severity::Warning
    }
    fn run(&self, ctx: &MeshContext<'_>) -> Vec<Finding> {
        let min_z = ctx.mesh.aabb().min.z;
        if min_z > BED_CONTACT_EPS_MM {
            vec![Finding::new(
                self.id(),
                self.severity(),
                format!("mesh floats above bed (min Z = {min_z:.3} mm)"),
            )]
        } else {
            vec![]
        }
    }
}

struct FitMargin;
impl Check for FitMargin {
    fn id(&self) -> CheckId {
        CheckId::Fit004
    }
    fn severity(&self) -> Severity {
        Severity::Warning
    }
    fn run(&self, ctx: &MeshContext<'_>) -> Vec<Finding> {
        let box_ = ctx.mesh.aabb();
        let m = ctx.margin;
        if box_.min.x < m - 1e-6
            || box_.min.y < m - 1e-6
            || box_.max.x > ctx.profile.bed_x_mm - m + 1e-6
            || box_.max.y > ctx.profile.bed_y_mm - m + 1e-6
        {
            vec![Finding::new(
                self.id(),
                self.severity(),
                format!("mesh invades the {m:.1} mm bed margin"),
            )]
        } else {
            vec![]
        }
    }
}

struct MeshOpenEdges;
impl Check for MeshOpenEdges {
    fn id(&self) -> CheckId {
        CheckId::Mesh001
    }
    fn severity(&self) -> Severity {
        Severity::Error
    }
    fn run(&self, ctx: &MeshContext<'_>) -> Vec<Finding> {
        let n = EdgeMap::build(ctx.mesh).open_edge_count();
        if n > 0 {
            vec![Finding::new(
                self.id(),
                self.severity(),
                format!("{n} open boundary edges (holes)"),
            )]
        } else {
            vec![]
        }
    }
}

struct MeshNonManifold;
impl Check for MeshNonManifold {
    fn id(&self) -> CheckId {
        CheckId::Mesh002
    }
    fn severity(&self) -> Severity {
        Severity::Error
    }
    fn run(&self, ctx: &MeshContext<'_>) -> Vec<Finding> {
        let n = EdgeMap::build(ctx.mesh).nonmanifold_edges().len();
        if n > 0 {
            vec![Finding::new(
                self.id(),
                self.severity(),
                format!("{n} non-manifold edges"),
            )]
        } else {
            vec![]
        }
    }
}

struct MeshWinding;
impl Check for MeshWinding {
    fn id(&self) -> CheckId {
        CheckId::Mesh003
    }
    fn severity(&self) -> Severity {
        Severity::Warning
    }
    fn run(&self, ctx: &MeshContext<'_>) -> Vec<Finding> {
        if ctx.mesh.signed_volume() < 0.0 {
            vec![Finding::new(
                self.id(),
                self.severity(),
                "inconsistent / inverted winding (negative signed volume)",
            )]
        } else {
            vec![]
        }
    }
}

struct MeshDegenerate;
impl Check for MeshDegenerate {
    fn id(&self) -> CheckId {
        CheckId::Mesh004
    }
    fn severity(&self) -> Severity {
        Severity::Warning
    }
    fn run(&self, ctx: &MeshContext<'_>) -> Vec<Finding> {
        let n = ctx
            .mesh
            .tris
            .iter()
            .filter(|t| ctx.mesh.is_degenerate(**t))
            .count();
        if n > 0 {
            vec![Finding::new(
                self.id(),
                self.severity(),
                format!("{n} degenerate triangles"),
            )]
        } else {
            vec![]
        }
    }
}

struct MeshDuplicates;
impl Check for MeshDuplicates {
    fn id(&self) -> CheckId {
        CheckId::Mesh005
    }
    fn severity(&self) -> Severity {
        Severity::Info
    }
    fn run(&self, ctx: &MeshContext<'_>) -> Vec<Finding> {
        let welded = crate::domain::spatial::weld_vertices(ctx.mesh);
        let dup_v = ctx
            .mesh
            .vertex_count()
            .saturating_sub(welded.vertex_count());
        if dup_v > 0 {
            vec![Finding::new(
                self.id(),
                self.severity(),
                format!("{dup_v} duplicate vertices"),
            )]
        } else {
            vec![]
        }
    }
}

struct MeshShells;
impl Check for MeshShells {
    fn id(&self) -> CheckId {
        CheckId::Mesh006
    }
    fn severity(&self) -> Severity {
        Severity::Info
    }
    fn run(&self, ctx: &MeshContext<'_>) -> Vec<Finding> {
        let n = ctx.mesh.connected_components().len();
        if n > 1 {
            vec![Finding::new(
                self.id(),
                self.severity(),
                format!("{n} disconnected shells"),
            )]
        } else {
            vec![]
        }
    }
}

struct MeshSelfIntersect;
impl Check for MeshSelfIntersect {
    fn id(&self) -> CheckId {
        CheckId::Mesh007
    }
    fn severity(&self) -> Severity {
        Severity::Warning
    }
    fn run(&self, ctx: &MeshContext<'_>) -> Vec<Finding> {
        // Sample pairwise intersections for small meshes; skip heavy scan above 2k tris.
        if ctx.mesh.tris.len() > 2_000 {
            return vec![];
        }
        let tris = &ctx.mesh.tris;
        let verts = &ctx.mesh.verts;
        let mut hits = 0usize;
        for i in 0..tris.len() {
            for j in (i + 1)..tris.len() {
                let a = tris[i];
                let b = tris[j];
                // skip shared-vertex pairs (adjacent faces)
                let share = [a[0], a[1], a[2]].iter().any(|va| b.contains(va));
                if share {
                    continue;
                }
                if crate::domain::spatial::triangles_intersect(
                    verts[a[0] as usize],
                    verts[a[1] as usize],
                    verts[a[2] as usize],
                    verts[b[0] as usize],
                    verts[b[1] as usize],
                    verts[b[2] as usize],
                ) {
                    hits += 1;
                    if hits > 5 {
                        break;
                    }
                }
            }
            if hits > 5 {
                break;
            }
        }
        if hits > 0 {
            vec![Finding::new(
                self.id(),
                self.severity(),
                format!("possible self-intersections detected ({hits}+ pairs)"),
            )]
        } else {
            vec![]
        }
    }
}

struct PrtThinWalls;
impl Check for PrtThinWalls {
    fn id(&self) -> CheckId {
        CheckId::Prt001
    }
    fn severity(&self) -> Severity {
        Severity::Warning
    }
    fn run(&self, ctx: &MeshContext<'_>) -> Vec<Finding> {
        let Ok(bvh) = MeshBvh::build(ctx.mesh) else {
            return vec![];
        };
        let nozzle = ctx.profile.nozzle_mm;
        let mut thin = 0usize;
        let sample_every = (ctx.mesh.tris.len() / 500).max(1);
        for (i, tri) in ctx.mesh.tris.iter().enumerate() {
            if i % sample_every != 0 {
                continue;
            }
            let n = ctx.mesh.triangle_normal(*tri);
            let len = n.norm();
            if len < 1e-12 {
                continue;
            }
            let dir = -n / len;
            let origin = ctx.mesh.triangle_centroid(*tri) + dir * 1e-4;
            if let Some(toi) = bvh.cast_ray(origin, dir, nozzle * 4.0) {
                if toi < nozzle {
                    thin += 1;
                }
            }
        }
        if thin > 0 {
            vec![Finding::new(
                self.id(),
                self.severity(),
                format!("{thin} sampled faces thinner than nozzle ({nozzle:.2} mm)"),
            )]
        } else {
            vec![]
        }
    }
}

struct PrtOverhangs;
impl Check for PrtOverhangs {
    fn id(&self) -> CheckId {
        CheckId::Prt002
    }
    fn severity(&self) -> Severity {
        Severity::Warning
    }
    fn run(&self, ctx: &MeshContext<'_>) -> Vec<Finding> {
        let limit = OVERHANG_LIMIT_DEG.to_radians();
        let mut overhang_area = 0.0;
        let mut total = 0.0;
        for tri in &ctx.mesh.tris {
            let area = ctx.mesh.triangle_area(*tri);
            total += area;
            let n = ctx.mesh.triangle_normal(*tri);
            let len = n.norm();
            if len < 1e-12 {
                continue;
            }
            let nz = (n.z / len).clamp(-1.0, 1.0);
            // angle from +Z
            let angle = nz.acos();
            if angle > limit && n.z < 0.0 {
                overhang_area += area;
            }
        }
        if total <= 0.0 {
            return vec![];
        }
        let pct = 100.0 * overhang_area / total;
        if pct > 5.0 {
            vec![Finding::new(
                self.id(),
                self.severity(),
                format!("{pct:.1}% of surface overhangs > {OVERHANG_LIMIT_DEG}°"),
            )]
        } else {
            vec![]
        }
    }
}

struct PrtFineDetail;
impl Check for PrtFineDetail {
    fn id(&self) -> CheckId {
        CheckId::Prt003
    }
    fn severity(&self) -> Severity {
        Severity::Info
    }
    fn run(&self, ctx: &MeshContext<'_>) -> Vec<Finding> {
        let layer = ctx.profile.layer_height_mm;
        let small = ctx
            .mesh
            .tris
            .iter()
            .filter(|t| ctx.mesh.triangle_area(**t).sqrt() < layer)
            .count();
        if small > 10 {
            vec![Finding::new(
                self.id(),
                self.severity(),
                format!("{small} faces smaller than layer height ({layer} mm)"),
            )]
        } else {
            vec![]
        }
    }
}

struct PrtSuspiciousScale;
impl Check for PrtSuspiciousScale {
    fn id(&self) -> CheckId {
        CheckId::Prt004
    }
    fn severity(&self) -> Severity {
        Severity::Warning
    }
    fn run(&self, ctx: &MeshContext<'_>) -> Vec<Finding> {
        let d = ctx.mesh.aabb().diagonal();
        if d < SUSPICIOUS_MIN_DIAG_MM || d > SUSPICIOUS_MAX_DIAG_MM {
            vec![Finding::new(
                self.id(),
                self.severity(),
                format!("suspicious scale (bbox diagonal {d:.3} mm) — check units"),
            )]
        } else {
            vec![]
        }
    }
}

struct PrtTooManyTris;
impl Check for PrtTooManyTris {
    fn id(&self) -> CheckId {
        CheckId::Prt005
    }
    fn severity(&self) -> Severity {
        Severity::Info
    }
    fn run(&self, ctx: &MeshContext<'_>) -> Vec<Finding> {
        let n = ctx.mesh.triangle_count();
        if n > EXCESSIVE_TRIANGLE_COUNT {
            vec![Finding::new(
                self.id(),
                self.severity(),
                format!("{n} triangles — consider decimation"),
            )]
        } else {
            vec![]
        }
    }
}

/// Highest severity among findings.
#[must_use]
pub fn worst_severity(findings: &[Finding]) -> Option<Severity> {
    let mut worst = None;
    for f in findings {
        worst = match (worst, f.severity) {
            (None, s) => Some(s),
            (Some(Severity::Error), _) => Some(Severity::Error),
            (Some(Severity::Warning), Severity::Error) => Some(Severity::Error),
            (Some(Severity::Warning), _) => Some(Severity::Warning),
            (Some(Severity::Info), s) => Some(s),
        };
    }
    worst
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::diagnostics::{run_all_checks, MeshContext};
    use crate::domain::profiles::load_profile;
    use crate::domain::topology::cube_missing_top;

    #[test]
    fn open_edges_detected() {
        let mesh = cube_missing_top();
        let profile = load_profile("k1").expect("p");
        let ctx = MeshContext {
            mesh: &mesh,
            profile: &profile,
            margin: 3.0,
        };
        let findings = run_all_checks(&ctx);
        assert!(findings.iter().any(|f| f.id == "MESH001"));
    }

    #[test]
    fn tray_exceeds_xy() {
        let mesh = IndexedMesh::kitchen_tray();
        let profile = load_profile("k1").expect("p");
        let ctx = MeshContext {
            mesh: &mesh,
            profile: &profile,
            margin: 3.0,
        };
        let findings = run_all_checks(&ctx);
        assert!(findings.iter().any(|f| f.id == "FIT001"));
    }

    #[test]
    fn closed_cube_clean_mesh_checks() {
        let mesh = IndexedMesh::unit_cube();
        let profile = load_profile("k1").expect("p");
        let ctx = MeshContext {
            mesh: &mesh,
            profile: &profile,
            margin: 3.0,
        };
        let findings = run_all_checks(&ctx);
        assert!(!findings.iter().any(|f| f.id == "MESH001"));
        assert!(!findings.iter().any(|f| f.id == "MESH002"));
    }

    #[test]
    fn check_id_display() {
        assert_eq!(CheckId::Fit001.to_string(), "FIT001");
        assert_eq!(CheckId::Prt005.as_str(), "PRT005");
    }

    #[test]
    fn worst_severity_ordering() {
        let f = vec![
            Finding::new(CheckId::Prt003, Severity::Info, "i"),
            Finding::new(CheckId::Fit003, Severity::Warning, "w"),
            Finding::new(CheckId::Fit001, Severity::Error, "e"),
        ];
        assert_eq!(worst_severity(&f), Some(Severity::Error));
    }

    #[test]
    fn inverted_winding_warning() {
        let mut mesh = IndexedMesh::unit_cube();
        mesh.flip_winding();
        let profile = load_profile("k1").expect("p");
        let ctx = MeshContext {
            mesh: &mesh,
            profile: &profile,
            margin: 3.0,
        };
        let findings = run_all_checks(&ctx);
        assert!(findings.iter().any(|f| f.id == "MESH003"));
    }
}
