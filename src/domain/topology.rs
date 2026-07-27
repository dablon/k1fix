//! Mesh topology: edge adjacency, boundary loops, connected components.

use rustc_hash::FxHashMap;
use smallvec::SmallVec;

use crate::domain::mesh::IndexedMesh;

/// Face identifier (index into `IndexedMesh::tris`).
pub type FaceId = u32;

/// Canonical undirected edge key (min, max).
#[must_use]
pub fn edge_key(a: u32, b: u32) -> (u32, u32) {
    if a < b {
        (a, b)
    } else {
        (b, a)
    }
}

/// Edge-to-face adjacency map.
#[derive(Debug, Clone, Default)]
pub struct EdgeMap {
    /// Faces incident to each undirected edge.
    pub edges: FxHashMap<(u32, u32), SmallVec<[FaceId; 2]>>,
}

impl EdgeMap {
    /// Build the edge map from a mesh.
    #[must_use]
    pub fn build(mesh: &IndexedMesh) -> Self {
        let mut edges: FxHashMap<(u32, u32), SmallVec<[FaceId; 2]>> = FxHashMap::default();
        for (fi, tri) in mesh.tris.iter().enumerate() {
            let face = fi as FaceId;
            for (i, j) in [(0, 1), (1, 2), (2, 0)] {
                let key = edge_key(tri[i], tri[j]);
                edges.entry(key).or_default().push(face);
            }
        }
        Self { edges }
    }

    /// Boundary edges (exactly one incident face).
    #[must_use]
    pub fn boundary_edges(&self) -> Vec<(u32, u32)> {
        self.edges
            .iter()
            .filter(|(_, faces)| faces.len() == 1)
            .map(|(e, _)| *e)
            .collect()
    }

    /// Non-manifold edges (more than two incident faces).
    #[must_use]
    pub fn nonmanifold_edges(&self) -> Vec<(u32, u32)> {
        self.edges
            .iter()
            .filter(|(_, faces)| faces.len() > 2)
            .map(|(e, _)| *e)
            .collect()
    }

    /// Count of open (boundary) edges.
    #[must_use]
    pub fn open_edge_count(&self) -> usize {
        self.boundary_edges().len()
    }
}

/// Topology queries over a mesh.
pub trait Topology {
    /// Build edge adjacency.
    fn edge_map(&self) -> EdgeMap;
    /// Boundary loops as ordered vertex rings.
    fn boundary_loops(&self) -> Vec<Vec<u32>>;
    /// Face-connected components (lists of face ids).
    fn connected_components(&self) -> Vec<Vec<FaceId>>;
}

impl Topology for IndexedMesh {
    fn edge_map(&self) -> EdgeMap {
        EdgeMap::build(self)
    }

    fn boundary_loops(&self) -> Vec<Vec<u32>> {
        extract_boundary_loops(self)
    }

    fn connected_components(&self) -> Vec<Vec<FaceId>> {
        face_connected_components(self)
    }
}

/// Extract closed boundary loops by walking boundary half-edges.
#[must_use]
pub fn extract_boundary_loops(mesh: &IndexedMesh) -> Vec<Vec<u32>> {
    let edge_map = EdgeMap::build(mesh);
    // Directed boundary half-edges from the owning face winding.
    let mut half: FxHashMap<u32, Vec<u32>> = FxHashMap::default();
    for (fi, tri) in mesh.tris.iter().enumerate() {
        for (i, j) in [(0usize, 1usize), (1, 2), (2, 0)] {
            let a = tri[i];
            let b = tri[j];
            let key = edge_key(a, b);
            if edge_map.edges.get(&key).is_some_and(|f| f.len() == 1) {
                half.entry(a).or_default().push(b);
            }
        }
        let _ = fi;
    }

    let mut visited: FxHashMap<(u32, u32), bool> = FxHashMap::default();
    let mut loops = Vec::new();

    let starts: Vec<(u32, u32)> = half
        .iter()
        .flat_map(|(a, bs)| bs.iter().map(move |b| (*a, *b)))
        .collect();

    for (start_a, start_b) in starts {
        if *visited.get(&(start_a, start_b)).unwrap_or(&false) {
            continue;
        }
        let mut loop_verts = vec![start_a];
        let mut cur = start_a;
        let mut next = start_b;
        loop {
            visited.insert((cur, next), true);
            loop_verts.push(next);
            let Some(candidates) = half.get(&next) else {
                break;
            };
            let mut advanced = false;
            for &cand in candidates {
                if !*visited.get(&(next, cand)).unwrap_or(&false) {
                    cur = next;
                    next = cand;
                    advanced = true;
                    break;
                }
            }
            if !advanced || next == start_a {
                if next == start_a {
                    // closed
                }
                break;
            }
            if loop_verts.len() > mesh.verts.len() + 2 {
                break;
            }
        }
        if loop_verts.len() >= 3 {
            // drop duplicate closing vertex if present
            if loop_verts.first() == loop_verts.last() {
                loop_verts.pop();
            }
            loops.push(loop_verts);
        }
    }
    loops
}

