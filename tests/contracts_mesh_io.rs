//! Contract tests (LSP): every MeshReader must obey the same rules.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use rstest::rstest;

use k1fix::application::ports::{MeshReader, MeshWriter};
use k1fix::domain::mesh::IndexedMesh;
use k1fix::infrastructure::io::{
    mesh_to_faceted_step, StepReader, StlReader, StlWriter, ThreeMfReader, ThreeMfWriter,
};

fn assert_reader_roundtrip_volume<R: MeshReader, W: MeshWriter>(reader: R, writer: W) {
    let cube = IndexedMesh::unit_cube();
    let bytes = writer.write_bytes(&cube).expect("write");
    let back = reader.read_bytes(&bytes).expect("read");
    assert_eq!(back.triangle_count(), cube.triangle_count());
    assert!(
        (back.volume() - cube.volume()).abs() < 1e-2,
        "volume drift too large"
    );
}

#[rstest]
#[case::stl(StlReader, StlWriter)]
#[case::threemf(ThreeMfReader, ThreeMfWriter)]
fn mesh_reader_writer_contract_preserves_cube<R: MeshReader, W: MeshWriter>(
    #[case] reader: R,
    #[case] writer: W,
) {
    assert_reader_roundtrip_volume(reader, writer);
}

#[test]
fn step_reader_contract_faceted_roundtrip() {
    let cube = IndexedMesh::unit_cube();
    let step = mesh_to_faceted_step(&cube);
    let back = StepReader::default()
        .read_bytes(step.as_bytes())
        .expect("step");
    assert_eq!(back.triangle_count(), cube.triangle_count());
}

#[test]
fn mesh_reader_rejects_garbage() {
    assert!(StlReader.read_bytes(b"nope").is_err());
    assert!(ThreeMfReader.read_bytes(b"nope").is_err());
    assert!(StepReader::default().read_bytes(b"nope").is_err());
}
