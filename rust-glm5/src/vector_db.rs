use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write, Cursor};
use std::path::Path;
use parking_lot::RwLock;
use rayon::prelude::*;
use std::cmp::Ordering;
use serde::{Deserialize, Serialize};

use crate::distance::{DistanceMetric, cosine_distance, euclidean_distance, dot_product_distance};
use crate::error::{Error, Result};

pub type Vector = Vec<f32>;

const FILE_MAGIC: &[u8; 4] = b"VDB2";
const FILE_VERSION: u32 = 2;
const HEADER_SIZE: usize = 64;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorEntry {
    pub id: u64,
    pub vector: Vector,
    pub metadata: Option<Vec<u8>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub id: u64,
    pub distance: f32,
    pub metadata: Option<Vec<u8>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Stats {
    pub insert_count: u64,
    pub delete_count: u64,
    pub search_count: u64,
    pub get_count: u64,
    pub avg_insert_us: f64,
    pub avg_search_ms: f64,
}

#[derive(Serialize, Deserialize)]
struct DBHeader {
    magic: [u8; 4],
    version: u32,
    dimension: usize,
    metric: DistanceMetric,
    entry_count: u64,
    flags: u32,
}

pub struct VectorDB {
    entries: RwLock<Vec<VectorEntry>>,
    id_index: RwLock<HashMap<u64, usize>>,
    dimension: usize,
    metric: DistanceMetric,
    stats: RwLock<Stats>,
    enable_stats: bool,
    dirty: RwLock<bool>,
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
            dirty: RwLock::new(false),
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
        
        *self.dirty.write() = true;
        
        if self.enable_stats {
            let mut stats = self.stats.write();
            let elapsed_us = start.elapsed().as_nanos() as f64 / 1000.0;
            stats.insert_count += 1;
            stats.avg_insert_us = (stats.avg_insert_us * (stats.insert_count - 1) as f64 + elapsed_us) 
                / stats.insert_count as f64;
        }
        
        Ok(())
    }
    
    pub fn insert_batch(&self, entries: Vec<(u64, Vec<f32>, Option<Vec<u8>>)>) -> Result<usize> {
        let start = std::time::Instant::now();
        let mut success_count = 0;
        
        let mut db_entries = self.entries.write();
        let mut id_index = self.id_index.write();
        
        for (id, vector, metadata) in entries {
            if vector.len() != self.dimension {
                continue;
            }
            
            if id_index.contains_key(&id) {
                continue;
            }
            
            let entry = VectorEntry { id, vector, metadata };
            let idx = db_entries.len();
            db_entries.push(entry);
            id_index.insert(id, idx);
            success_count += 1;
        }
        
        if success_count > 0 {
            *self.dirty.write() = true;
        }
        
        if self.enable_stats {
            let mut stats = self.stats.write();
            let elapsed_us = start.elapsed().as_nanos() as f64 / 1000.0;
            stats.insert_count += success_count as u64;
            if stats.insert_count > 0 {
                stats.avg_insert_us = (stats.avg_insert_us * (stats.insert_count - success_count as u64) as f64 
                    + elapsed_us) / stats.insert_count as f64;
            }
        }
        
        Ok(success_count)
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
        
        *self.dirty.write() = true;
        
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
    
    pub fn is_dirty(&self) -> bool {
        *self.dirty.read()
    }
    
    pub fn print_stats(&self) {
        let stats = self.stats();
        println!("\n=== Rust GLM5 VectorDB Statistics ===");
        println!("Version: {}", env!("CARGO_PKG_VERSION"));
        println!("Dimension: {}", self.dimension);
        println!("Size: {} vectors", self.len());
        println!("Metric: {}", self.metric);
        println!("Dirty: {}", self.is_dirty());
        println!("\nOperations:");
        println!("  Insert:  {} ({:.1} µs avg)", stats.insert_count, stats.avg_insert_us);
        println!("  Delete:  {}", stats.delete_count);
        println!("  Search:  {} ({:.3} ms avg)", stats.search_count, stats.avg_search_ms);
        println!("  Get:     {}", stats.get_count);
        println!("====================================\n");
    }
    
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let file = File::create(path.as_ref())?;
        let mut writer = BufWriter::with_capacity(64 * 1024, file);
        
        let entries = self.entries.read();
        let stats = self.stats.read();
        
        let header = DBHeader {
            magic: *FILE_MAGIC,
            version: FILE_VERSION,
            dimension: self.dimension,
            metric: self.metric,
            entry_count: entries.len() as u64,
            flags: 0,
        };
        
        let header_bytes = bincode::serialize(&header)?;
        writer.write_all(&header_bytes)?;
        
        let mut entry_count_bytes = vec![0u8; 8];
        for entry in entries.iter() {
            let entry_bytes = bincode::serialize(entry)?;
            let len = entry_bytes.len() as u64;
            entry_count_bytes.copy_from_slice(&len.to_le_bytes());
            writer.write_all(&entry_count_bytes)?;
            writer.write_all(&entry_bytes)?;
        }
        
        let stats_bytes = bincode::serialize(&*stats)?;
        writer.write_all(&stats_bytes)?;
        
        writer.flush()?;
        
        *self.dirty.write() = false;
        
        Ok(())
    }
    
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = File::open(path.as_ref())?;
        let mut reader = BufReader::with_capacity(64 * 1024, file);
        
        let header: DBHeader = bincode::deserialize_from(&mut reader)?;
        
        if &header.magic != FILE_MAGIC {
            return Err(Error::InvalidFileFormat("Invalid magic number".to_string()));
        }
        
        if header.version != FILE_VERSION {
            return Err(Error::VersionMismatch {
                expected: FILE_VERSION,
                got: header.version,
            });
        }
        
        let entry_count = header.entry_count as usize;
        let mut entries = Vec::with_capacity(entry_count);
        let mut id_index = HashMap::with_capacity(entry_count);
        
        let mut len_bytes = [0u8; 8];
        for idx in 0..entry_count {
            reader.read_exact(&mut len_bytes)?;
            let len = u64::from_le_bytes(len_bytes) as usize;
            
            let mut entry_bytes = vec![0u8; len];
            reader.read_exact(&mut entry_bytes)?;
            
            let entry: VectorEntry = bincode::deserialize(&entry_bytes)?;
            id_index.insert(entry.id, idx);
            entries.push(entry);
        }
        
        let stats: Stats = bincode::deserialize_from(&mut reader)?;
        
        Ok(Self {
            entries: RwLock::new(entries),
            id_index: RwLock::new(id_index),
            dimension: header.dimension,
            metric: header.metric,
            stats: RwLock::new(stats),
            enable_stats: true,
            dirty: RwLock::new(false),
        })
    }
    
    pub fn save_to_bytes(&self) -> Result<Vec<u8>> {
        let entries = self.entries.read();
        let stats = self.stats.read();
        
        let header = DBHeader {
            magic: *FILE_MAGIC,
            version: FILE_VERSION,
            dimension: self.dimension,
            metric: self.metric,
            entry_count: entries.len() as u64,
            flags: 0,
        };
        
        let header_bytes = bincode::serialize(&header)?;
        
        let mut data = Vec::with_capacity(header_bytes.len() + entries.len() * 1024);
        data.extend_from_slice(&header_bytes);
        
        for entry in entries.iter() {
            let entry_bytes = bincode::serialize(entry)?;
            let len = entry_bytes.len() as u64;
            data.extend_from_slice(&len.to_le_bytes());
            data.extend_from_slice(&entry_bytes);
        }
        
        let stats_bytes = bincode::serialize(&*stats)?;
        data.extend_from_slice(&stats_bytes);
        
        Ok(data)
    }
    
    pub fn load_from_bytes(bytes: &[u8]) -> Result<Self> {
        let mut cursor = Cursor::new(bytes);
        
        let header: DBHeader = bincode::deserialize_from(&mut cursor)?;
        
        if &header.magic != FILE_MAGIC {
            return Err(Error::InvalidFileFormat("Invalid magic number".to_string()));
        }
        
        if header.version != FILE_VERSION {
            return Err(Error::VersionMismatch {
                expected: FILE_VERSION,
                got: header.version,
            });
        }
        
        let entry_count = header.entry_count as usize;
        let mut entries = Vec::with_capacity(entry_count);
        let mut id_index = HashMap::with_capacity(entry_count);
        
        let mut len_bytes = [0u8; 8];
        for idx in 0..entry_count {
            cursor.read_exact(&mut len_bytes)?;
            let len = u64::from_le_bytes(len_bytes) as usize;
            
            let start = cursor.position() as usize;
            let end = start + len;
            
            if end > bytes.len() {
                return Err(Error::CorruptedData("Entry data exceeds buffer".to_string()));
            }
            
            let entry: VectorEntry = bincode::deserialize(&bytes[start..end])?;
            cursor.set_position(end as u64);
            
            id_index.insert(entry.id, idx);
            entries.push(entry);
        }
        
        let stats: Stats = bincode::deserialize_from(&mut cursor)?;
        
        Ok(Self {
            entries: RwLock::new(entries),
            id_index: RwLock::new(id_index),
            dimension: header.dimension,
            metric: header.metric,
            stats: RwLock::new(stats),
            enable_stats: true,
            dirty: RwLock::new(false),
        })
    }
    
    pub fn clear(&self) {
        let mut entries = self.entries.write();
        let mut id_index = self.id_index.write();
        
        entries.clear();
        id_index.clear();
        
        *self.dirty.write() = true;
        
        if self.enable_stats {
            self.stats.write().delete_count += entries.len() as u64;
        }
    }
    
    pub fn compact(&self) -> usize {
        let mut entries = self.entries.write();
        let mut id_index = self.id_index.write();
        
        entries.shrink_to_fit();
        id_index.shrink_to_fit();
        
        entries.len()
    }
    
    pub fn memory_usage(&self) -> usize {
        let entries = self.entries.read();
        let id_index = self.id_index.read();
        
        let entries_size: usize = entries.iter().map(|e| {
            std::mem::size_of::<VectorEntry>() + 
            e.vector.len() * std::mem::size_of::<f32>() +
            e.metadata.as_ref().map(|m| m.len()).unwrap_or(0)
        }).sum();
        
        let index_size = id_index.len() * (std::mem::size_of::<u64>() + std::mem::size_of::<usize>());
        
        entries_size + index_size
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    
    #[test]
    fn test_insert_and_get() {
        let db = VectorDB::new(3, DistanceMetric::Cosine);
        let vector = vec![1.0, 2.0, 3.0];
        
        db.insert(1, &vector, None).unwrap();
        assert_eq!(db.len(), 1);
        assert!(db.is_dirty());
        
        let entry = db.get(1).unwrap();
        assert_eq!(entry.id, 1);
    }
    
    #[test]
    fn test_insert_batch() {
        let db = VectorDB::new(3, DistanceMetric::Cosine);
        
        let entries = vec![
            (1, vec![1.0, 0.0, 0.0], None),
            (2, vec![0.0, 1.0, 0.0], Some(vec![1, 2, 3])),
            (3, vec![0.0, 0.0, 1.0], None),
        ];
        
        let count = db.insert_batch(entries).unwrap();
        assert_eq!(count, 3);
        assert_eq!(db.len(), 3);
        assert!(db.is_dirty());
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
    fn test_save_and_load() {
        let temp_path = std::env::temp_dir().join("test_vdb_persistence.bin");
        
        let db = VectorDB::new(128, DistanceMetric::Cosine);
        
        for i in 0..100 {
            let vector: Vec<f32> = (0..128).map(|j| (i * 128 + j) as f32).collect();
            db.insert(i, &vector, Some(vec![1, 2, 3])).unwrap();
        }
        
        assert!(db.is_dirty());
        
        db.save(&temp_path).unwrap();
        assert!(!db.is_dirty());
        
        let loaded_db = VectorDB::load(&temp_path).unwrap();
        
        assert_eq!(loaded_db.len(), 100);
        assert_eq!(loaded_db.dimension(), 128);
        assert_eq!(loaded_db.metric(), DistanceMetric::Cosine);
        assert!(!loaded_db.is_dirty());
        
        for i in 0..100 {
            let entry = loaded_db.get(i).unwrap();
            assert_eq!(entry.id, i);
            assert_eq!(entry.metadata, Some(vec![1, 2, 3]));
        }
        
        let query: Vec<f32> = (0..128).map(|i| i as f32).collect();
        let results = loaded_db.search(&query, 10).unwrap();
        assert_eq!(results.len(), 10);
        
        fs::remove_file(temp_path).ok();
    }
    
    #[test]
    fn test_save_and_load_bytes() {
        let db = VectorDB::new(64, DistanceMetric::Euclidean);
        
        for i in 0..50 {
            let vector: Vec<f32> = (0..64).map(|j| (i * 64 + j) as f32).collect();
            db.insert(i, &vector, None).unwrap();
        }
        
        let bytes = db.save_to_bytes().unwrap();
        
        let loaded_db = VectorDB::load_from_bytes(&bytes).unwrap();
        
        assert_eq!(loaded_db.len(), 50);
        assert_eq!(loaded_db.dimension(), 64);
        assert_eq!(loaded_db.metric(), DistanceMetric::Euclidean);
        
        for i in 0..50 {
            let entry = loaded_db.get(i).unwrap();
            assert_eq!(entry.id, i);
        }
    }
    
    #[test]
    fn test_clear() {
        let db = VectorDB::new(3, DistanceMetric::Cosine);
        
        db.insert(1, &[1.0, 0.0, 0.0], None).unwrap();
        db.insert(2, &[0.0, 1.0, 0.0], None).unwrap();
        
        assert_eq!(db.len(), 2);
        
        db.clear();
        
        assert_eq!(db.len(), 0);
        assert!(db.get(1).is_none());
        assert!(db.get(2).is_none());
    }
    
    #[test]
    fn test_memory_usage() {
        let db = VectorDB::new(128, DistanceMetric::Cosine);
        
        for i in 0..100 {
            let vector: Vec<f32> = (0..128).map(|j| j as f32).collect();
            db.insert(i, &vector, None).unwrap();
        }
        
        let mem = db.memory_usage();
        assert!(mem > 0);
        
        println!("Memory usage for 100 vectors (128-dim): {} bytes", mem);
    }
}
