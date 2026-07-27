//! Infrastructure adapters: implement application ports with real IO / libraries.
//! May depend on application ports and domain — never the reverse.

pub mod fs;
pub mod io;
pub mod mem_fs;
pub mod mesh_repository;
pub mod progress;

pub use fs::StdFsAdapter;
pub use io::{read_mesh_bytes, read_mesh_file, write_mesh_bytes, write_mesh_file, MeshFormat};
pub use mem_fs::MemFs;
pub use mesh_repository::FileMeshRepository;
pub use progress::{SilentProgress, StderrProgress};
