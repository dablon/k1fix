//! Printer profiles embedded at compile time.

use serde::{Deserialize, Serialize};

use crate::domain::error::{K1FixError, Result};

/// Printer build-volume profile.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PrinterProfile {
    /// Short id (`k1`, `k1c`, `k1max`).
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// Bed size X in millimetres.
    pub bed_x_mm: f64,
    /// Bed size Y in millimetres.
    pub bed_y_mm: f64,
    /// Build height Z in millimetres.
    pub build_z_mm: f64,
    /// Nozzle diameter in millimetres.
    pub nozzle_mm: f64,
    /// Default layer height in millimetres.
    pub layer_height_mm: f64,
    /// Default margin from bed edge in millimetres.
    pub default_margin_mm: f64,
}

impl PrinterProfile {
    /// Usable bed width after margins.
    #[must_use]
    pub fn usable_x(&self, margin: f64) -> f64 {
        (self.bed_x_mm - 2.0 * margin).max(0.0)
    }

    /// Usable bed depth after margins.
    #[must_use]
    pub fn usable_y(&self, margin: f64) -> f64 {
        (self.bed_y_mm - 2.0 * margin).max(0.0)
    }

    /// Usable bed area as (x, y).
    #[must_use]
    pub fn usable_bed_area(&self, margin: f64) -> (f64, f64) {
        (self.usable_x(margin), self.usable_y(margin))
    }
}

const PROFILES: &[(&str, &str)] = &[
    ("k1", include_str!("../../profiles/k1.toml")),
    ("k1c", include_str!("../../profiles/k1c.toml")),
    ("k1max", include_str!("../../profiles/k1max.toml")),
];

/// List all embedded profiles.
#[must_use]
pub fn list_profiles() -> Vec<PrinterProfile> {
    PROFILES
        .iter()
        .filter_map(|(_, raw)| toml::from_str::<PrinterProfile>(raw).ok())
        .collect()
}

/// Load a profile by id.
///
/// # Errors
/// Returns [`K1FixError::UnknownProfile`] when the id is not embedded.
pub fn load_profile(id: &str) -> Result<PrinterProfile> {
    for (pid, raw) in PROFILES {
        if *pid == id {
            return Ok(toml::from_str(raw)?);
        }
    }
    Err(K1FixError::UnknownProfile(id.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_k1() {
        let p = load_profile("k1").expect("k1");
        assert_eq!(p.bed_x_mm, 220.0);
        assert_eq!(p.build_z_mm, 250.0);
        let (ux, uy) = p.usable_bed_area(3.0);
        assert!((ux - 214.0).abs() < 1e-9);
        assert!((uy - 214.0).abs() < 1e-9);
    }

    #[test]
    fn loads_k1max() {
        let p = load_profile("k1max").expect("k1max");
        assert_eq!(p.bed_x_mm, 300.0);
    }

    #[test]
    fn unknown_profile_errors() {
        assert!(load_profile("nope").is_err());
    }

    #[test]
    fn lists_three() {
        assert_eq!(list_profiles().len(), 3);
    }
}
