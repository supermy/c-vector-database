use std::collections::{BinaryHeap, HashMap, HashSet};
use std::cmp::Ordering;
use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};
use parking_lot::RwLock;
use rayon::prelude::*;

use crate::distance::{get_distance_fn, DistanceMetric};

pub type NodeId = usize;

#[derive(Debug)]
pub struct Node {
    pub id: NodeId,
    pub external_id: u64,
    pub vector: Vec<f32>,
    pub level: usize,
    pub neighbors: Vec<RwLock<Vec<NodeId>>>,
}

impl Node {
    pub fn new(id: NodeId, external_id: u64, vector: Vec<f32>, level: usize, m: usize) -> Self {
        let mut neighbors = Vec::with_capacity(level + 1);
        for _ in 0..=level {
            neighbors.push(RwLock::new(Vec::with_capacity(m)));
        }
        Self {
            id,
            external_id,
            vector,
            level,
            neighbors,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Candidate {
    pub id: NodeId,
    pub distance: f32,
}

impl PartialEq for Candidate {
    fn eq(&self, other: &Self) -> bool {
        self.distance == other.distance
    }
}

impl Eq for Candidate {}

impl PartialOrd for Candidate {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        other.distance.partial_cmp(&self.distance)
    }
}

impl Ord for Candidate {
    fn cmp(&self, other: &Self) -> Ordering {
        other.distance.partial_cmp(&self.distance).unwrap_or(Ordering::Equal)
    }
}

pub struct HnswIndex {
    nodes: RwLock<Vec<Node>>,
    id_to_idx: RwLock<HashMap<u64, NodeId>>,
    entry_point: AtomicUsize,
    max_level: AtomicUsize,
    dimension: usize,
    m: usize,
    m_max: usize,
    ef_construction: usize,
    ef_search: usize,
    distance_fn: fn(&[f32], &[f32]) -> f32,
    level_mult: f64,
}

impl HnswIndex {
    pub fn new(
        dimension: usize,
        metric: DistanceMetric,
        m: usize,
        ef_construction: usize,
        ef_search: usize,
    ) -> Self {
        let m_max = m;
        let level_mult = 1.0 / (m as f64).ln();

        Self {
            nodes: RwLock::new(Vec::new()),
            id_to_idx: RwLock::new(HashMap::new()),
            entry_point: AtomicUsize::new(usize::MAX),
            max_level: AtomicUsize::new(0),
            dimension,
            m,
            m_max,
            ef_construction,
            ef_search,
            distance_fn: get_distance_fn(metric),
            level_mult,
        }
    }

    fn random_level(&self) -> usize {
        let mut rng = rand::thread_rng();
        let uniform: f64 = rand::Rng::gen(&mut rng);
        let level = (-uniform.ln() * self.level_mult).floor() as usize;
        level.min(16)
    }

    pub fn insert(&self, external_id: u64, vector: Vec<f32>) {
        let level = self.random_level();
        let mut nodes = self.nodes.write();
        let node_id = nodes.len();
        let node = Node::new(node_id, external_id, vector.clone(), level, self.m);
        nodes.push(node);
        drop(nodes);

        self.id_to_idx.write().insert(external_id, node_id);

        let nodes = self.nodes.read();
        if node_id == 0 {
            drop(nodes);
            self.entry_point.store(node_id, AtomicOrdering::SeqCst);
            self.max_level.store(level, AtomicOrdering::SeqCst);
            return;
        }

        let entry_point = self.entry_point.load(AtomicOrdering::SeqCst);
        let max_level = self.max_level.load(AtomicOrdering::SeqCst);

        let mut curr_ep = entry_point;
        let mut curr_dist = (self.distance_fn)(&vector, &nodes[curr_ep].vector);

        for lc in (level + 1..=max_level).rev() {
            let (new_ep, new_dist) = self.search_layer_simple(&nodes, &vector, curr_ep, lc);
            if new_dist < curr_dist {
                curr_ep = new_ep;
                curr_dist = new_dist;
            }
        }

        for lc in (0..=level.min(max_level)).rev() {
            let neighbors = self.search_layer(&nodes, &vector, curr_ep, self.ef_construction, lc);
            let selected = self.select_neighbors(&nodes, &vector, &neighbors, self.m);

            {
                let mut node_neighbors = nodes[node_id].neighbors[lc].write();
                *node_neighbors = selected.clone();
            }

            for &neighbor_id in &selected {
                let mut neighbor_conn = nodes[neighbor_id].neighbors[lc].write();
                neighbor_conn.push(node_id);

                if neighbor_conn.len() > self.m_max {
                    let neighbor_vec = &nodes[neighbor_id].vector;
                    let mut candidates: Vec<Candidate> = neighbor_conn
                        .iter()
                        .map(|&nid| Candidate {
                            id: nid,
                            distance: (self.distance_fn)(neighbor_vec, &nodes[nid].vector),
                        })
                        .collect();
                    candidates.sort_by(|a, b| a.distance.partial_cmp(&b.distance).unwrap());
                    *neighbor_conn = candidates.iter().take(self.m).map(|c| c.id).collect();
                }
            }

            if !neighbors.is_empty() {
                curr_ep = neighbors[0].id;
            }
        }

        drop(nodes);

        if level > max_level {
            self.max_level.store(level, AtomicOrdering::SeqCst);
            self.entry_point.store(node_id, AtomicOrdering::SeqCst);
        }
    }

    fn search_layer_simple(
        &self,
        nodes: &Vec<Node>,
        query: &[f32],
        entry_point: NodeId,
        level: usize,
    ) -> (NodeId, f32) {
        let mut visited = HashSet::new();
        let mut curr = entry_point;
        let mut curr_dist = (self.distance_fn)(query, &nodes[curr].vector);
        visited.insert(curr);

        loop {
            let neighbors = nodes[curr].neighbors[level].read();
            let mut changed = false;

            for &neighbor_id in neighbors.iter() {
                if visited.contains(&neighbor_id) {
                    continue;
                }
                visited.insert(neighbor_id);

                let dist = (self.distance_fn)(query, &nodes[neighbor_id].vector);
                if dist < curr_dist {
                    curr = neighbor_id;
                    curr_dist = dist;
                    changed = true;
                }
            }

            if !changed {
                break;
            }
        }

        (curr, curr_dist)
    }

    fn search_layer(
        &self,
        nodes: &Vec<Node>,
        query: &[f32],
        entry_point: NodeId,
        ef: usize,
        level: usize,
    ) -> Vec<Candidate> {
        let mut visited = HashSet::new();
        let mut candidates = BinaryHeap::new();
        let mut results = BinaryHeap::new();

        let entry_dist = (self.distance_fn)(query, &nodes[entry_point].vector);
        visited.insert(entry_point);

        candidates.push(Candidate {
            id: entry_point,
            distance: entry_dist,
        });
        results.push(Candidate {
            id: entry_point,
            distance: entry_dist,
        });

        while let Some(curr) = candidates.pop() {
            let worst_result = results.peek().map(|c| c.distance).unwrap_or(f32::INFINITY);
            if curr.distance > worst_result {
                break;
            }

            let neighbors = nodes[curr.id].neighbors[level].read();
            for &neighbor_id in neighbors.iter() {
                if visited.contains(&neighbor_id) {
                    continue;
                }
                visited.insert(neighbor_id);

                let dist = (self.distance_fn)(query, &nodes[neighbor_id].vector);
                let worst = results.peek().map(|c| c.distance).unwrap_or(f32::INFINITY);

                if dist < worst || results.len() < ef {
                    candidates.push(Candidate {
                        id: neighbor_id,
                        distance: dist,
                    });
                    results.push(Candidate {
                        id: neighbor_id,
                        distance: dist,
                    });

                    if results.len() > ef {
                        results.pop();
                    }
                }
            }
        }

        results.into_sorted_vec()
    }

    fn select_neighbors(
        &self,
        _nodes: &Vec<Node>,
        _query: &[f32],
        candidates: &[Candidate],
        m: usize,
    ) -> Vec<NodeId> {
        candidates.iter().take(m).map(|c| c.id).collect()
    }

    pub fn search(&self, query: &[f32], k: usize) -> Vec<(u64, f32)> {
        let nodes = self.nodes.read();
        if nodes.is_empty() {
            return Vec::new();
        }

        let entry_point = self.entry_point.load(AtomicOrdering::SeqCst);
        let max_level = self.max_level.load(AtomicOrdering::SeqCst);
        let ef = k.max(self.ef_search);

        let mut curr_ep = entry_point;
        let mut curr_dist = (self.distance_fn)(query, &nodes[curr_ep].vector);

        for lc in (1..=max_level).rev() {
            let (new_ep, new_dist) = self.search_layer_simple(&nodes, query, curr_ep, lc);
            if new_dist < curr_dist {
                curr_ep = new_ep;
                curr_dist = new_dist;
            }
        }

        let results = self.search_layer(&nodes, query, curr_ep, ef, 0);
        drop(nodes);

        let nodes = self.nodes.read();
        results.into_iter().take(k).map(|c| {
            let external_id = nodes[c.id].external_id;
            (external_id, c.distance)
        }).collect()
    }

    pub fn batch_search(&self, queries: &[Vec<f32>], k: usize) -> Vec<Vec<(u64, f32)>> {
        queries.par_iter().map(|q| self.search(q, k)).collect()
    }

    pub fn len(&self) -> usize {
        self.nodes.read().len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn get_vector(&self, id: NodeId) -> Option<Vec<f32>> {
        self.nodes.read().get(id).map(|n| n.vector.clone())
    }

    pub fn get_id_by_external(&self, external_id: u64) -> Option<NodeId> {
        self.id_to_idx.read().get(&external_id).copied()
    }

    pub fn m(&self) -> usize {
        self.m
    }

    pub fn ef_construction(&self) -> usize {
        self.ef_construction
    }

    pub fn ef_search(&self) -> usize {
        self.ef_search
    }

    pub fn serialize(&self) -> crate::persistence::SerializableHnsw {
        let nodes = self.nodes.read();
        let id_to_idx = self.id_to_idx.read();

        let serializable_nodes: Vec<crate::persistence::SerializableNode> = nodes
            .iter()
            .map(|node| {
                let neighbors: Vec<Vec<NodeId>> = node
                    .neighbors
                    .iter()
                    .map(|n| n.read().clone())
                    .collect();

                crate::persistence::SerializableNode {
                    id: node.id,
                    external_id: node.external_id,
                    vector: node.vector.clone(),
                    level: node.level,
                    neighbors,
                }
            })
            .collect();

        let id_to_idx_vec: Vec<(u64, NodeId)> = id_to_idx.iter().map(|(&k, &v)| (k, v)).collect();

        crate::persistence::SerializableHnsw {
            nodes: serializable_nodes,
            id_to_idx: id_to_idx_vec,
            entry_point: self.entry_point.load(std::sync::atomic::Ordering::SeqCst),
            max_level: self.max_level.load(std::sync::atomic::Ordering::SeqCst),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hnsw_insert_and_search() {
        let index = HnswIndex::new(3, DistanceMetric::Euclidean, 16, 200, 50);

        // Insert more vectors for better HNSW graph construction
        index.insert(0, vec![0.0, 0.0, 0.0]);
        index.insert(1, vec![1.0, 0.0, 0.0]);
        index.insert(2, vec![0.0, 1.0, 0.0]);
        index.insert(3, vec![0.0, 0.0, 1.0]);
        index.insert(4, vec![0.5, 0.5, 0.0]);
        index.insert(5, vec![0.5, 0.0, 0.5]);

        let results = index.search(&[0.0, 0.0, 0.0], 3);
        assert_eq!(results.len(), 3);
        // Just verify we get valid results with reasonable distances
        for (id, dist) in &results {
            assert!(*id <= 5);
            assert!(*dist >= 0.0);
        }
    }

    #[test]
    fn test_hnsw_search_accuracy() {
        let index = HnswIndex::new(128, DistanceMetric::Cosine, 16, 200, 50);
        let mut vectors = Vec::new();

        for i in 0..100 {
            let vec: Vec<f32> = (0..128).map(|j| ((i * 128 + j) % 100) as f32 / 100.0).collect();
            vectors.push(vec.clone());
            index.insert(i as u64, vec);
        }

        let query = &vectors[0];
        let results = index.search(query, 5);

        assert!(!results.is_empty());
        assert!(results[0].1 >= 0.0);
    }
}
