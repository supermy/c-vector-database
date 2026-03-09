//! Kimi25 Vector Database - High-performance vector database with HNSW index and SIMD optimization
//!
//! Features:
//! - HNSW (Hierarchical Navigable Small World) index for fast approximate nearest neighbor search
//! - SIMD-optimized distance calculations
//! - Thread-safe operations with fine-grained locking
//! - Memory-efficient storage
//! - Parallel search capabilities
//! - Multiple distance metrics (Cosine, Euclidean, DotProduct, Manhattan)
//! - High-performance persistence with LZ4 compression and memory mapping

pub mod distance;
pub mod error;
pub mod hnsw;
pub mod persistence;
pub mod vector_db;

pub use distance::DistanceMetric;
pub use error::{Error, Result};
pub use hnsw::{HnswIndex, NodeId};
pub use persistence::{
    Persistence, PersistenceConfig, CompressionType, SaveStats, LoadStats, Header
};
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

/// Save vector database to file with default config (LZ4 compression)
pub fn save_db<P: AsRef<std::path::Path>>(db: &VectorDB, path: P) -> Result<SaveStats> {
    Persistence::save(db, path, &PersistenceConfig::default())
}

/// Load vector database from file with default config
pub fn load_db<P: AsRef<std::path::Path>>(path: P) -> Result<(VectorDB, LoadStats)> {
    Persistence::load(path, &PersistenceConfig::default())
}

/// Save vector database without compression
pub fn save_db_uncompressed<P: AsRef<std::path::Path>>(db: &VectorDB, path: P) -> Result<SaveStats> {
    let config = PersistenceConfig {
        compression: CompressionType::None,
        use_mmap: true,
        verify_checksum: false,
    };
    Persistence::save(db, path, &config)
}
