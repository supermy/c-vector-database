use crate::distance::DistanceMetric;
use crate::error::{Error, Result};
use crate::vector_db::{Stats, VectorEntry};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use lz4_flex::{compress_prepend_size, decompress_size_prepended};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DatabaseMetadata {
    version: u32,
    dimension: usize,
    metric: String,
    size: usize,
    compressed: bool,
    timestamp: u64,
}

pub struct VectorDB {
    entries: Arc<RwLock<Vec<VectorEntry>>>,
    id_map: Arc<RwLock<HashMap<i64, usize>>>,
    dimension: usize,
    metric: DistanceMetric,
    normalized: bool,
    stats: Arc<RwLock<Stats>>,
    enable_stats: bool,
}

impl VectorDB {
    pub fn new(dimension: usize, metric: DistanceMetric) -> Self {
        VectorDB {
            entries: Arc::new(RwLock::new(Vec::new())),
            id_map: Arc::new(RwLock::new(HashMap::new())),
            dimension,
            metric,
            normalized: metric == DistanceMetric::Cosine,
            stats: Arc::new(RwLock::new(Stats::default())),
            enable_stats: false,
        }
    }

    pub fn with_capacity(dimension: usize, metric: DistanceMetric, capacity: usize) -> Self {
        let db = VectorDB::new(dimension, metric);
        db.entries.write().reserve(capacity);
        db.id_map.write().reserve(capacity);
        db
    }

    pub fn dimension(&self) -> usize {
        self.dimension
    }

    pub fn metric(&self) -> DistanceMetric {
        self.metric
    }

    pub fn size(&self) -> usize {
        self.entries.read().len()
    }

    pub fn enable_stats(&mut self, enable: bool) {
        self.enable_stats = enable;
    }

    pub fn stats(&self) -> Stats {
        self.stats.read().clone()
    }

    pub fn reset_stats(&self) {
        *self.stats.write() = Stats::default();
    }

    pub fn insert(&self, id: i64, vector: Vec<f32>, metadata: Option<Vec<u8>>) -> Result<()> {
        if vector.len() != self.dimension {
            return Err(Error::DimensionMismatch {
                expected: self.dimension,
                got: vector.len(),
            });
        }

        let mut vector = vector;
        if self.normalized {
            crate::distance::normalize(&mut vector);
        }

        let mut entries = self.entries.write();
        let mut id_map = self.id_map.write();

        if id_map.contains_key(&id) {
            return Err(Error::DuplicateId(id));
        }

        let index = entries.len();
        entries.push(VectorEntry::new(id, vector, metadata));
        id_map.insert(id, index);

        if self.enable_stats {
            let mut stats = self.stats.write();
            stats.insert_count += 1;
        }

        Ok(())
    }

    pub fn delete(&self, id: i64) -> Result<()> {
        let mut entries = self.entries.write();
        let mut id_map = self.id_map.write();

        let index = id_map.remove(&id).ok_or(Error::NotFound(id))?;

        entries.remove(index);

        for (i, entry) in entries.iter().enumerate() {
            id_map.insert(entry.id, i);
        }

        if self.enable_stats {
            let mut stats = self.stats.write();
            stats.delete_count += 1;
        }

        Ok(())
    }

    pub fn get(&self, id: i64) -> Result<VectorEntry> {
        let entries = self.entries.read();
        let id_map = self.id_map.read();

        let index = id_map.get(&id).ok_or(Error::NotFound(id))?;

        if self.enable_stats {
            let mut stats = self.stats.write();
            stats.get_count += 1;
        }

        Ok(entries[*index].clone())
    }

