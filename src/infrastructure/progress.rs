//! Progress adapters for the CLI.

use crate::application::ports::ProgressSink;

/// No-op progress sink.
#[derive(Debug, Default, Clone, Copy)]
pub struct SilentProgress;

impl ProgressSink for SilentProgress {
    fn message(&self, _msg: &str) {}
    fn fraction(&self, _value: f64) {}
}

/// Stderr progress sink.
#[derive(Debug, Default, Clone, Copy)]
pub struct StderrProgress;

impl ProgressSink for StderrProgress {
    fn message(&self, msg: &str) {
        eprintln!("{msg}");
    }

    fn fraction(&self, value: f64) {
        eprintln!("progress: {value:.0}%");
    }
}
