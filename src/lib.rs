//! `k1fix` — diagnose, repair and auto-fit meshes for Creality K1 printers.
//!
//! # Architecture
//!
//! Clean Architecture layers (dependencies point inward only):
//!
//! - [`domain`] — entities and business rules (no IO)
//! - [`application`] — use cases and ports (no infrastructure)
//! - [`infrastructure`] — adapters (STL/3MF/STEP, filesystem)
//! - [`presentation`] — CLI wiring
//!
#![cfg_attr(
    not(test),
    deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)
)]

pub mod application;
pub mod domain;
pub mod infrastructure;
pub mod presentation;

pub use application::{FixOptions, InspectOptions, Report};
pub use domain::error::{K1FixError, Result};
pub use domain::mesh::IndexedMesh;
pub use domain::profiles::{list_profiles, load_profile, PrinterProfile};