    pub fn search(&self, query: &[f32], k: usize) -> Result<Vec<crate::vector_db::SearchResult>> {
        if query.len() != self.dimension {
            return Err(Error::DimensionMismatch {
                expected: self.dimension,
                got: query.len(),
            });
        }

        let entries = self.entries.read();

        let mut query_vec = query.to_vec();
        if self.normalized {
            crate::distance::normalize(&mut query_vec);
        }

        let start = if self.enable_stats {
            Some(std::time::Instant::now())
        } else {
            None
        };

        use rayon::prelude::*;
        let mut distances: Vec<(i64, f32)> = entries
            .par_iter()
            .map(|entry| {
                let distance = match self.metric {
                    DistanceMetric::Cosine => crate::distance::cosine_distance(&query_vec, &entry.vector),
                    DistanceMetric::Euclidean => crate::distance::euclidean_distance(&query_vec, &entry.vector),
                    DistanceMetric::DotProduct => -crate::distance::dot_product(&query_vec, &entry.vector),
                    DistanceMetric::Manhattan => {
                        let mut sum = 0.0f32;
                        for i in 0..query_vec.len() {
                            sum += (query_vec[i] - entry.vector[i]).abs();
                        }
                        sum
                    }
                };
                (entry.id, distance)
            })
            .collect();

        distances.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
        distances.truncate(k);

        let results: Vec<crate::vector_db::SearchResult> = distances
            .into_iter()
            .map(|(id, distance)| crate::vector_db::SearchResult { id, distance })
            .collect();

        if self.enable_stats {
            if let Some(start) = start {
                let elapsed = start.elapsed().as_secs_f64() * 1000.0;
                let mut stats = self.stats.write();
                stats.search_count += 1;
                stats.avg_search_time_ms = (stats.avg_search_time_ms * (stats.search_count - 1) as f64
                    + elapsed)
                    / stats.search_count as f64;
            }
        }

        Ok(results)
    }

    pub fn search_batch(
        &self,
        queries: &[Vec<f32>],
        k: usize,
    ) -> Result<Vec<Vec<crate::vector_db::SearchResult>>> {
        use rayon::prelude::*;
        let results: Result<Vec<Vec<crate::vector_db::SearchResult>>> = queries
            .par_iter()
            .map(|query| self.search(query, k))
            .collect();

        results
    }

    /// 优化版本：支持压缩的保存功能
    pub fn save(&self, filename: &str) -> Result<()> {
        self.save_with_compression(filename, true)
    }

    /// 支持压缩选项的保存功能
    pub fn save_with_compression(&self, filename: &str, use_compression: bool) -> Result<()> {
        let file = File::create(filename)?;
        let mut writer = BufWriter::with_capacity(1024 * 1024, file);

        let entries = self.entries.read();
        let id_map = self.id_map.read();

        let metadata = DatabaseMetadata {
            version: 1,
            dimension: self.dimension,
            metric: match self.metric {
                DistanceMetric::Cosine => "cosine".to_string(),
                DistanceMetric::Euclidean => "euclidean".to_string(),
                DistanceMetric::DotProduct => "dot_product".to_string(),
                DistanceMetric::Manhattan => "manhattan".to_string(),
            },
            size: entries.len(),
            compressed: use_compression,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };

        let mut data_buffer = Vec::with_capacity(entries.len() * (8 + 4 + self.dimension * 4 + 4));

        for entry in entries.iter() {
            data_buffer.write_i64::<LittleEndian>(entry.id)?;
            data_buffer.write_u32::<LittleEndian>(entry.vector.len() as u32)?;

            for value in &entry.vector {
                data_buffer.write_f32::<LittleEndian>(*value)?;
            }

            let metadata_len = entry.metadata.as_ref().map_or(0, |m| m.len()) as u32;
            data_buffer.write_u32::<LittleEndian>(metadata_len)?;

            if let Some(metadata) = &entry.metadata {
                data_buffer.write_all(metadata)?;
            }
        }

        let id_map_data = self.serialize_id_map(&id_map);

        let metadata_json = serde_json::to_string(&metadata)
            .map_err(|e| Error::SerializationError(e.to_string()))?;
        let metadata_bytes = metadata_json.as_bytes();
        let metadata_len = metadata_bytes.len() as u32;

        writer.write_u32::<LittleEndian>(metadata_len)?;
        writer.write_all(metadata_bytes)?;

        if use_compression {
            let compressed_data = compress_prepend_size(&data_buffer);
            writer.write_u32::<LittleEndian>(compressed_data.len() as u32)?;
            writer.write_all(&compressed_data)?;

            let compressed_id_map = compress_prepend_size(&id_map_data);
            writer.write_u32::<LittleEndian>(compressed_id_map.len() as u32)?;
            writer.write_all(&compressed_id_map)?;
        } else {
            writer.write_u32::<LittleEndian>(data_buffer.len() as u32)?;
            writer.write_all(&data_buffer)?;

            writer.write_u32::<LittleEndian>(id_map_data.len() as u32)?;
            writer.write_all(&id_map_data)?;
        }

        writer.flush()?;
        Ok(())
    }

