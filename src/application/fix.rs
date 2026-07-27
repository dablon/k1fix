//! Fix use case: repair → autofit → write.

use std::path::Path;

use crate::application::ports::{MeshLoader, MeshStore, ProgressSink};
use crate::application::report::Report;
use crate::domain::autofit::{autofit, AutofitOptions};
use crate::domain::diagnostics::{run_all_checks, MeshContext};
use crate::domain::error::{K1FixError, Result};
use crate::domain::profiles::PrinterProfile;
use crate::domain::repair::{repair_mesh, RepairOptions};

/// Options for the fix pipeline.
#[derive(Debug, Clone)]
pub struct FixOptions {
    /// Run mesh repair.
    pub repair: bool,
    /// Run auto-fit.
    pub autofit: bool,
    /// Allow uniform scale when fitting.
    pub scale_to_fit: bool,
    /// Prefer low Z (flat) orientations for printability.
    pub prefer_flat: bool,
    /// Bed margin mm.
    pub margin: f64,
    /// Drop speck shells.
    pub drop_specks: bool,
    /// Do not write output.
    pub dry_run: bool,
    /// STEP tessellation tolerance.
    pub tess_tol: f64,
}

impl Default for FixOptions {
    fn default() -> Self {
        Self {
            repair: true,
            autofit: true,
            scale_to_fit: false,
            prefer_flat: true,
            margin: 3.0,
            drop_specks: true,
            dry_run: false,
            tess_tol: crate::domain::constants::DEFAULT_TESS_TOL_MM,
        }
    }
}

/// Repair and/or auto-fit a mesh, then optionally write it.
pub struct FixUseCase<'a, L: MeshLoader, S: MeshStore, P: ProgressSink> {
    loader: &'a L,
    store: &'a S,
    progress: &'a P,
}

impl<'a, L: MeshLoader, S: MeshStore, P: ProgressSink> FixUseCase<'a, L, S, P> {
    /// Create a use case with injected ports.
    #[must_use]
    pub fn new(loader: &'a L, store: &'a S, progress: &'a P) -> Self {
        Self {
            loader,
            store,
            progress,
        }
    }

    /// Execute the fix pipeline.
    ///
    /// # Errors
    /// Propagates IO / parse / fit errors.
    pub fn execute(
        &self,
        input: &Path,
        output: &Path,
        profile: &PrinterProfile,
        opts: &FixOptions,
    ) -> Result<Report> {
        self.progress.message("reading mesh");
        let mut mesh = self.loader.load(input, opts.tess_tol)?;

        let mut repair_steps = Vec::new();
        if opts.repair {
            self.progress.message("repairing mesh");
            repair_steps = repair_mesh(
                &mut mesh,
                &RepairOptions {
                    drop_specks: opts.drop_specks,
                },
            );
        }

        let mut autofit_summary = None;
        if opts.autofit {
            self.progress.message("auto-fitting");
            match autofit(
                &mut mesh,
                profile,
                &AutofitOptions {
                    margin: opts.margin,
                    scale_to_fit: opts.scale_to_fit,
                    prefer_flat: opts.prefer_flat,
                },
            ) {
                Ok(res) => autofit_summary = Some(res.summary),
                Err(K1FixError::DoesNotFit { detail }) => {
                    let ctx = MeshContext {
                        mesh: &mesh,
                        profile,
                        margin: opts.margin,
                    };
                    let findings = run_all_checks(&ctx);
                    let mut report = Report::from_inspection(
                        Some(input.display().to_string()),
                        profile,
                        &mesh,
                        findings,
                    )
                    .with_repair(&repair_steps);
                    report.exit_code = 2;
                    report.autofit = Some(detail);
                    return Ok(report);
                }
                Err(e) => return Err(e),
            }
        }

        if !opts.dry_run {
            self.progress.message("writing output");
            self.store.store(output, &mesh)?;
        }

        let ctx = MeshContext {
            mesh: &mesh,
            profile,
            margin: opts.margin,
        };
        let findings = run_all_checks(&ctx);
        let mut report =
            Report::from_inspection(Some(input.display().to_string()), profile, &mesh, findings)
                .with_repair(&repair_steps);
        if let Some(s) = autofit_summary {
            report = report.with_autofit(s);
        }
        self.progress.fraction(1.0);
        Ok(report)
    }
}
