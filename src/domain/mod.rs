//! Domain layer: pure entities and business rules. No IO, no frameworks.

pub mod autofit;
pub mod constants;
pub mod diagnostics;
pub mod error;
pub mod mesh;
pub mod profiles;
pub mod repair;
pub mod spatial;
pub mod topology;

pub use error::{K1FixError, Result};
pub use mesh::IndexedMesh;
pub use profiles::{list_profiles, load_profile, PrinterProfile};
