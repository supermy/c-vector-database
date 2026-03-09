//! Kimi25 Vector Database - High-performance vector database with HNSW index and SIMD optimization
//!
//! Features:
//! - HNSW (Hierarchical Navigable Small World) index for fast approximate nearest neighbor search
//! - SIMD-optimized distance calculations
//! - Thread-safe operations with fine-grained locking
//! - Memory-efficient storage
//! - Parallel search capabilities
//! - Multiple distance metrics (Cosine, Euclidean, DotProduct, Manhattan)

pub mod distance;
pub mod error;
pub mod hnsw;
pub mod vector_db;

pub use distance::DistanceMetric;
pub use error::{Error, Result};
pub use hnsw::{HnswIndex, NodeId};
pub use vector_db::{VectorDB, VectorEntry, SearchResult, Stats, Vector};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn create_db(dimension: usize, metric: DistanceMetric) -> VectorDB {
    VectorDB::new(dimension, metric)
}

pub fn create_db_with_hnsw(
    dimension: usize,
    metric: DistanceMetric,
    m: usize,
    ef_construction: usize,
    ef_search: usize,
) -> VectorDB {
    VectorDB::with_hnsw_params(dimension, metric, m, ef_construction, ef_search)
}
