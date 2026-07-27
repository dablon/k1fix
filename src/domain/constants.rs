//! Shared numeric constants (no magic numbers in algorithms).

/// Default weld tolerance floor in millimetres.
pub const WELD_TOL_FLOOR_MM: f64 = 1e-6;

/// Weld tolerance as a fraction of the bounding-box diagonal.
pub const WELD_TOL_RATIO: f64 = 1e-5;

/// Degenerate triangle area threshold in mm².
pub const DEGENERATE_AREA_EPS: f64 = 1e-12;

/// Maximum boundary loop vertices accepted by ear-clipping hole fill.
pub const MAX_HOLE_LOOP_VERTS: usize = 1000;

/// Specks smaller than this fraction of total absolute volume are dropped.
pub const SPECK_VOLUME_RATIO: f64 = 0.001;

/// Overhang angle limit in degrees (from vertical) for PRT002.
pub const OVERHANG_LIMIT_DEG: f64 = 45.0;

/// Default nozzle diameter in millimetres (PRT001).
pub const NOZZLE_DIAMETER_MM: f64 = 0.4;

/// Suspicious scale: bbox diagonal below this (mm) suggests wrong units.
pub const SUSPICIOUS_MIN_DIAG_MM: f64 = 1.0;

/// Suspicious scale: bbox diagonal above this (mm) suggests wrong units.
pub const SUSPICIOUS_MAX_DIAG_MM: f64 = 1000.0;

/// Triangle count above which PRT005 fires.
pub const EXCESSIVE_TRIANGLE_COUNT: usize = 2_000_000;

/// Floating-point bed contact epsilon in millimetres.
pub const BED_CONTACT_EPS_MM: f64 = 1e-3;

/// Contact-area face z-tolerance when scoring bed contact.
pub const CONTACT_Z_TOL_MM: f64 = 0.05;

/// Default tessellation tolerance for STEP in millimetres.
pub const DEFAULT_TESS_TOL_MM: f64 = 0.05;
