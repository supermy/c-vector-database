//! GLM5 Vector Database - High-performance vector database with SIMD optimization
//!
//! Features:
//! - SIMD-optimized distance calculations
//! - Thread-safe operations with fine-grained locking
//! - Memory-efficient storage
//! - Parallel search capabilities

pub mod distance;
pub mod vector_db;
pub mod error;

pub use distance::DistanceMetric;
pub use vector_db::{VectorDB, VectorEntry, SearchResult, Stats};
pub use error::{Error, Result};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
