//! Application use-case tests with in-memory ports.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::Path;

use k1fix::application::ports::{Fs, NullProgress};
use k1fix::application::{
    ConvertUseCase, FixOptions, FixUseCase, InspectOptions, InspectUseCase,
};
use k1fix::domain::mesh::IndexedMesh;
use k1fix::domain::profiles::load_profile;
use k1fix::infrastructure::io::{write_mesh_bytes, MeshFormat};
use k1fix::infrastructure::{FileMeshRepository, MemFs};

fn seed_stl(fs: &MemFs, path: &str, mesh: &IndexedMesh) {
    let bytes = write_mesh_bytes(MeshFormat::Stl, mesh).unwrap();
    fs.write(Path::new(path), &bytes).unwrap();
}

#[test]
fn inspect_use_case_on_open_mesh() {
    let fs = MemFs::new();
    let mut mesh = IndexedMesh::unit_cube();
    mesh.tris.retain(|t| *t != [4, 5, 6] && *t != [4, 6, 7]);
    seed_stl(&fs, "open.stl", &mesh);
    let repo = FileMeshRepository::new(fs);
    let profile = load_profile("k1").unwrap();
    let report = InspectUseCase::new(&repo, &NullProgress)
        .execute(
            Path::new("open.stl"),
            &profile,
            &InspectOptions::default(),
        )
        .unwrap();
    assert!(report.findings.iter().any(|f| f.id == "MESH001"));
    assert!(report.exit_code >= 1);
}

#[test]
fn fix_use_case_fits_tray_and_writes() {
    let fs = MemFs::new();
    seed_stl(&fs, "tray.stl", &IndexedMesh::kitchen_tray());
    let repo = FileMeshRepository::new(fs.clone());
    let profile = load_profile("k1").unwrap();
    let report = FixUseCase::new(&repo, &repo, &NullProgress)
        .execute(
            Path::new("tray.stl"),
            Path::new("out.stl"),
            &profile,
            &FixOptions::default(),
        )
        .unwrap();
    assert!(report.exit_code <= 1);
    assert!(fs.exists(Path::new("out.stl")));
}

#[test]
fn fix_dry_run_does_not_write() {
    let fs = MemFs::new();
    seed_stl(&fs, "cube.stl", &IndexedMesh::unit_cube());
    let repo = FileMeshRepository::new(fs.clone());
    let profile = load_profile("k1").unwrap();
    let report = FixUseCase::new(&repo, &repo, &NullProgress)
        .execute(
            Path::new("cube.stl"),
            Path::new("out.stl"),
            &profile,
            &FixOptions {
                dry_run: true,
                ..FixOptions::default()
            },
        )
        .unwrap();
    assert!(!fs.exists(Path::new("out.stl")));
    assert!(report.autofit.is_some());
}

#[test]
fn fix_impossible_sets_exit_2() {
    let fs = MemFs::new();
    seed_stl(&fs, "huge.stl", &IndexedMesh::box_mesh(500.0, 500.0, 500.0));
    let repo = FileMeshRepository::new(fs);
    let profile = load_profile("k1").unwrap();
    let report = FixUseCase::new(&repo, &repo, &NullProgress)
        .execute(
            Path::new("huge.stl"),
            Path::new("out.stl"),
            &profile,
            &FixOptions::default(),
        )
        .unwrap();
    assert_eq!(report.exit_code, 2);
}

#[test]
fn convert_stl_to_3mf_in_memory() {
    let fs = MemFs::new();
    seed_stl(&fs, "cube.stl", &IndexedMesh::unit_cube());
    let repo = FileMeshRepository::new(fs.clone());
    ConvertUseCase::new(&repo, &repo, &NullProgress)
        .execute(Path::new("cube.stl"), Path::new("cube.3mf"), 0.05)
        .unwrap();
    assert!(fs.exists(Path::new("cube.3mf")));
}
