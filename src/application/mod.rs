//! Application layer: use cases and ports.
//! Depends only on [`crate::domain`] — never on infrastructure.

pub mod convert;
pub mod fix;
pub mod inspect;
pub mod ports;
pub mod report;

pub use convert::ConvertUseCase;
pub use fix::{FixOptions, FixUseCase};
pub use inspect::{InspectOptions, InspectUseCase};
pub use ports::{
    Clock, Fs, MeshLoader, MeshReader, MeshStore, MeshWriter, NullProgress, ProgressSink,
    SystemClock,
};
pub use report::Report;
