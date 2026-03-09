use std::sync::Arc;
use parking_lot::RwLock;
use dashmap::DashMap;
use smallvec::SmallVec;

use crate::distance::{self, DistanceMetric};
use crate::error::{Error, Result};

const DEFAULT_CAPACITY: usize = 1024;
const DEFAULT_CLUSTERS: usize = 32;

#[derive(Debug, Clone)]
pub struct VectorEntry {
    pub id: u64,
    pub vector: Vec<f32>,
    pub metadata: Option<Vec<u8>>,
}

impl VectorEntry {
    pub fn new(id: u64, vector: Vec<f32>, metadata: Option<Vec<u8>>) -> Self {
        Self { id, vector, metadata }
    }
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
    dimension: u32,
    entries: Arc<RwLock<Vec<VectorEntry>>>,
    id_map: Arc<DashMap<u64, usize>>,
    metric: DistanceMetric,
    use_index: bool,
    ivf_built: bool,
    num_clusters: usize,
    cluster_centers: Option<Vec<Vec<f32>>>,
    clusters: Option<Vec<SmallVec<[usize; 64]>>>,
    stats: Stats,
}

impl VectorDB {
    pub fn new(dimension: u32) -> Self {
        Self {
            dimension,
            entries: Arc::new(RwLock::new(Vec::with_capacity(DEFAULT_CAPACITY))),
            id_map: Arc::new(DashMap::with_capacity(DEFAULT_CAPACITY)),
            metric: DistanceMetric::Cosine,
            use_index: false,
            ivf_built: false,
            num_clusters: 0,
            cluster_centers: None,
            clusters: None,
            stats: Stats::default(),
        }
    }

    pub fn with_metric(dimension: u32, metric: DistanceMetric) -> Self {
        let mut db = Self::new(dimension);
        db.metric = metric;
        db
    }

    pub fn dimension(&self) -> u32 {
        self.dimension
    }

