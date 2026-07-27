//! Human and JSON reports.

use serde::{Deserialize, Serialize};

use crate::domain::diagnostics::{worst_severity, Finding, Severity};
use crate::domain::mesh::IndexedMesh;
use crate::domain::profiles::PrinterProfile;
use crate::domain::repair::StepOutcome;

/// Full inspection / fix report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Report {
    /// Input path if known.
    pub input: Option<String>,
    /// Profile id.
    pub profile: String,
    /// Bounding box extents mm.
    pub extents_mm: [f64; 3],
    /// Triangle count.
    pub triangles: usize,
    /// Vertex count.
    pub vertices: usize,
    /// Volume mm³.
    pub volume_mm3: f64,
    /// Diagnostic findings.
    pub findings: Vec<Finding>,
    /// Repair step summaries (if repair ran).
    pub repair_steps: Vec<String>,
    /// Autofit summary (if ran).
    pub autofit: Option<String>,
    /// Suggested process exit code.
    pub exit_code: i32,
}

impl Report {
    /// Build a report from mesh + findings.
    #[must_use]
    pub fn from_inspection(
        input: Option<String>,
        profile: &PrinterProfile,
        mesh: &IndexedMesh,
        findings: Vec<Finding>,
    ) -> Self {
        let e = mesh.aabb().extents();
        let exit_code = exit_code_for(&findings);
        Self {
            input,
            profile: profile.id.clone(),
            extents_mm: [e.x, e.y, e.z],
            triangles: mesh.triangle_count(),
            vertices: mesh.vertex_count(),
            volume_mm3: mesh.volume(),
            findings,
            repair_steps: Vec::new(),
            autofit: None,
            exit_code,
        }
    }

    /// Attach repair outcomes.
    pub fn with_repair(mut self, steps: &[StepOutcome]) -> Self {
        self.repair_steps = steps.iter().map(|s| s.summary.clone()).collect();
        self
    }

    /// Attach autofit summary.
    pub fn with_autofit(mut self, summary: impl Into<String>) -> Self {
        self.autofit = Some(summary.into());
        self
    }

    /// Recompute exit code from findings (call after mutating findings).
    pub fn recompute_exit_code(&mut self) {
        self.exit_code = exit_code_for(&self.findings);
    }

    /// Render a compact human-readable table.
    #[must_use]
    pub fn render_human(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!(
            "profile: {}\nextents: {:.2} x {:.2} x {:.2} mm\ntriangles: {}\nvertices: {}\nvolume: {:.1} mm³\n",
            self.profile,
            self.extents_mm[0],
            self.extents_mm[1],
            self.extents_mm[2],
            self.triangles,
            self.vertices,
            self.volume_mm3
        ));
        if let Some(af) = &self.autofit {
            out.push_str(&format!("autofit: {af}\n"));
        }
        if !self.repair_steps.is_empty() {
            out.push_str("repair:\n");
            for s in &self.repair_steps {
                out.push_str(&format!("  - {s}\n"));
            }
        }
        if self.findings.is_empty() {
            out.push_str("findings: none\n");
        } else {
            out.push_str("findings:\n");
            for f in &self.findings {
                out.push_str(&format!("  [{:?}] {}: {}\n", f.severity, f.id, f.message));
            }
        }
        out.push_str(&format!("exit_code: {}\n", self.exit_code));
        out
    }

    /// Serialize to JSON.
    ///
    /// # Errors
    /// Returns a serde error on failure.
    pub fn to_json(&self) -> crate::domain::error::Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }
}

/// Map findings to process exit code: 0 clean/info, 1 warnings, 2 errors.
#[must_use]
pub fn exit_code_for(findings: &[Finding]) -> i32 {
    match worst_severity(findings) {
        None | Some(Severity::Info) => 0,
        Some(Severity::Warning) => 1,
        Some(Severity::Error) => 2,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::diagnostics::{run_all_checks, MeshContext};
    use crate::domain::profiles::load_profile;

    #[test]
    fn report_json_round_trip_shape() {
        let mesh = IndexedMesh::unit_cube();
        let profile = load_profile("k1").expect("p");
        let ctx = MeshContext {
            mesh: &mesh,
            profile: &profile,
            margin: 3.0,
        };
        let findings = run_all_checks(&ctx);
        let report = Report::from_inspection(Some("cube.stl".into()), &profile, &mesh, findings);
        let json = report.to_json().expect("json");
        assert!(json.contains("extents_mm"));
        let human = report.render_human();
        assert!(human.contains("profile:"));
    }
}
