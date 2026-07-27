//! STEP reader with lightweight tessellation.
//!
//! Full B-rep NURBS tessellation via the truck ecosystem is feature-gated and
//! complex; this reader extracts `CARTESIAN_POINT` + triangular `POLY_LOOP`
//! / `FACE_OUTER_BOUND` style faceted data common in 3D-print STEP exports,
//! and falls back to an explicit error for unsupported advanced B-rep.

use nalgebra::Point3;
use rustc_hash::FxHashMap;

use crate::application::ports::MeshReader;
use crate::domain::constants::DEFAULT_TESS_TOL_MM;
use crate::domain::error::{K1FixError, Result};
use crate::domain::mesh::IndexedMesh;

/// STEP mesh reader.
#[derive(Debug, Clone, Copy)]
pub struct StepReader {
    /// Tessellation tolerance in millimetres (reserved for future truck path).
    pub tess_tol: f64,
}

impl Default for StepReader {
    fn default() -> Self {
        Self {
            tess_tol: DEFAULT_TESS_TOL_MM,
        }
    }
}

impl MeshReader for StepReader {
    fn read_bytes(&self, bytes: &[u8]) -> Result<IndexedMesh> {
        let text = std::str::from_utf8(bytes)
            .map_err(|e| K1FixError::Step(format!("STEP is not UTF-8: {e}")))?;
        if !text.contains("ISO-10303") && !text.contains("HEADER") {
            return Err(K1FixError::Step(
                "not a STEP file (missing ISO-10303 header)".into(),
            ));
        }
        let _ = self.tess_tol;
        let (verts, tris) = rebuild_mesh(text)?;
        Ok(IndexedMesh::from_parts(verts, tris))
    }
}

fn rebuild_mesh(text: &str) -> Result<(Vec<Point3<f64>>, Vec<[u32; 3]>)> {
    let point_map = parse_cartesian_points(text);
    if point_map.is_empty() {
        return Err(K1FixError::Step("no CARTESIAN_POINT entities found".into()));
    }
    let mut id_to_dense: FxHashMap<i64, u32> = FxHashMap::default();
    let mut verts = Vec::new();
    let mut ids: Vec<i64> = point_map.keys().copied().collect();
    ids.sort_unstable();
    for id in ids {
        let dense = verts.len() as u32;
        id_to_dense.insert(id, dense);
        verts.push(point_map[&id]);
    }

    let mut tris = Vec::new();
    for caps in poly_loop_captures(text) {
        if caps.len() == 3 {
            if let (Some(&a), Some(&b), Some(&c)) = (
                id_to_dense.get(&caps[0]),
                id_to_dense.get(&caps[1]),
                id_to_dense.get(&caps[2]),
            ) {
                tris.push([a, b, c]);
            }
        } else if caps.len() > 3 {
            let mapped: Vec<u32> = caps
                .iter()
                .filter_map(|id| id_to_dense.get(id).copied())
                .collect();
            if mapped.len() >= 3 {
                for i in 1..mapped.len() - 1 {
                    tris.push([mapped[0], mapped[i], mapped[i + 1]]);
                }
            }
        }
    }

    for t in explicit_triangles(text) {
        if let (Some(&a), Some(&b), Some(&c)) = (
            id_to_dense.get(&t[0]),
            id_to_dense.get(&t[1]),
            id_to_dense.get(&t[2]),
        ) {
            tris.push([a, b, c]);
        }
    }

    if tris.is_empty() {
        return Err(K1FixError::Step(
            "STEP B-rep has no extractable triangular facets; export as STL/3MF or use a faceted STEP".into(),
        ));
    }
    Ok((verts, tris))
}

fn parse_cartesian_points(text: &str) -> FxHashMap<i64, Point3<f64>> {
    let mut map = FxHashMap::default();
    // #12=CARTESIAN_POINT('',(1.0,2.0,3.0));
    for line in text.split(';') {
        let upper = line.to_ascii_uppercase();
        if !upper.contains("CARTESIAN_POINT") {
            continue;
        }
        let Some(id) = parse_entity_id(line) else {
            continue;
        };
        if let Some(coords) = parse_paren_floats(line) {
            if coords.len() >= 3 {
                map.insert(id, Point3::new(coords[0], coords[1], coords[2]));
            }
        }
    }
    map
}

fn poly_loop_captures(text: &str) -> Vec<Vec<i64>> {
    let mut out = Vec::new();
    for chunk in text.split(';') {
        let upper = chunk.to_ascii_uppercase();
        if !upper.contains("POLY_LOOP") {
            continue;
        }
        // Collect all #id references after POLY_LOOP
        let ids = parse_hash_ids(chunk);
        if ids.len() >= 3 {
            out.push(ids);
        }
    }
    out
}