    /// 优化版本：支持压缩的加载功能
    pub fn load(filename: &str) -> Result<Self> {
        let file = File::open(filename)?;
        let mut reader = BufReader::with_capacity(1024 * 1024, file);

        let mut metadata_len_buf = [0u8; 4];
        reader.read_exact(&mut metadata_len_buf)?;
        let metadata_len = u32::from_le_bytes(metadata_len_buf) as usize;

        let mut metadata_bytes = vec![0u8; metadata_len];
        reader.read_exact(&mut metadata_bytes)?;
        let metadata_str = String::from_utf8(metadata_bytes)
            .map_err(|e| Error::SerializationError(e.to_string()))?;
        let metadata: DatabaseMetadata = serde_json::from_str(&metadata_str)
            .map_err(|e| Error::SerializationError(e.to_string()))?;

        let dimension = metadata.dimension;
        let metric = match metadata.metric.as_str() {
            "cosine" => DistanceMetric::Cosine,
            "euclidean" => DistanceMetric::Euclidean,
            "dot_product" => DistanceMetric::DotProduct,
            "manhattan" => DistanceMetric::Manhattan,
            _ => return Err(Error::InvalidDistanceMetric),
        };

        let mut entries = Vec::with_capacity(metadata.size);
        let mut id_map = HashMap::with_capacity(metadata.size);

        let mut data_len_buf = [0u8; 4];
        reader.read_exact(&mut data_len_buf)?;
        let data_len = u32::from_le_bytes(data_len_buf) as usize;

        let mut data_buffer = vec![0u8; data_len];
        reader.read_exact(&mut data_buffer)?;

        if metadata.compressed {
            data_buffer = decompress_size_prepended(&data_buffer)
                .map_err(|e| Error::SerializationError(format!("LZ4 解压失败：{}", e)))?;
        }

        let mut cursor = std::io::Cursor::new(&data_buffer);
        for _ in 0..metadata.size {
            let id = cursor.read_i64::<LittleEndian>()?;
            let len = cursor.read_u32::<LittleEndian>()? as usize;

            let mut vector = Vec::with_capacity(len);
            for _ in 0..len {
                vector.push(cursor.read_f32::<LittleEndian>()?);
            }

            let metadata_len = cursor.read_u32::<LittleEndian>()? as usize;
            let metadata = if metadata_len > 0 {
                let mut metadata_bytes = vec![0u8; metadata_len];
                cursor.read_exact(&mut metadata_bytes)?;
                Some(metadata_bytes)
            } else {
                None
            };

            let index = entries.len();
            entries.push(VectorEntry::new(id, vector, metadata));
            id_map.insert(id, index);
        }

        let mut id_map_len_buf = [0u8; 4];
        reader.read_exact(&mut id_map_len_buf)?;
        let id_map_len = u32::from_le_bytes(id_map_len_buf) as usize;

        let mut id_map_buffer = vec![0u8; id_map_len];
        reader.read_exact(&mut id_map_buffer)?;

        if metadata.compressed {
            id_map_buffer = decompress_size_prepended(&id_map_buffer)
                .map_err(|e| Error::SerializationError(format!("LZ4 解压失败：{}", e)))?;
        }

        let _ = Self::deserialize_id_map(&id_map_buffer, &mut id_map)?;

        Ok(VectorDB {
            entries: Arc::new(RwLock::new(entries)),
            id_map: Arc::new(RwLock::new(id_map)),
            dimension,
            metric,
            normalized: metric == DistanceMetric::Cosine,
            stats: Arc::new(RwLock::new(Stats::default())),
            enable_stats: false,
        })
    }

    fn serialize_id_map(&self, id_map: &HashMap<i64, usize>) -> Vec<u8> {
        let mut buffer = Vec::with_capacity(id_map.len() * 12);
        for (id, index) in id_map {
            buffer.extend_from_slice(&id.to_le_bytes());
            buffer.extend_from_slice(&(*index as u32).to_le_bytes());
        }
        buffer
    }

    fn deserialize_id_map(data: &[u8], id_map: &mut HashMap<i64, usize>) -> Result<()> {
        let mut cursor = std::io::Cursor::new(data);
        while (cursor.position() as usize) < data.len() {
            let id = cursor.read_i64::<LittleEndian>()?;
            let index = cursor.read_u32::<LittleEndian>()? as usize;
            id_map.insert(id, index);
        }
        Ok(())
    }

