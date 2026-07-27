//! Convert use case: read one format, write another.

use std::path::Path;

use crate::application::ports::{MeshLoader, MeshStore, ProgressSink};
use crate::domain::error::Result;

/// Convert a mesh between formats inferred from path extensions.
pub struct ConvertUseCase<'a, L: MeshLoader, S: MeshStore, P: ProgressSink> {
    loader: &'a L,
    store: &'a S,
    progress: &'a P,
}

impl<'a, L: MeshLoader, S: MeshStore, P: ProgressSink> ConvertUseCase<'a, L, S, P> {
    /// Create a use case with injected ports.
    #[must_use]
    pub fn new(loader: &'a L, store: &'a S, progress: &'a P) -> Self {
        Self {
            loader,
            store,
            progress,
        }
    }

    /// Execute conversion.
    ///
    /// # Errors
    /// Propagates IO / parse / encode errors.
    pub fn execute(&self, input: &Path, output: &Path, tess_tol: f64) -> Result<()> {
        self.progress.message("reading");
        let mesh = self.loader.load(input, tess_tol)?;
        self.progress.message("writing");
        self.store.store(output, &mesh)?;
        self.progress.fraction(1.0);
        Ok(())
    }
}