/// Face connectivity via shared edges (manifold or not).
#[must_use]
pub fn face_connected_components(mesh: &IndexedMesh) -> Vec<Vec<FaceId>> {
    let edge_map = EdgeMap::build(mesh);
    let n = mesh.tris.len();
    let mut adj: Vec<Vec<FaceId>> = vec![Vec::new(); n];
    for faces in edge_map.edges.values() {
        for i in 0..faces.len() {
            for j in (i + 1)..faces.len() {
                adj[faces[i] as usize].push(faces[j]);
                adj[faces[j] as usize].push(faces[i]);
            }
        }
    }
    let mut seen = vec![false; n];
    let mut comps = Vec::new();
    for start in 0..n {
        if seen[start] {
            continue;
        }
        let mut stack = vec![start as FaceId];
        let mut comp = Vec::new();
        seen[start] = true;
        while let Some(f) = stack.pop() {
            comp.push(f);
            for &nface in &adj[f as usize] {
                if !seen[nface as usize] {
                    seen[nface as usize] = true;
                    stack.push(nface);
                }
            }
        }
        comps.push(comp);
    }
    comps
}

/// Cube with the top face removed (open boundary of 4 edges / 1 loop).
#[must_use]
pub fn cube_missing_top() -> IndexedMesh {
    let mut cube = IndexedMesh::unit_cube();
    // Remove top triangles (outward +Z): [4,5,6] and [4,6,7].
    cube.tris.retain(|t| *t != [4, 5, 6] && *t != [4, 6, 7]);
    cube
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn closed_cube_has_no_boundary() {
        let cube = IndexedMesh::unit_cube();
        let map = EdgeMap::build(&cube);
        assert_eq!(map.open_edge_count(), 0);
        assert!(map.nonmanifold_edges().is_empty());
        assert!(cube.boundary_loops().is_empty());
        assert_eq!(cube.connected_components().len(), 1);
    }

    #[test]
    fn cube_missing_top_has_one_boundary_loop() {
        let mesh = cube_missing_top();
        let map = EdgeMap::build(&mesh);
        assert_eq!(map.open_edge_count(), 4);
        let loops = mesh.boundary_loops();
        assert_eq!(loops.len(), 1);
        assert_eq!(loops[0].len(), 4);
    }

    #[test]
    fn two_disjoint_cubes_two_components() {
        let mut a = IndexedMesh::unit_cube();
        let mut b = IndexedMesh::unit_cube();
        b.translate(nalgebra::Vector3::new(5.0, 0.0, 0.0));
        let offset = a.verts.len() as u32;
        a.verts.extend(b.verts);
        a.tris.extend(
            b.tris
                .into_iter()
                .map(|t| [t[0] + offset, t[1] + offset, t[2] + offset]),
        );
        assert_eq!(a.connected_components().len(), 2);
    }

    #[test]
    fn edge_key_is_ordered() {
        assert_eq!(edge_key(3, 1), (1, 3));
        assert_eq!(edge_key(1, 3), (1, 3));
    }

    #[test]
    fn nonmanifold_edge_detected() {
        // Three triangles sharing the same edge (0,1).
        let mesh = IndexedMesh::from_parts(
            vec![
                nalgebra::Point3::origin(),
                nalgebra::Point3::new(1.0, 0.0, 0.0),
                nalgebra::Point3::new(0.5, 1.0, 0.0),
                nalgebra::Point3::new(0.5, -1.0, 0.0),
                nalgebra::Point3::new(0.5, 0.0, 1.0),
            ],
            vec![[0, 1, 2], [0, 1, 3], [0, 1, 4]],
        );
        let nm = EdgeMap::build(&mesh).nonmanifold_edges();
        assert!(nm.contains(&(0, 1)));
    }
}
