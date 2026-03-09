use std::collections::HashMap;
use parking_lot::RwLock;
use rayon::prelude::*;
use std::cmp::Ordering;

use crate::distance::{DistanceMetric, cosine_distance, euclidean_distance, dot_product_distance};
use crate::error::{Error, Result};

pub type Vector = Vec<f32>;

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone, Default)]
pub struct Stats {
    pub insert_count: u64,
    pub delete_count: u64,
    pub search_count: u64,
    pub get_count: u64,
    pub avg_insert_us: f64,
    pub avg_search_ms: f64,
}

pub struct VectorDB {
    entries: RwLock<Vec<VectorEntry>>,
    id_index: RwLock<HashMap<u64, usize>>,
    dimension: usize,
    metric: DistanceMetric,
    stats: RwLock<Stats>,
    enable_stats: bool,
}

impl VectorDB {
    pub fn new(dimension: usize, metric: DistanceMetric) -> Self {
        Self {
            entries: RwLock::new(Vec::with_capacity(4096)),
            id_index: RwLock::new(HashMap::with_capacity(4096)),
            dimension,
            metric,
            stats: RwLock::new(Stats::default()),
            enable_stats: true,
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
        
        let entries = self.entries.read();
        let distance_fn = self.distance_fn();
        let n = entries.len();
        let k = k.min(n);
        
        if k == 0 {
            return Ok(Vec::new());
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
        
        if self.enable_stats {
            let mut stats = self.stats.write();
            let elapsed_ms = start.elapsed().as_nanos() as f64 / 1_000_000.0;
            stats.search_count += 1;
            stats.avg_search_ms = (stats.avg_search_ms * (stats.search_count - 1) as f64 + elapsed_ms) 
                / stats.search_count as f64;
        }
        
        Ok(results)
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
        println!("\n=== Rust GLM5 VectorDB Statistics ===");
        println!("Version: {}", env!("CARGO_PKG_VERSION"));
        println!("Dimension: {}", self.dimension);
        println!("Size: {} vectors", self.len());
        println!("Metric: {}", self.metric);
        println!("\nOperations:");
        println!("  Insert:  {} ({:.1} µs avg)", stats.insert_count, stats.avg_insert_us);
        println!("  Delete:  {}", stats.delete_count);
        println!("  Search:  {} ({:.3} ms avg)", stats.search_count, stats.avg_search_ms);
        println!("  Get:     {}", stats.get_count);
        println!("====================================\n");
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
}