    /// 增量保存：只保存变更的数据
    pub fn save_incremental(&self, filename: &str, modified_ids: &[i64]) -> Result<()> {
        if modified_ids.is_empty() {
            return self.save(filename);
        }

        let entries = self.entries.read();
        let id_map = self.id_map.read();

        let mut modified_entries = Vec::new();
        for &id in modified_ids {
            if let Some(&index) = id_map.get(&id) {
                modified_entries.push(&entries[index]);
            }
        }

        drop(entries);
        drop(id_map);

        let file = File::create(filename)?;
        let mut writer = BufWriter::with_capacity(1024 * 1024, file);

        let metadata = DatabaseMetadata {
            version: 1,
            dimension: self.dimension,
            metric: match self.metric {
                DistanceMetric::Cosine => "cosine".to_string(),
                DistanceMetric::Euclidean => "euclidean".to_string(),
                DistanceMetric::DotProduct => "dot_product".to_string(),
                DistanceMetric::Manhattan => "manhattan".to_string(),
            },
            size: modified_entries.len(),
            compressed: true,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };

        let mut data_buffer = Vec::new();
        for entry in modified_entries {
            data_buffer.write_i64::<LittleEndian>(entry.id)?;
            data_buffer.write_u32::<LittleEndian>(entry.vector.len() as u32)?;

            for value in &entry.vector {
                data_buffer.write_f32::<LittleEndian>(*value)?;
            }

            let metadata_len = entry.metadata.as_ref().map_or(0, |m| m.len()) as u32;
            data_buffer.write_u32::<LittleEndian>(metadata_len)?;

            if let Some(metadata) = &entry.metadata {
                data_buffer.write_all(metadata)?;
            }
        }

        let metadata_json = serde_json::to_string(&metadata)
            .map_err(|e| Error::SerializationError(e.to_string()))?;
        let metadata_bytes = metadata_json.as_bytes();
        let metadata_len = metadata_bytes.len() as u32;

        writer.write_u32::<LittleEndian>(metadata_len)?;
        writer.write_all(metadata_bytes)?;

        let compressed_data = compress_prepend_size(&data_buffer);
        writer.write_u32::<LittleEndian>(compressed_data.len() as u32)?;
        writer.write_all(&compressed_data)?;

        writer.write_u32::<LittleEndian>(0)?;

        writer.flush()?;
        Ok(())
    }

    pub fn print_stats(&self) {
        let stats = self.stats.read();
        println!("=== VectorDB Statistics ===");
        println!("Insert count: {}", stats.insert_count);
        println!("Delete count: {}", stats.delete_count);
        println!("Search count: {}", stats.search_count);
        println!("Get count: {}", stats.get_count);
        println!("Cache hits: {}", stats.cache_hits);
        println!("Cache misses: {}", stats.cache_misses);
        println!("Avg search time: {:.3} ms", stats.avg_search_time_ms);
        println!("Avg insert time: {:.3} μs", stats.avg_insert_time_us);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_save_load_with_compression() {
        let db = VectorDB::new(128, DistanceMetric::Cosine);

        for i in 0..100 {
            let vector: Vec<f32> = (0..128).map(|j| ((i * 128 + j) as f32) / 1000.0).collect();
            db.insert(i as i64, vector, None).unwrap();
        }

        db.save_with_compression("/tmp/test_compressed.bin", true).unwrap();
        let loaded_db = VectorDB::load("/tmp/test_compressed.bin").unwrap();

        assert_eq!(loaded_db.size(), 100);
        assert_eq!(loaded_db.dimension(), 128);

        std::fs::remove_file("/tmp/test_compressed.bin").ok();
    }

    #[test]
    fn test_save_load_without_compression() {
        let db = VectorDB::new(64, DistanceMetric::Euclidean);

        for i in 0..50 {
            let vector: Vec<f32> = (0..64).map(|j| (i + j) as f32).collect();
            db.insert(i as i64, vector, None).unwrap();
        }

        db.save_with_compression("/tmp/test_uncompressed.bin", false).unwrap();
        let loaded_db = VectorDB::load("/tmp/test_uncompressed.bin").unwrap();

        assert_eq!(loaded_db.size(), 50);
        assert_eq!(loaded_db.dimension(), 64);

        std::fs::remove_file("/tmp/test_uncompressed.bin").ok();
    }
}