    pub fn len(&self) -> usize {
        self.entries.read().len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn insert(&self, id: u64, vector: Vec<f32>, metadata: Option<Vec<u8>>) -> Result<()> {
        if vector.len() != self.dimension as usize {
            return Err(Error::InvalidDimension);
        }

        if self.id_map.contains_key(&id) {
            return Err(Error::DuplicateId);
        }

        let mut entries = self.entries.write();
        let index = entries.len();
        
        let mut entry_vector = vector;
        if self.metric == DistanceMetric::Cosine {
            distance::normalize(&mut entry_vector);
        }
        
        let entry = VectorEntry::new(id, entry_vector, metadata);
        
        self.id_map.insert(id, index);
        entries.push(entry);

        Ok(())
    }

    pub fn delete(&self, id: u64) -> Result<()> {
        if let Some((_, index)) = self.id_map.remove(&id) {
            let mut entries = self.entries.write();
            if index < entries.len() {
                entries.remove(index);
            }
            Ok(())
        } else {
            Err(Error::NotFound)
        }
    }

    pub fn get(&self, id: u64) -> Option<VectorEntry> {
        if let Some(index) = self.id_map.get(&id) {
            let entries = self.entries.read();
            entries.get(*index).cloned()
        } else {
            None
        }
    }

    pub fn search(&self, query: &[f32], top_k: usize) -> Vec<SearchResult> {
        if query.len() != self.dimension as usize || self.is_empty() {
            return Vec::new();
        }

        let mut query_vec = query.to_vec();
        if self.metric == DistanceMetric::Cosine {
            distance::normalize(&mut query_vec);
        }

        let entries = self.entries.read();
        
        let mut results: Vec<(usize, f32)> = entries
            .iter()
            .enumerate()
            .map(|(idx, entry)| {
                let dist = distance::compute_distance(&query_vec, &entry.vector, self.metric);
                (idx, dist)
            })
            .collect();

        results.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

        let results: Vec<SearchResult> = results
            .into_iter()
            .take(top_k)
            .map(|(idx, dist)| {
                let entry = &entries[idx];
                SearchResult {
                    id: entry.id,
                    distance: dist,
                    metadata: entry.metadata.clone(),
                }
            })
            .collect();

        results
    }

    pub fn par_search(&self, query: &[f32], top_k: usize) -> Vec<SearchResult> {
        if query.len() != self.dimension as usize || self.is_empty() {
            return Vec::new();
        }

        let mut query_vec = query.to_vec();
        if self.metric == DistanceMetric::Cosine {
            distance::normalize(&mut query_vec);
        }

        let entries = self.entries.read();
        
        use rayon::prelude::*;
        
        let mut results: Vec<(usize, f32)> = entries
            .par_iter()
            .enumerate()
            .map(|(idx, entry)| {
                let dist = distance::compute_distance(&query_vec, &entry.vector, self.metric);
                (idx, dist)
            })
            .collect();

        results.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

        results
            .into_iter()
            .take(top_k)
            .map(|(idx, dist)| {
                let entry = &entries[idx];
                SearchResult {
                    id: entry.id,
                    distance: dist,
                    metadata: entry.metadata.clone(),
                }
            })
            .collect()
    }

    pub fn build_ivf_index(&mut self, num_clusters: usize) -> Result<()> {
        let entries = self.entries.read();
        if entries.is_empty() {
            return Err(Error::InvalidInput);
        }

        let k = if num_clusters == 0 { DEFAULT_CLUSTERS } else { num_clusters };
        let k = k.min(entries.len());

        let mut centers: Vec<Vec<f32>> = Vec::with_capacity(k);
        use rand::Rng;
        let mut rng = rand::thread_rng();
        
        let mut selected: std::collections::HashSet<usize> = std::collections::HashSet::new();
        while selected.len() < k {
            let idx = rng.gen_range(0..entries.len());
            selected.insert(idx);
        }
        
        for idx in selected {
            centers.push(entries[idx].vector.clone());
        }

        let mut assignments = vec![0usize; entries.len()];
        
        for _ in 0..10 {
            for (i, entry) in entries.iter().enumerate() {
                let mut min_dist = f32::MAX;
                let mut best = 0;
                for (j, center) in centers.iter().enumerate() {
                    let dist = distance::euclidean_distance(&entry.vector, center);
                    if dist < min_dist {
                        min_dist = dist;
                        best = j;
                    }
                }
                assignments[i] = best;
            }

            let mut new_centers: Vec<Vec<f32>> = vec![vec![0.0; self.dimension as usize]; k];
            let mut counts = vec![0usize; k];

            for (i, &cluster) in assignments.iter().enumerate() {
                for (j, val) in entries[i].vector.iter().enumerate() {
                    new_centers[cluster][j] += val;
                }
                counts[cluster] += 1;
            }

            for (i, center) in new_centers.iter_mut().enumerate() {
                if counts[i] > 0 {
                    for val in center.iter_mut() {
                        *val /= counts[i] as f32;
                    }
                }
            }
            centers = new_centers;
        }

        let mut clusters: Vec<SmallVec<[usize; 64]>> = vec![SmallVec::new(); k];
        for (i, &cluster) in assignments.iter().enumerate() {
            clusters[cluster].push(i);
        }

        drop(entries);

        self.cluster_centers = Some(centers);
        self.clusters = Some(clusters);
        self.num_clusters = k;
        self.ivf_built = true;

        Ok(())
    }

    pub fn search_ivf(&self, query: &[f32], top_k: usize, nprobe: usize) -> Vec<SearchResult> {
        if query.len() != self.dimension as usize || self.is_empty() {
            return Vec::new();
        }

        if !self.ivf_built {
            return self.search(query, top_k);
        }

        let query_vec = if self.metric == DistanceMetric::Cosine {
            let mut v = query.to_vec();
            distance::normalize(&mut v);
            v
        } else {
            query.to_vec()
        };

        let centers = self.cluster_centers.as_ref().unwrap();
        
        let mut cluster_dists: Vec<(usize, f32)> = centers
            .iter()
            .enumerate()
            .map(|(i, center)| {
                let dist = distance::euclidean_distance(&query_vec, center);
                (i, dist)
            })
            .collect();
        
        cluster_dists.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

        let nprobe = nprobe.min(cluster_dists.len());
        
        let clusters = self.clusters.as_ref().unwrap();

        let entries = self.entries.read();
        
        let mut candidates: Vec<(usize, f32)> = Vec::new();
        
        for i in 0..nprobe {
            let cluster_id = cluster_dists[i].0;
            for &idx in &clusters[cluster_id] {
                let dist = distance::compute_distance(&query_vec, &entries[idx].vector, self.metric);
                candidates.push((idx, dist));
            }
        }

        candidates.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

        candidates
            .into_iter()
            .take(top_k)
            .map(|(idx, dist)| {
                let entry = &entries[idx];
                SearchResult {
                    id: entry.id,
                    distance: dist,
                    metadata: entry.metadata.clone(),
                }
            })
            .collect()
    }

    pub fn batch_search(&self, queries: &[Vec<f32>], top_k: usize) -> Vec<Vec<SearchResult>> {
        queries
            .iter()
            .map(|q| self.search(q, top_k))
            .collect()
    }

    pub fn par_batch_search(&self, queries: &[Vec<f32>], top_k: usize) -> Vec<Vec<SearchResult>> {
        use rayon::prelude::*;
        
        queries
            .par_iter()
            .map(|q| self.search(q, top_k))
            .collect()
    }

    pub fn set_index(&mut self, enable: bool) {
        self.use_index = enable;
    }

    pub fn build_index(&mut self) -> Result<()> {
        self.use_index = true;
        Ok(())
    }

    pub fn stats(&self) -> Stats {
        self.stats.clone()
    }

    pub fn reset_stats(&mut self) {
        self.stats = Stats::default();
    }

    pub fn print_stats(&self) {
        println!("=== Minimax25 VectorDB Statistics ===");
        println!("Dimension: {}", self.dimension);
        println!("Size: {} / {}", self.len(), DEFAULT_CAPACITY);
        println!("IVF Built: {}", self.ivf_built);
        println!("Clusters: {}", self.num_clusters);
        println!("\nOperations:");
        println!("  Insert:  {}", self.stats.insert_count);
        println!("  Delete:  {}", self.stats.delete_count);
        println!("  Search:  {}", self.stats.search_count);
        println!("  Get:     {}", self.stats.get_count);
        println!("======================================");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert_and_search() {
        let db = VectorDB::new(3);
        
        db.insert(1, vec![1.0, 0.0, 0.0], None).unwrap();
        db.insert(2, vec![0.0, 1.0, 0.0], None).unwrap();
        db.insert(3, vec![0.0, 0.0, 1.0], None).unwrap();
        
        let results = db.search(&[1.0, 0.0, 0.0], 2);
        
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].id, 1);
    }

    #[test]
    fn test_delete() {
        let db = VectorDB::new(3);
        
        db.insert(1, vec![1.0, 0.0, 0.0], None).unwrap();
        db.delete(1).unwrap();
        
        assert!(db.get(1).is_none());
    }

    #[test]
    fn test_ivf_index() {
        let mut db = VectorDB::new(4);
        
        for i in 0..100 {
            db.insert(i as u64, vec![i as f32, i as f32, i as f32, i as f32], None).unwrap();
        }
        
        db.build_ivf_index(10).unwrap();
        
        let results = db.search_ivf(&[50.0, 50.0, 50.0, 50.0], 5, 3);
        
        assert!(!results.is_empty());
    }
}
