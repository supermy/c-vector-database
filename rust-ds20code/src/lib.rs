pub mod distance;
pub mod vector_db;
pub mod hnsw;
pub mod error;

pub use distance::DistanceMetric;
pub use vector_db::{VectorDB, VectorEntry, SearchResult, Stats, IndexType};
pub use hnsw::HnswIndex;
pub use error::{Error, Result};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
