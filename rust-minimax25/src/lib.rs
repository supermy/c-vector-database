pub mod distance;
pub mod vector_db;
pub mod error;

pub use distance::DistanceMetric;
pub use vector_db::{VectorDB, VectorEntry, SearchResult, Stats};
pub use error::{Error, Result};

pub const VERSION: &str = "1.3.0-production";