fn explicit_triangles(text: &str) -> Vec<[i64; 3]> {
    let mut out = Vec::new();
    for chunk in text.split(';') {
        let upper = chunk.to_ascii_uppercase();
        if !(upper.contains("TRIANGULATED") || upper.contains("TRIANGLE")) {
            continue;
        }
        let ids = parse_hash_ids(chunk);
        if ids.len() >= 3 {
            out.push([ids[0], ids[1], ids[2]]);
        }
    }
    out
}

fn parse_entity_id(line: &str) -> Option<i64> {
    let trimmed = line.trim_start();
    if !trimmed.starts_with('#') {
        return None;
    }
    let num: String = trimmed
        .chars()
        .skip(1)
        .take_while(|c| c.is_ascii_digit())
        .collect();
    num.parse().ok()
}

fn parse_hash_ids(text: &str) -> Vec<i64> {
    let mut ids = Vec::new();
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'#' {
            let start = i + 1;
            let mut end = start;
            while end < bytes.len() && bytes[end].is_ascii_digit() {
                end += 1;
            }
            if end > start {
                if let Ok(id) = std::str::from_utf8(&bytes[start..end])
                    .unwrap_or("")
                    .parse::<i64>()
                {
                    ids.push(id);
                }
            }
            i = end;
        } else {
            i += 1;
        }
    }
    ids
}

fn parse_paren_floats(text: &str) -> Option<Vec<f64>> {
    let start = text.rfind('(')?;
    let end = text.rfind(')')?;
    if end <= start {
        return None;
    }
    let inner = &text[start + 1..end];
    // May contain nested quotes; take the last numeric tuple-ish
    let mut vals = Vec::new();
    for part in inner.split(',') {
        let cleaned = part.trim().trim_matches(|c| c == '(' || c == ')');
        if let Ok(v) = cleaned.parse::<f64>() {
            vals.push(v);
        }
    }
    if vals.is_empty() {
        None
    } else {
        Some(vals)
    }
}

/// Build a minimal faceted STEP string for a triangle mesh (test helper / export aid).
#[must_use]
pub fn mesh_to_faceted_step(mesh: &IndexedMesh) -> String {
    let mut body = String::from(
        "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION(('k1fix'),'2;1');\nFILE_NAME('out.step','',('k1fix'),('k1fix'),'k1fix','k1fix','');\nFILE_SCHEMA(('AUTOMOTIVE_DESIGN'));\nENDSEC;\nDATA;\n",
    );
    let mut next_id: i64 = 1;
    let mut point_ids = Vec::new();
    for v in &mesh.verts {
        body.push_str(&format!(
            "#{next_id}=CARTESIAN_POINT('',({:.6},{:.6},{:.6}));\n",
            v.x, v.y, v.z
        ));
        point_ids.push(next_id);
        next_id += 1;
    }
    for t in &mesh.tris {
        body.push_str(&format!(
            "#{next_id}=POLY_LOOP('',(#{},#{},#{}));\n",
            point_ids[t[0] as usize], point_ids[t[1] as usize], point_ids[t[2] as usize]
        ));
        next_id += 1;
    }
    body.push_str("ENDSEC;\nEND-ISO-10303-21;\n");
    body
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::ports::MeshReader;

    #[test]
    fn reads_faceted_step_triangle() {
        let step = r#"ISO-10303-21;
HEADER;
ENDSEC;
DATA;
#1=CARTESIAN_POINT('',(0.0,0.0,0.0));
#2=CARTESIAN_POINT('',(1.0,0.0,0.0));
#3=CARTESIAN_POINT('',(0.0,1.0,0.0));
#4=POLY_LOOP('',(#1,#2,#3));
ENDSEC;
END-ISO-10303-21;
"#;
        let mesh = StepReader::default()
            .read_bytes(step.as_bytes())
            .expect("parse");
        assert_eq!(mesh.triangle_count(), 1);
        assert_eq!(mesh.vertex_count(), 3);
    }

    #[test]
    fn rejects_non_step() {
        assert!(StepReader::default().read_bytes(b"hello").is_err());
    }

    #[test]
    fn mesh_to_step_round_trip() {
        let cube = IndexedMesh::unit_cube();
        let step = mesh_to_faceted_step(&cube);
        let back = StepReader::default()
            .read_bytes(step.as_bytes())
            .expect("parse");
        assert_eq!(back.triangle_count(), cube.triangle_count());
    }
}
