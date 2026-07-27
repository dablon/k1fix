//! End-to-end CLI tests against the real binary.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use assert_cmd::assert::OutputAssertExt;
use assert_cmd::cargo::CommandCargoExt;
use assert_fs::prelude::*;
use assert_fs::TempDir;
use predicates::prelude::*;
use std::process::Command;

use k1fix::domain::mesh::IndexedMesh;
use k1fix::infrastructure::io::{write_mesh_bytes, MeshFormat};

fn write_stl(dir: &TempDir, name: &str, mesh: &IndexedMesh) -> assert_fs::fixture::ChildPath {
    let file = dir.child(name);
    let bytes = write_mesh_bytes(MeshFormat::Stl, mesh).expect("stl bytes");
    file.write_binary(&bytes).expect("write");
    file
}

#[test]
fn profiles_list_exits_zero() {
    let mut cmd = Command::cargo_bin("k1fix").unwrap();
    cmd.arg("profiles").arg("list");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("k1"));
}

#[test]
fn inspect_clean_cube_exit_ok_or_warn() {
    let dir = TempDir::new().unwrap();
    let stl = write_stl(&dir, "cube.stl", &IndexedMesh::unit_cube());
    let mut cmd = Command::cargo_bin("k1fix").unwrap();
    cmd.args(["inspect", stl.path().to_str().unwrap(), "--profile", "k1"]);
    cmd.assert().code(predicate::in_iter([0i32, 1]));
}

#[test]
fn inspect_open_mesh_reports_mesh001() {
    let dir = TempDir::new().unwrap();
    let mut mesh = IndexedMesh::unit_cube();
    mesh.tris.retain(|t| *t != [4, 5, 6] && *t != [4, 6, 7]);
    let stl = write_stl(&dir, "open.stl", &mesh);
    let json = dir.child("out.json");
    let mut cmd = Command::cargo_bin("k1fix").unwrap();
    cmd.args([
        "inspect",
        stl.path().to_str().unwrap(),
        "--json",
        json.path().to_str().unwrap(),
    ]);
    cmd.assert().code(predicate::in_iter([1i32, 2]));
    let body = std::fs::read_to_string(json.path()).unwrap();
    assert!(body.contains("MESH001"));
}

#[test]
fn fix_tray_fits_k1() {
    let dir = TempDir::new().unwrap();
    let stl = write_stl(&dir, "tray.stl", &IndexedMesh::kitchen_tray());
    let out = dir.child("fitted.stl");
    let mut cmd = Command::cargo_bin("k1fix").unwrap();
    cmd.args([
        "fix",
        stl.path().to_str().unwrap(),
        "-o",
        out.path().to_str().unwrap(),
        "--profile",
        "k1",
    ]);
    cmd.assert().code(predicate::in_iter([0i32, 1]));
    out.assert(predicate::path::exists());
}

#[test]
fn fix_dry_run_writes_nothing() {
    let dir = TempDir::new().unwrap();
    let stl = write_stl(&dir, "cube.stl", &IndexedMesh::unit_cube());
    let out = dir.child("nope.stl");
    let mut cmd = Command::cargo_bin("k1fix").unwrap();
    cmd.args([
        "fix",
        stl.path().to_str().unwrap(),
        "-o",
        out.path().to_str().unwrap(),
        "--dry-run",
    ]);
    cmd.assert().code(predicate::in_iter([0i32, 1]));
    out.assert(predicate::path::missing());
}

#[test]
fn fix_impossible_without_scale_exit_2() {
    let dir = TempDir::new().unwrap();
    let stl = write_stl(
        &dir,
        "huge.stl",
        &IndexedMesh::box_mesh(500.0, 500.0, 500.0),
    );
    let out = dir.child("out.stl");
    let mut cmd = Command::cargo_bin("k1fix").unwrap();
    cmd.args([
        "fix",
        stl.path().to_str().unwrap(),
        "-o",
        out.path().to_str().unwrap(),
    ]);
    cmd.assert().code(2);
}

#[test]
fn missing_file_exit_3() {
    let mut cmd = Command::cargo_bin("k1fix").unwrap();
    cmd.args(["inspect", "does-not-exist.stl"]);
    cmd.assert().code(3);
}

#[test]
fn unknown_profile_exit_3() {
    let dir = TempDir::new().unwrap();
    let stl = write_stl(&dir, "cube.stl", &IndexedMesh::unit_cube());
    let mut cmd = Command::cargo_bin("k1fix").unwrap();
    cmd.args(["inspect", stl.path().to_str().unwrap(), "--profile", "nope"]);
    cmd.assert().code(3);
}

#[test]
fn convert_stl_to_3mf() {
    let dir = TempDir::new().unwrap();
    let stl = write_stl(&dir, "cube.stl", &IndexedMesh::unit_cube());
    let out = dir.child("cube.3mf");
    let mut cmd = Command::cargo_bin("k1fix").unwrap();
    cmd.args([
        "convert",
        stl.path().to_str().unwrap(),
        "-o",
        out.path().to_str().unwrap(),
    ]);
    cmd.assert().success();
    out.assert(predicate::path::exists());
}
