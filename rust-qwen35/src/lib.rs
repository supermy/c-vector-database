//! Qwen35 Vector Database - High-performance vector database with SIMD optimization
//!
//! Features:
//! - SIMD-optimized distance calculations (AVX2/FMA)
//! - Thread-safe operations with fine-grained locking
//! - Memory-efficient storage using SmallVec
//! - Parallel search capabilities with Rayon
//! - Multiple distance metrics (Cosine, Euclidean, DotProduct, Manhattan)
//! - Serialization and deserialization support
//! - Statistics tracking

pub mod distance;
pub mod error;
pub mod vector_db;

pub use distance::DistanceMetric;
pub use error::{Error, Result};
pub use vector_db::{VectorDB, VectorEntry, SearchResult, Stats};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn create_db(dimension: usize, metric: DistanceMetric) -> VectorDB {
    VectorDB::new(dimension, metric)
}

pub fn create_db_with_capacity(dimension: usize, metric: DistanceMetric, capacity: usize) -> VectorDB {
    VectorDB::with_capacity(dimension, metric, capacity)
}
