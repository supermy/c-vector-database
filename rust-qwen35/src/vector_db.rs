use crate::distance::{cosine_distance, dot_product, euclidean_distance, manhattan_distance, normalize, DistanceMetric};
use crate::error::{Error, Result};
use parking_lot::RwLock;
use rayon::prelude::*;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct VectorEntry {
    pub id: i64,
    pub vector: Vec<f32>,
    pub metadata: Option<Vec<u8>>,
}

impl VectorEntry {
    pub fn new(id: i64, vector: Vec<f32>, metadata: Option<Vec<u8>>) -> Self {
        VectorEntry {
            id,
            vector,
            metadata,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub id: i64,
    pub distance: f32,
}

#[derive(Debug, Default, Clone)]
pub struct Stats {
    pub insert_count: usize,
    pub delete_count: usize,
    pub search_count: usize,
    pub get_count: usize,
    pub cache_hits: usize,
    pub cache_misses: usize,
    pub avg_search_time_ms: f64,
    pub avg_insert_time_us: f64,
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
            normalize(&mut vector);
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

    pub fn search(&self, query: &[f32], k: usize) -> Result<Vec<SearchResult>> {
        if query.len() != self.dimension {
            return Err(Error::DimensionMismatch {
                expected: self.dimension,
                got: query.len(),
            });
        }

        let entries = self.entries.read();
        
        let mut query_vec = query.to_vec();
        if self.normalized {
            normalize(&mut query_vec);
        }

        let start = if self.enable_stats {
            Some(std::time::Instant::now())
        } else {
            None
        };

        let mut distances: Vec<(i64, f32)> = entries
            .par_iter()
            .map(|entry| {
                let distance = match self.metric {
                    DistanceMetric::Cosine => cosine_distance(&query_vec, &entry.vector),
                    DistanceMetric::Euclidean => euclidean_distance(&query_vec, &entry.vector),
                    DistanceMetric::DotProduct => -dot_product(&query_vec, &entry.vector),
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

        let results: Vec<SearchResult> = distances
            .into_iter()
            .map(|(id, distance)| SearchResult { id, distance })
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
    ) -> Result<Vec<Vec<SearchResult>>> {
        let results: Result<Vec<Vec<SearchResult>>> = queries
            .par_iter()
            .map(|query| self.search(query, k))
            .collect();

        results
    }

    pub fn save(&self, filename: &str) -> Result<()> {
        let file = File::create(filename)?;
        let mut writer = BufWriter::new(file);

        let entries = self.entries.read();
        let id_map = self.id_map.read();

        let metadata = serde_json::json!({
            "dimension": self.dimension,
            "metric": match self.metric {
                DistanceMetric::Cosine => "cosine",
                DistanceMetric::Euclidean => "euclidean",
                DistanceMetric::DotProduct => "dot_product",
                DistanceMetric::Manhattan => "manhattan",
            },
            "size": entries.len(),
        });

        let metadata_str = serde_json::to_string(&metadata)
            .map_err(|e| Error::SerializationError(e.to_string()))?;
        let metadata_bytes = metadata_str.as_bytes();
        let metadata_len = metadata_bytes.len() as u32;

        writer.write_all(&metadata_len.to_le_bytes())?;
        writer.write_all(metadata_bytes)?;

        for entry in entries.iter() {
            writer.write_all(&entry.id.to_le_bytes())?;
            writer.write_all(&(entry.vector.len() as u32).to_le_bytes())?;
            
            for value in entry.vector.iter() {
                writer.write_all(&value.to_le_bytes())?;
            }

            let metadata_len = entry.metadata.as_ref().map_or(0, |m| m.len()) as u32;
            writer.write_all(&metadata_len.to_le_bytes())?;
            
            if let Some(metadata) = &entry.metadata {
                writer.write_all(metadata)?;
            }
        }

        writer.write_all(&(id_map.len() as u32).to_le_bytes())?;
        for (id, index) in id_map.iter() {
            writer.write_all(&id.to_le_bytes())?;
            writer.write_all(&(*index as u32).to_le_bytes())?;
        }

        writer.flush()?;
        Ok(())
    }

    pub fn load(filename: &str) -> Result<Self> {
        let file = File::open(filename)?;
        let mut reader = BufReader::new(file);

        let mut metadata_len_buf = [0u8; 4];
        reader.read_exact(&mut metadata_len_buf)?;
        let metadata_len = u32::from_le_bytes(metadata_len_buf) as usize;

        let mut metadata_bytes = vec![0u8; metadata_len];
        reader.read_exact(&mut metadata_bytes)?;
        let metadata_str = String::from_utf8(metadata_bytes)
            .map_err(|e| Error::SerializationError(e.to_string()))?;
        let metadata: serde_json::Value = serde_json::from_str(&metadata_str)
            .map_err(|e| Error::SerializationError(e.to_string()))?;

        let dimension = metadata["dimension"]
            .as_u64()
            .ok_or_else(|| Error::SerializationError("Invalid dimension".to_string()))? as usize;

        let metric_str = metadata["metric"]
            .as_str()
            .ok_or_else(|| Error::SerializationError("Invalid metric".to_string()))?;

        let metric = match metric_str {
            "cosine" => DistanceMetric::Cosine,
            "euclidean" => DistanceMetric::Euclidean,
            "dot_product" => DistanceMetric::DotProduct,
            "manhattan" => DistanceMetric::Manhattan,
            _ => return Err(Error::InvalidDistanceMetric),
        };

        let mut entries = Vec::new();
        let mut id_map = HashMap::new();

        let num_entries = metadata["size"]
            .as_u64()
            .ok_or_else(|| Error::SerializationError("Invalid size".to_string()))? as usize;

        for _ in 0..num_entries {
            let mut id_buf = [0u8; 8];
            reader.read_exact(&mut id_buf)?;
            let id = i64::from_le_bytes(id_buf);

            let mut len_buf = [0u8; 4];
            reader.read_exact(&mut len_buf)?;
            let len = u32::from_le_bytes(len_buf) as usize;

            let mut vector = Vec::with_capacity(len);
            for _ in 0..len {
                let mut value_buf = [0u8; 4];
                reader.read_exact(&mut value_buf)?;
                vector.push(f32::from_le_bytes(value_buf));
            }

            reader.read_exact(&mut len_buf)?;
            let metadata_len = u32::from_le_bytes(len_buf) as usize;

            let metadata = if metadata_len > 0 {
                let mut metadata_bytes = vec![0u8; metadata_len];
                reader.read_exact(&mut metadata_bytes)?;
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
        let _id_map_len = u32::from_le_bytes(id_map_len_buf);

        for _ in 0.._id_map_len {
            let mut id_buf = [0u8; 8];
            reader.read_exact(&mut id_buf)?;
            let _id = i64::from_le_bytes(id_buf);

            let mut index_buf = [0u8; 4];
            reader.read_exact(&mut index_buf)?;
            let _index = u32::from_le_bytes(index_buf);
        }

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
    fn test_insert_and_search() {
        let db = VectorDB::new(3, DistanceMetric::Cosine);
        
        db.insert(1, vec![1.0, 0.0, 0.0], None).unwrap();
        db.insert(2, vec![0.0, 1.0, 0.0], None).unwrap();
        db.insert(3, vec![0.0, 0.0, 1.0], None).unwrap();

        let results = db.search(&[1.0, 0.0, 0.0], 1).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, 1);
    }

    #[test]
    fn test_delete() {
        let db = VectorDB::new(3, DistanceMetric::Cosine);
        
        db.insert(1, vec![1.0, 0.0, 0.0], None).unwrap();
        db.delete(1).unwrap();
        
        assert!(db.get(1).is_err());
    }

    #[test]
    fn test_duplicate_id() {
        let db = VectorDB::new(3, DistanceMetric::Cosine);
        
        db.insert(1, vec![1.0, 0.0, 0.0], None).unwrap();
        assert!(db.insert(1, vec![0.0, 1.0, 0.0], None).is_err());
    }
}
