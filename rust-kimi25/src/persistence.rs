use std::fs::{File, OpenOptions};
use std::io::{Read, Write, BufReader, BufWriter};
use std::path::Path;
use std::time::Instant;
use serde::{Serialize, Deserialize};
use memmap2::Mmap;

use crate::distance::DistanceMetric;
use crate::error::{Error, Result};
use crate::vector_db::{VectorDB, VectorEntry};
use crate::hnsw::NodeId;

pub const CURRENT_VERSION: u32 = 2;
pub const MAGIC_NUMBER: &[u8; 8] = b"KIMI25DB";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressionType {
    None,
    Lz4,
}

impl Default for CompressionType {
    fn default() -> Self {
        Self::Lz4
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Header {
    pub magic: [u8; 8],
    pub version: u32,
    pub dimension: usize,
    pub metric: DistanceMetric,
    pub entry_count: u64,
    pub hnsw_m: usize,
    pub hnsw_ef_construction: usize,
    pub hnsw_ef_search: usize,
    pub compression: u8,
    pub data_offset: u64,
    pub index_offset: u64,
    pub checksum: u64,
}

impl Header {
    pub fn new(
        dimension: usize,
        metric: DistanceMetric,
        entry_count: u64,
        hnsw_m: usize,
        hnsw_ef_construction: usize,
        hnsw_ef_search: usize,
        compression: CompressionType,
    ) -> Self {
        let compression_flag = match compression {
            CompressionType::None => 0,
            CompressionType::Lz4 => 1,
        };

        Self {
            magic: *MAGIC_NUMBER,
            version: CURRENT_VERSION,
            dimension,
            metric,
            entry_count,
            hnsw_m,
            hnsw_ef_construction,
            hnsw_ef_search,
            compression: compression_flag,
            data_offset: 0,
            index_offset: 0,
            checksum: 0,
        }
    }

    pub fn validate(&self) -> Result<()> {
        if &self.magic != MAGIC_NUMBER {
            return Err(Error::SerializationError(
                format!("Invalid magic number: expected {:?}, got {:?}", MAGIC_NUMBER, &self.magic)
            ));
        }
        if self.version != CURRENT_VERSION {
            return Err(Error::SerializationError(
                format!("Unsupported version: expected {}, got {}", CURRENT_VERSION, self.version)
            ));
        }
        Ok(())
    }

    pub fn get_compression(&self) -> CompressionType {
        match self.compression {
            0 => CompressionType::None,
            1 => CompressionType::Lz4,
            _ => CompressionType::Lz4,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializableEntry {
    pub id: u64,
    pub vector: Vec<f32>,
    pub metadata: Option<Vec<u8>>,
}

impl From<&VectorEntry> for SerializableEntry {
    fn from(entry: &VectorEntry) -> Self {
        Self {
            id: entry.id,
            vector: entry.vector.clone(),
            metadata: entry.metadata.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializableNode {
    pub id: NodeId,
    pub external_id: u64,
    pub vector: Vec<f32>,
    pub level: usize,
    pub neighbors: Vec<Vec<NodeId>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializableHnsw {
    pub nodes: Vec<SerializableNode>,
    pub id_to_idx: Vec<(u64, NodeId)>,
    pub entry_point: usize,
    pub max_level: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseData {
    pub entries: Vec<SerializableEntry>,
    pub hnsw: SerializableHnsw,
}

pub struct PersistenceConfig {
    pub compression: CompressionType,
    pub use_mmap: bool,
    pub verify_checksum: bool,
}

impl Default for PersistenceConfig {
    fn default() -> Self {
        Self {
            compression: CompressionType::Lz4,
            use_mmap: true,
            verify_checksum: false,
        }
    }
}

pub struct Persistence;

impl Persistence {
    /// Save database to file with high performance
    pub fn save<P: AsRef<Path>>(
        db: &VectorDB,
        path: P,
        config: &PersistenceConfig,
    ) -> Result<SaveStats> {
        let start = Instant::now();
        let path = path.as_ref();

        // Collect entries efficiently
        let entries: Vec<SerializableEntry> = db
            .get_all_entries()
            .iter()
            .map(|e| e.into())
            .collect();

        let entry_count = entries.len() as u64;

        // Serialize HNSW index
        let hnsw_data = db.serialize_hnsw();

        // Create database data
        let db_data = DatabaseData {
            entries,
            hnsw: hnsw_data,
        };

        // Serialize to bytes
        let data_bytes = bincode::serialize(&db_data)
            .map_err(|e| Error::SerializationError(e.to_string()))?;

        let uncompressed_size = data_bytes.len();

        // Compress if needed
        let (final_bytes, _compressed_size) = match config.compression {
            CompressionType::None => (data_bytes, uncompressed_size),
            CompressionType::Lz4 => {
                let compressed = lz4::block::compress(&data_bytes, None, true)
                    .map_err(|e| Error::SerializationError(format!("LZ4 compression failed: {}", e)))?;
                (compressed, uncompressed_size)
            }
        };

        // Calculate checksum
        let checksum = if config.verify_checksum {
            Self::calculate_checksum(&final_bytes)
        } else {
            0
        };

        // Create header
        let mut header = Header::new(
            db.dimension(),
            db.metric(),
            entry_count,
            db.hnsw_m(),
            db.hnsw_ef_construction(),
            db.hnsw_ef_search(),
            config.compression,
        );
        header.checksum = checksum;

        // Open file
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(path)
            .map_err(|e| Error::SerializationError(format!("Failed to create file: {}", e)))?;

        let mut writer = BufWriter::with_capacity(1024 * 1024, file);

        // Write header
        let header_bytes = bincode::serialize(&header)
            .map_err(|e| Error::SerializationError(e.to_string()))?;
        writer.write_all(&header_bytes)
            .map_err(|e| Error::SerializationError(format!("Failed to write header: {}", e)))?;

        // Write data
        writer.write_all(&final_bytes)
            .map_err(|e| Error::SerializationError(format!("Failed to write data: {}", e)))?;

        writer.flush()
            .map_err(|e| Error::SerializationError(format!("Failed to flush: {}", e)))?;

        let elapsed = start.elapsed();

        Ok(SaveStats {
            duration: elapsed,
            entries_saved: entry_count as usize,
            bytes_written: header_bytes.len() + final_bytes.len(),
            uncompressed_size,
            compression_ratio: if config.compression != CompressionType::None {
                1.0 - (final_bytes.len() as f64 / uncompressed_size as f64)
            } else {
                0.0
            },
        })
    }

    /// Load database from file with optional memory mapping
    pub fn load<P: AsRef<Path>>(path: P, config: &PersistenceConfig) -> Result<(VectorDB, LoadStats)> {
        let start = Instant::now();
        let path = path.as_ref();

        if config.use_mmap {
            Self::load_mmap(path, config)
        } else {
            Self::load_buffered(path, config)
        }
    }

    fn load_buffered(path: &Path, config: &PersistenceConfig) -> Result<(VectorDB, LoadStats)> {
        let start = Instant::now();
        let file = File::open(path)
            .map_err(|e| Error::SerializationError(format!("Failed to open file: {}", e)))?;

        let mut reader = BufReader::with_capacity(1024 * 1024, file);

        // Read header
        let header: Header = bincode::deserialize_from(&mut reader)
            .map_err(|e| Error::SerializationError(format!("Failed to read header: {}", e)))?;
        header.validate()?;

        // Read data
        let mut data_buffer = Vec::new();
        reader.read_to_end(&mut data_buffer)
            .map_err(|e| Error::SerializationError(format!("Failed to read data: {}", e)))?;

        // Verify checksum
        if config.verify_checksum && header.checksum != 0 {
            let checksum = Self::calculate_checksum(&data_buffer);
            if checksum != header.checksum {
                return Err(Error::SerializationError("Checksum mismatch".to_string()));
            }
        }

        // Decompress if needed
        let decompressed = match header.get_compression() {
            CompressionType::None => data_buffer,
            CompressionType::Lz4 => {
                lz4::block::decompress(&data_buffer, None)
                    .map_err(|e| Error::SerializationError(format!("LZ4 decompression failed: {}", e)))?
            }
        };

        // Deserialize
        let db_data: DatabaseData = bincode::deserialize(&decompressed)
            .map_err(|e| Error::SerializationError(format!("Failed to deserialize data: {}", e)))?;

        // Create database
        let db = VectorDB::with_hnsw_params(
            header.dimension,
            header.metric,
            header.hnsw_m,
            header.hnsw_ef_construction,
            header.hnsw_ef_search,
        );

        // Insert entries in batches for better performance
        for entry in db_data.entries {
            db.insert(entry.id, &entry.vector, entry.metadata)?;
        }

        let elapsed = start.elapsed();

        let stats = LoadStats {
            duration: elapsed,
            entries_loaded: db.len(),
            bytes_read: std::fs::metadata(path)?.len() as usize,
        };

        Ok((db, stats))
    }

    fn load_mmap(path: &Path, config: &PersistenceConfig) -> Result<(VectorDB, LoadStats)> {
        let start = Instant::now();
        let file = File::open(path)
            .map_err(|e| Error::SerializationError(format!("Failed to open file: {}", e)))?;

        // Memory map the file
        let mmap = unsafe { Mmap::map(&file) }
            .map_err(|e| Error::SerializationError(format!("Failed to mmap file: {}", e)))?;

        // Read header from mmap
        let header_size = bincode::serialized_size(&Header {
            magic: *MAGIC_NUMBER,
            version: CURRENT_VERSION,
            dimension: 0,
            metric: DistanceMetric::Cosine,
            entry_count: 0,
            hnsw_m: 0,
            hnsw_ef_construction: 0,
            hnsw_ef_search: 0,
            compression: 0,
            data_offset: 0,
            index_offset: 0,
            checksum: 0,
        }).map_err(|e| Error::SerializationError(e.to_string()))? as usize;

        let header: Header = bincode::deserialize(&mmap[..header_size])
            .map_err(|e| Error::SerializationError(format!("Failed to read header: {}", e)))?;
        header.validate()?;

        // Get data slice
        let data_slice = &mmap[header_size..];

        // Verify checksum
        if config.verify_checksum && header.checksum != 0 {
            let checksum = Self::calculate_checksum(data_slice);
            if checksum != header.checksum {
                return Err(Error::SerializationError("Checksum mismatch".to_string()));
            }
        }

        // Decompress if needed
        let decompressed = match header.get_compression() {
            CompressionType::None => data_slice.to_vec(),
            CompressionType::Lz4 => {
                lz4::block::decompress(data_slice, None)
                    .map_err(|e| Error::SerializationError(format!("LZ4 decompression failed: {}", e)))?
            }
        };

        // Deserialize
        let db_data: DatabaseData = bincode::deserialize(&decompressed)
            .map_err(|e| Error::SerializationError(format!("Failed to deserialize data: {}", e)))?;

        // Create database
        let db = VectorDB::with_hnsw_params(
            header.dimension,
            header.metric,
            header.hnsw_m,
            header.hnsw_ef_construction,
            header.hnsw_ef_search,
        );

        // Insert entries
        for entry in db_data.entries {
            db.insert(entry.id, &entry.vector, entry.metadata)?;
        }

        let elapsed = start.elapsed();

        let stats = LoadStats {
            duration: elapsed,
            entries_loaded: db.len(),
            bytes_read: mmap.len(),
        };

        Ok((db, stats))
    }

    /// Quick file info without loading
    pub fn get_file_info<P: AsRef<Path>>(path: P) -> Result<Header> {
        let file = File::open(path)
            .map_err(|e| Error::SerializationError(format!("Failed to open file: {}", e)))?;
        let mut reader = BufReader::new(file);

        let header: Header = bincode::deserialize_from(&mut reader)
            .map_err(|e| Error::SerializationError(format!("Failed to read header: {}", e)))?;
        header.validate()?;

        Ok(header)
    }

    /// Calculate simple checksum
    fn calculate_checksum(data: &[u8]) -> u64 {
        let mut checksum: u64 = 0;
        for chunk in data.chunks(8) {
            let mut value: u64 = 0;
            for (i, &byte) in chunk.iter().enumerate() {
                value |= (byte as u64) << (i * 8);
            }
            checksum = checksum.wrapping_add(value);
            checksum = checksum.rotate_left(13);
        }
        checksum
    }
}

#[derive(Debug, Clone)]
pub struct SaveStats {
    pub duration: std::time::Duration,
    pub entries_saved: usize,
    pub bytes_written: usize,
    pub uncompressed_size: usize,
    pub compression_ratio: f64,
}

#[derive(Debug, Clone)]
pub struct LoadStats {
    pub duration: std::time::Duration,
    pub entries_loaded: usize,
    pub bytes_read: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_save_and_load() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.db");

        let db = VectorDB::new(128, DistanceMetric::Cosine);
        for i in 0..100 {
            let vector: Vec<f32> = (0..128).map(|j| (i * 128 + j) as f32 / 10000.0).collect();
            db.insert(i as u64, &vector, Some(vec![i as u8])).unwrap();
        }

        let config = PersistenceConfig {
            compression: CompressionType::None,
            use_mmap: false,
            verify_checksum: false,
        };

        let save_stats = Persistence::save(&db, &file_path, &config).unwrap();
        println!("Save stats: {:?}", save_stats);

        let (loaded_db, load_stats) = Persistence::load(&file_path, &config).unwrap();
        println!("Load stats: {:?}", load_stats);

        assert_eq!(loaded_db.len(), 100);
        assert_eq!(loaded_db.dimension(), 128);

        for i in 0..100 {
            let entry = loaded_db.get(i as u64).unwrap();
            assert_eq!(entry.id, i as u64);
        }

        let query: Vec<f32> = (0..128).map(|j| j as f32 / 10000.0).collect();
        let results = loaded_db.search(&query, 5).unwrap();
        assert_eq!(results.len(), 5);
    }

    #[test]
    fn test_lz4_compression() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test_compressed.db");

        let db = VectorDB::new(128, DistanceMetric::Cosine);
        for i in 0..1000 {
            let vector: Vec<f32> = (0..128).map(|j| (i * 128 + j) as f32 / 10000.0).collect();
            db.insert(i as u64, &vector, Some(vec![0u8; 100])).unwrap();
        }

        let config = PersistenceConfig {
            compression: CompressionType::Lz4,
            use_mmap: false,
            verify_checksum: false,
        };

        let save_stats = Persistence::save(&db, &file_path, &config).unwrap();
        println!("LZ4 compression ratio: {:.2}%", save_stats.compression_ratio * 100.0);

        let (loaded_db, _) = Persistence::load(&file_path, &config).unwrap();
        assert_eq!(loaded_db.len(), 1000);

        let query: Vec<f32> = (0..128).map(|j| j as f32 / 10000.0).collect();
        let results = loaded_db.search(&query, 5).unwrap();
        assert_eq!(results.len(), 5);
    }

    #[test]
    fn test_mmap_loading() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test_mmap.db");

        let db = VectorDB::new(128, DistanceMetric::Cosine);
        for i in 0..100 {
            let vector: Vec<f32> = (0..128).map(|j| (i * 128 + j) as f32 / 10000.0).collect();
            db.insert(i as u64, &vector, None).unwrap();
        }

        let save_config = PersistenceConfig::default();
        Persistence::save(&db, &file_path, &save_config).unwrap();

        let load_config = PersistenceConfig {
            compression: CompressionType::Lz4,
            use_mmap: true,
            verify_checksum: false,
        };

        let (loaded_db, stats) = Persistence::load(&file_path, &load_config).unwrap();
        println!("MMAP load stats: {:?}", stats);
        assert_eq!(loaded_db.len(), 100);
    }

    #[test]
    fn test_file_info() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test_info.db");

        let db = VectorDB::new(256, DistanceMetric::Euclidean);
        Persistence::save(&db, &file_path, &PersistenceConfig::default()).unwrap();

        let info = Persistence::get_file_info(&file_path).unwrap();
        assert_eq!(info.dimension, 256);
        assert_eq!(info.metric, DistanceMetric::Euclidean);
        assert_eq!(info.entry_count, 0);
    }

    #[test]
    fn test_compression_comparison() {
        let temp_dir = TempDir::new().unwrap();
        let uncompressed_path = temp_dir.path().join("uncompressed.db");
        let lz4_path = temp_dir.path().join("lz4.db");

        let db = VectorDB::new(128, DistanceMetric::Cosine);
        for i in 0..1000 {
            let vector: Vec<f32> = (0..128).map(|j| (i * 128 + j) as f32 / 10000.0).collect();
            db.insert(i as u64, &vector, Some(vec![0u8; 100])).unwrap();
        }

        // Save uncompressed
        let no_compression = PersistenceConfig {
            compression: CompressionType::None,
            use_mmap: false,
            verify_checksum: false,
        };
        Persistence::save(&db, &uncompressed_path, &no_compression).unwrap();
        let uncompressed_size = std::fs::metadata(&uncompressed_path).unwrap().len();

        // Save with LZ4
        let lz4_config = PersistenceConfig {
            compression: CompressionType::Lz4,
            use_mmap: false,
            verify_checksum: false,
        };
        Persistence::save(&db, &lz4_path, &lz4_config).unwrap();
        let lz4_size = std::fs::metadata(&lz4_path).unwrap().len();

        println!("Uncompressed: {} bytes", uncompressed_size);
        println!("LZ4: {} bytes", lz4_size);
        println!("LZ4 compression ratio: {:.2}%", (1.0 - lz4_size as f64 / uncompressed_size as f64) * 100.0);

        assert!(lz4_size < uncompressed_size, "LZ4 compressed file should be smaller");
    }
}
