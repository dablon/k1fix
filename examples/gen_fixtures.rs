//! Generate checked-in example meshes under `fixtures/`.
//! Run: `cargo run --example gen_fixtures`

use std::path::Path;

use k1fix::domain::mesh::IndexedMesh;
use k1fix::infrastructure::io::{mesh_to_faceted_step, write_mesh_bytes, MeshFormat};

fn main() {
    let root = Path::new("fixtures");
    std::fs::create_dir_all(root).expect("fixtures dir");

    let cube = IndexedMesh::unit_cube();
    let tray = IndexedMesh::kitchen_tray();
    let mut open = IndexedMesh::unit_cube();
    open.tris.retain(|t| *t != [4, 5, 6] && *t != [4, 6, 7]);

    write(
        root.join("cube.stl"),
        &write_mesh_bytes(MeshFormat::Stl, &cube).expect("stl"),
    );
    write(
        root.join("cube.3mf"),
        &write_mesh_bytes(MeshFormat::ThreeMf, &cube).expect("3mf"),
    );
    write(
        root.join("tray.stl"),
        &write_mesh_bytes(MeshFormat::Stl, &tray).expect("tray"),
    );
    write(
        root.join("open_cube.stl"),
        &write_mesh_bytes(MeshFormat::Stl, &open).expect("open"),
    );
    write(
        root.join("cube.step"),
        mesh_to_faceted_step(&cube).as_bytes(),
    );
    eprintln!("wrote fixtures to {}", root.display());
}

fn write(path: impl AsRef<Path>, bytes: &[u8]) {
    std::fs::write(path.as_ref(), bytes).unwrap_or_else(|e| {
        panic!("write {}: {e}", path.as_ref().display());
    });
}
