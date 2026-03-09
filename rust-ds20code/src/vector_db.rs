use std::sync::Arc;
use parking_lot::RwLock;
use rayon::prelude::*;
use std::cmp::Ordering;
use std::fs::File;
use std::path::Path;
use std::io::{Read, Write};

use crate::distance::{DistanceMetric, cosine_distance, euclidean_distance, dot_product_distance};
use crate::hnsw::HnswIndex;
use crate::error::{Error, Result};

pub type Vector = Vec<f32>;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VectorEntry {
    pub id: u64,
    pub vector: Vector,
    pub metadata: Option<Vec<u8>>,
}

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub id: u64,
    pub distance: f32,
    pub metadata: Option<Vec<u8>>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct Stats {
    pub insert_count: u64,
    pub delete_count: u64,
    pub search_count: u64,
    pub get_count: u64,
    pub avg_insert_us: f64,
    pub avg_search_ms: f64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct SerializableDatabase {
    dimension: usize,
    metric: DistanceMetric,
    index_type: IndexType,
    entries: Vec<VectorEntry>,
    stats: Stats,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum IndexType {
    Flat,
    Hnsw,
}

pub struct VectorDB {
    entries: RwLock<Vec<VectorEntry>>,
    id_index: RwLock<ahash::AHashMap<u64, usize>>,
    dimension: usize,
    metric: DistanceMetric,
    stats: RwLock<Stats>,
    enable_stats: bool,
    index_type: IndexType,
    hnsw_index: Option<Arc<HnswIndex>>,
}

impl VectorDB {
    pub fn new(dimension: usize, metric: DistanceMetric) -> Self {
        Self::with_index_type(dimension, metric, IndexType::Flat)
    }
    
    pub fn with_hnsw(dimension: usize, metric: DistanceMetric) -> Self {
        Self::with_index_type(dimension, metric, IndexType::Hnsw)
    }
    
    pub fn with_index_type(dimension: usize, metric: DistanceMetric, index_type: IndexType) -> Self {
        let hnsw_index = if index_type == IndexType::Hnsw {
            Some(Arc::new(HnswIndex::new(dimension, metric)))
        } else {
            None
        };
        
        Self {
            entries: RwLock::new(Vec::with_capacity(4096)),
            id_index: RwLock::new(ahash::AHashMap::with_capacity(4096)),
            dimension,
            metric,
            stats: RwLock::new(Stats::default()),
            enable_stats: true,
            index_type,
            hnsw_index,
        }
    }
    
    pub fn insert(&self, id: u64, vector: &[f32], metadata: Option<Vec<u8>>) -> Result<()> {
        if vector.len() != self.dimension {
            return Err(Error::InvalidDimension {
                expected: self.dimension,
                got: vector.len(),
            });
        }
        
        let start = std::time::Instant::now();
        
        {
            let id_index = self.id_index.read();
            if id_index.contains_key(&id) {
                return Err(Error::DuplicateId(id));
            }
        }
        
        if let Some(ref hnsw) = self.hnsw_index {
            hnsw.insert(id, vector.to_vec());
        }
        
        let mut entries = self.entries.write();
        let mut id_index = self.id_index.write();
        
        let entry = VectorEntry {
            id,
            vector: vector.to_vec(),
            metadata,
        };
        
        let idx = entries.len();
        entries.push(entry);
        id_index.insert(id, idx);
        
        if self.enable_stats {
            let mut stats = self.stats.write();
            let elapsed_us = start.elapsed().as_nanos() as f64 / 1000.0;
            stats.insert_count += 1;
            stats.avg_insert_us = (stats.avg_insert_us * (stats.insert_count - 1) as f64 + elapsed_us) 
                / stats.insert_count as f64;
        }
        
        Ok(())
    }
    
    pub fn delete(&self, id: u64) -> Result<()> {
        let mut entries = self.entries.write();
        let mut id_index = self.id_index.write();
        
        let idx = id_index.remove(&id).ok_or(Error::NotFound(id))?;
        
        if idx < entries.len() - 1 {
            entries.swap_remove(idx);
            if let Some(_moved_idx) = id_index.get(&entries[idx].id) {
                id_index.insert(entries[idx].id, idx);
            }
        } else {
            entries.pop();
        }
        
        if self.enable_stats {
            self.stats.write().delete_count += 1;
        }
        
        Ok(())
    }
    
    pub fn get(&self, id: u64) -> Option<VectorEntry> {
        let id_index = self.id_index.read();
        let idx = id_index.get(&id)?;
        let entries = self.entries.read();
        
        if self.enable_stats {
            self.stats.write().get_count += 1;
        }
        
        Some(entries[*idx].clone())
    }
    
    fn distance_fn(&self) -> fn(&[f32], &[f32]) -> f32 {
        match self.metric {
            DistanceMetric::Cosine => cosine_distance,
            DistanceMetric::Euclidean => euclidean_distance,
            DistanceMetric::DotProduct => dot_product_distance,
        }
    }
    
    pub fn search(&self, query: &[f32], k: usize) -> Result<Vec<SearchResult>> {
        if query.len() != self.dimension {
            return Err(Error::InvalidDimension {
                expected: self.dimension,
                got: query.len(),
            });
        }
        
        let start = std::time::Instant::now();
        
        let results = if let Some(ref hnsw) = self.hnsw_index {
            self.search_hnsw(hnsw, query, k)
        } else {
            self.search_flat(query, k)
        };
        
        if self.enable_stats {
            let mut stats = self.stats.write();
            let elapsed_ms = start.elapsed().as_nanos() as f64 / 1_000_000.0;
            stats.search_count += 1;
            stats.avg_search_ms = (stats.avg_search_ms * (stats.search_count - 1) as f64 + elapsed_ms) 
                / stats.search_count as f64;
        }
        
        Ok(results)
    }
    
    fn search_flat(&self, query: &[f32], k: usize) -> Vec<SearchResult> {
        let entries = self.entries.read();
        let distance_fn = self.distance_fn();
        let n = entries.len();
        let k = k.min(n);
        
        if k == 0 {
            return Vec::new();
        }
        
        let mut results: Vec<SearchResult> = entries
            .par_iter()
            .map(|entry| {
                let distance = distance_fn(query, &entry.vector);
                SearchResult {
                    id: entry.id,
                    distance,
                    metadata: entry.metadata.clone(),
                }
            })
            .collect();
        
        results.select_nth_unstable_by(k - 1, |a, b| {
            b.distance.partial_cmp(&a.distance).unwrap_or(Ordering::Equal)
        });
        results.truncate(k);
        
        results
    }
    
    fn search_hnsw(&self, hnsw: &HnswIndex, query: &[f32], k: usize) -> Vec<SearchResult> {
        let hnsw_results = hnsw.search(query, k);
        let entries = self.entries.read();
        let id_index = self.id_index.read();
        
        hnsw_results
            .into_iter()
            .filter_map(|(id, distance)| {
                let idx = id_index.get(&id)?;
                let entry = &entries[*idx];
                Some(SearchResult {
                    id,
                    distance,
                    metadata: entry.metadata.clone(),
                })
            })
            .collect()
    }
    
    pub fn search_with_threshold(&self, query: &[f32], k: usize, threshold: f32) -> Result<Vec<SearchResult>> {
        let mut results = self.search(query, k)?;
        results.retain(|r| r.distance >= threshold);
        Ok(results)
    }
    
    pub fn batch_search(&self, queries: &[&[f32]], k: usize) -> Result<Vec<Vec<SearchResult>>> {
        queries.iter().map(|q| self.search(q, k)).collect()
    }
    
    pub fn len(&self) -> usize {
        self.entries.read().len()
    }
    
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    
    pub fn dimension(&self) -> usize {
        self.dimension
    }
    
    pub fn metric(&self) -> DistanceMetric {
        self.metric
    }
    
    pub fn index_type(&self) -> IndexType {
        self.index_type
    }
    
    pub fn stats(&self) -> Stats {
        self.stats.read().clone()
    }
    
    pub fn enable_stats(&mut self, enable: bool) {
        self.enable_stats = enable;
    }
    
    pub fn reset_stats(&self) {
        *self.stats.write() = Stats::default();
    }
    
    pub fn print_stats(&self) {
        let stats = self.stats();
        println!("\n=== Rust DS20Code VectorDB Statistics ===");
        println!("Version: {}", env!("CARGO_PKG_VERSION"));
        println!("Dimension: {}", self.dimension);
        println!("Size: {} vectors", self.len());
        println!("Metric: {}", self.metric);
        println!("Index Type: {:?}", self.index_type);
        println!("\nOperations:");
        println!("  Insert:  {} ({:.1} µs avg)", stats.insert_count, stats.avg_insert_us);
        println!("  Delete:  {}", stats.delete_count);
        println!("  Search:  {} ({:.3} ms avg)", stats.search_count, stats.avg_search_ms);
        println!("  Get:     {}", stats.get_count);
        println!("========================================\n");
    }

    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let serializable = SerializableDatabase {
            dimension: self.dimension,
            metric: self.metric,
            index_type: self.index_type,
            entries: self.entries.read().clone(),
            stats: self.stats.read().clone(),
        };

        let file = File::create(path)
            .map_err(|e| Error::IoError(e.to_string()))?;

        let writer = std::io::BufWriter::new(file);
        bincode::serialize_into(writer, &serializable)
            .map_err(|e| Error::IoError(e.to_string()))?;

        Ok(())
    }

    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = File::open(path)
            .map_err(|e| Error::IoError(e.to_string()))?;

        let reader = std::io::BufReader::new(file);
        let serializable: SerializableDatabase = bincode::deserialize_from(reader)
            .map_err(|e| Error::IoError(e.to_string()))?;

        let hnsw_index = if serializable.index_type == IndexType::Hnsw {
            let hnsw = Arc::new(HnswIndex::new(
                serializable.dimension,
                serializable.metric,
            ));

            for entry in &serializable.entries {
                hnsw.insert(entry.id, entry.vector.clone());
            }

            Some(hnsw)
        } else {
            None
        };

        let id_index = ahash::AHashMap::from_iter(
            serializable.entries
                .iter()
                .enumerate()
                .map(|(idx, entry)| (entry.id, idx))
        );

        Ok(Self {
            entries: RwLock::new(serializable.entries),
            id_index: RwLock::new(id_index),
            dimension: serializable.dimension,
            metric: serializable.metric,
            stats: RwLock::new(serializable.stats),
            enable_stats: true,
            index_type: serializable.index_type,
            hnsw_index,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_insert_and_get() {
        let db = VectorDB::new(3, DistanceMetric::Cosine);
        let vector = vec![1.0, 2.0, 3.0];
        
        db.insert(1, &vector, None).unwrap();
        assert_eq!(db.len(), 1);
        
        let entry = db.get(1).unwrap();
        assert_eq!(entry.id, 1);
    }
    
    #[test]
    fn test_search() {
        let db = VectorDB::new(3, DistanceMetric::Cosine);
        
        db.insert(1, &[1.0, 0.0, 0.0], None).unwrap();
        db.insert(2, &[0.0, 1.0, 0.0], None).unwrap();
        db.insert(3, &[0.0, 0.0, 1.0], None).unwrap();
        
        let query = vec![1.0, 0.0, 0.0];
        let results = db.search(&query, 2).unwrap();
        
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].id, 1);
        assert!((results[0].distance - 1.0).abs() < 1e-6);
    }
    
    #[test]
    fn test_delete() {
        let db = VectorDB::new(3, DistanceMetric::Cosine);
        let vector = vec![1.0, 2.0, 3.0];
        
        db.insert(1, &vector, None).unwrap();
        assert_eq!(db.len(), 1);
        
        db.delete(1).unwrap();
        assert_eq!(db.len(), 0);
        assert!(db.get(1).is_none());
    }
    
    #[test]
    fn test_batch_search() {
        let db = VectorDB::new(3, DistanceMetric::Cosine);
        
        db.insert(1, &[1.0, 0.0, 0.0], None).unwrap();
        db.insert(2, &[0.0, 1.0, 0.0], None).unwrap();
        db.insert(3, &[0.0, 0.0, 1.0], None).unwrap();
        
        let queries: Vec<&[f32]> = vec![
            &[1.0, 0.0, 0.0],
            &[0.0, 1.0, 0.0],
        ];
        
        let results = db.batch_search(&queries, 2).unwrap();
        assert_eq!(results.len(), 2);
        assert!(results[0].len() <= 2);
        assert!(results[1].len() <= 2);
        assert_eq!(results[0][0].id, 1);
        assert_eq!(results[1][0].id, 2);
    }
    
    #[test]
    fn test_hnsw_search() {
        let db = VectorDB::with_hnsw(3, DistanceMetric::Euclidean);
        
        db.insert(1, &[1.0, 0.0, 0.0], None).unwrap();
        db.insert(2, &[0.0, 1.0, 0.0], None).unwrap();
        db.insert(3, &[0.0, 0.0, 1.0], None).unwrap();
        
        let query = vec![1.0, 0.0, 0.0];
        let results = db.search(&query, 2).unwrap();
        
        assert!(!results.is_empty());
    }
    
    #[test]
    fn test_persistence_flat() {
        let temp_path = std::env::temp_dir().join("test_db_flat.bin");
        
        {
            let db = VectorDB::new(3, DistanceMetric::Cosine);
            db.insert(1, &[1.0, 0.0, 0.0], None).unwrap();
            db.insert(2, &[0.0, 1.0, 0.0], None).unwrap();
            db.insert(3, &[0.0, 0.0, 1.0], Some(b"metadata".to_vec())).unwrap();
            db.save(&temp_path).unwrap();
        }
        
        {
            let db = VectorDB::load(&temp_path).unwrap();
            assert_eq!(db.len(), 3);
            
            let entry = db.get(3).unwrap();
            assert_eq!(entry.id, 3);
            assert_eq!(entry.metadata, Some(b"metadata".to_vec()));
            
            let query = vec![1.0, 0.0, 0.0];
            let results = db.search(&query, 1).unwrap();
            assert_eq!(results.len(), 1);
            assert_eq!(results[0].id, 1);
        }
        
        std::fs::remove_file(&temp_path).unwrap();
    }
    
    #[test]
    fn test_persistence_hnsw() {
        let temp_path = std::env::temp_dir().join("test_db_hnsw.bin");
        
        {
            let db = VectorDB::with_hnsw(3, DistanceMetric::Cosine);
            db.insert(1, &[1.0, 0.0, 0.0], None).unwrap();
            db.insert(2, &[0.0, 1.0, 0.0], None).unwrap();
            db.insert(3, &[0.0, 0.0, 1.0], None).unwrap();
            db.save(&temp_path).unwrap();
        }
        
        {
            let db = VectorDB::load(&temp_path).unwrap();
            assert_eq!(db.len(), 3);
            assert_eq!(db.index_type(), IndexType::Hnsw);
            
            let query = vec![1.0, 0.0, 0.0];
            let results = db.search(&query, 1).unwrap();
            assert!(!results.is_empty());
        }
        
        std::fs::remove_file(&temp_path).unwrap();
    }
}
