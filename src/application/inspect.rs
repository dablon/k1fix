//! Inspect use case: load mesh → run diagnostics → build report.

use std::path::Path;

use crate::application::ports::{MeshLoader, ProgressSink};
use crate::application::report::Report;
use crate::domain::diagnostics::{run_all_checks, MeshContext};
use crate::domain::error::Result;
use crate::domain::profiles::PrinterProfile;

/// Options for inspection.
#[derive(Debug, Clone)]
pub struct InspectOptions {
    /// Bed margin in millimetres.
    pub margin: f64,
    /// STEP tessellation tolerance.
    pub tess_tol: f64,
}

impl Default for InspectOptions {
    fn default() -> Self {
        Self {
            margin: 3.0,
            tess_tol: crate::domain::constants::DEFAULT_TESS_TOL_MM,
        }
    }
}

/// Inspect a mesh file and produce a [`Report`].
pub struct InspectUseCase<'a, L: MeshLoader, P: ProgressSink> {
    loader: &'a L,
    progress: &'a P,
}

impl<'a, L: MeshLoader, P: ProgressSink> InspectUseCase<'a, L, P> {
    /// Create a use case with injected ports.
    #[must_use]
    pub fn new(loader: &'a L, progress: &'a P) -> Self {
        Self { loader, progress }
    }

    /// Execute inspection.
    ///
    /// # Errors
    /// Propagates IO / parse errors from the loader.
    pub fn execute(
        &self,
        path: &Path,
        profile: &PrinterProfile,
        opts: &InspectOptions,
    ) -> Result<Report> {
        self.progress.message("reading mesh");
        let mesh = self.loader.load(path, opts.tess_tol)?;
        self.progress.message("running diagnostics");
        let ctx = MeshContext {
            mesh: &mesh,
            profile,
            margin: opts.margin,
        };
        let findings = run_all_checks(&ctx);
        self.progress.fraction(1.0);
        Ok(Report::from_inspection(
            Some(path.display().to_string()),
            profile,
            &mesh,
            findings,
        ))
    }
}
