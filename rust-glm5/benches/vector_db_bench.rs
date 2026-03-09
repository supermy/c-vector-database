use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;

use rust_glm5::{VectorDB, DistanceMetric};
use std::fs;

fn generate_random_vector(dim: usize, rng: &mut StdRng) -> Vec<f32> {
    (0..dim).map(|_| rng.gen::<f32>()).collect()
}

fn bench_insert(c: &mut Criterion) {
    let mut group = c.benchmark_group("insert");
    
    for size in [100, 1_000, 10_000].iter() {
        let dim = 128;
        
        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(BenchmarkId::new("insert", size), size, |b, size| {
            b.iter(|| {
                let db = VectorDB::new(dim, DistanceMetric::Cosine);
                let mut rng = StdRng::seed_from_u64(42);
                for i in 0..*size {
                    let vector = generate_random_vector(dim, &mut rng);
                    db.insert(i as u64, &vector, None).unwrap();
                }
            });
        });
    }
    
    group.finish();
}

fn bench_insert_batch(c: &mut Criterion) {
    let mut group = c.benchmark_group("insert_batch");
    
    for size in [100, 1_000, 10_000].iter() {
        let dim = 128;
        let mut rng = StdRng::seed_from_u64(42);
        
        let entries: Vec<(u64, Vec<f32>, Option<Vec<u8>>)> = (0..*size)
            .map(|i| {
                let vector = generate_random_vector(dim, &mut rng);
                (i as u64, vector, None)
            })
            .collect();
        
        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(BenchmarkId::new("batch", size), size, |b, _| {
            b.iter(|| {
                let db = VectorDB::new(dim, DistanceMetric::Cosine);
                db.insert_batch(entries.clone()).unwrap();
            });
        });
    }
    
    group.finish();
}

fn bench_search(c: &mut Criterion) {
    let mut group = c.benchmark_group("search");
    
    for size in [100, 1_000, 10_000, 50_000].iter() {
        let dim = 128;
        let db = VectorDB::new(dim, DistanceMetric::Cosine);
        let mut rng = StdRng::seed_from_u64(42);
        
        for i in 0..*size {
            let vector = generate_random_vector(dim, &mut rng);
            db.insert(i as u64, &vector, None).unwrap();
        }
        
        let query = generate_random_vector(dim, &mut rng);
        
        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(BenchmarkId::new("search_k10", size), size, |b, _| {
            b.iter(|| {
                db.search(black_box(&query), 10).unwrap();
            });
        });
    }
    
    group.finish();
}

fn bench_search_k_values(c: &mut Criterion) {
    let mut group = c.benchmark_group("search_k_values");
    
    let size = 10_000;
    let dim = 128;
    let db = VectorDB::new(dim, DistanceMetric::Cosine);
    let mut rng = StdRng::seed_from_u64(42);
    
    for i in 0..size {
        let vector = generate_random_vector(dim, &mut rng);
        db.insert(i as u64, &vector, None).unwrap();
    }
    
    let query = generate_random_vector(dim, &mut rng);
    
    for k in [1, 10, 50, 100, 500].iter() {
        group.bench_with_input(BenchmarkId::new("k", k), k, |b, _| {
            b.iter(|| {
                db.search(black_box(&query), *k).unwrap();
            });
        });
    }
    
    group.finish();
}

fn bench_distance_metrics(c: &mut Criterion) {
    let mut group = c.benchmark_group("distance_metrics");
    
    let dim = 128;
    let size = 1_000;
    
    for metric in [DistanceMetric::Cosine, DistanceMetric::Euclidean, DistanceMetric::DotProduct].iter() {
        let db = VectorDB::new(dim, *metric);
        let mut rng = StdRng::seed_from_u64(42);
        
        for i in 0..size {
            let vector = generate_random_vector(dim, &mut rng);
            db.insert(i as u64, &vector, None).unwrap();
        }
        
        let query = generate_random_vector(dim, &mut rng);
        
        group.bench_with_input(BenchmarkId::new(format!("{:?}", metric), size), metric, |b, _| {
            b.iter(|| {
                db.search(black_box(&query), 10).unwrap();
            });
        });
    }
    
    group.finish();
}

fn bench_dimensions(c: &mut Criterion) {
    let mut group = c.benchmark_group("dimensions");
    
    let size = 1_000;
    
    for dim in [64, 128, 256, 512, 768, 1024].iter() {
        let db = VectorDB::new(*dim, DistanceMetric::Cosine);
        let mut rng = StdRng::seed_from_u64(42);
        
        for i in 0..size {
            let vector = generate_random_vector(*dim, &mut rng);
            db.insert(i as u64, &vector, None).unwrap();
        }
        
        let query = generate_random_vector(*dim, &mut rng);
        
        group.bench_with_input(BenchmarkId::new("dim", dim), dim, |b, _| {
            b.iter(|| {
                db.search(black_box(&query), 10).unwrap();
            });
        });
    }
    
    group.finish();
}

fn bench_batch_search(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_search");
    
    let size = 10_000;
    let dim = 128;
    let db = VectorDB::new(dim, DistanceMetric::Cosine);
    let mut rng = StdRng::seed_from_u64(42);
    
    for i in 0..size {
        let vector = generate_random_vector(dim, &mut rng);
        db.insert(i as u64, &vector, None).unwrap();
    }
    
    for batch_size in [1, 10, 50, 100].iter() {
        let queries: Vec<Vec<f32>> = (0..*batch_size)
            .map(|_| generate_random_vector(dim, &mut rng))
            .collect();
        let query_refs: Vec<&[f32]> = queries.iter().map(|q| q.as_slice()).collect();
        
        group.bench_with_input(BenchmarkId::new("batch", batch_size), batch_size, |b, _| {
            b.iter(|| {
                db.batch_search(black_box(&query_refs), 10).unwrap();
            });
        });
    }
    
    group.finish();
}

fn bench_persistence(c: &mut Criterion) {
    let mut group = c.benchmark_group("persistence");
    
    let temp_path = std::env::temp_dir().join("bench_vdb_persistence.bin");
    let dim = 128;
    
    for size in [100, 1_000, 10_000].iter() {
        let db = VectorDB::new(dim, DistanceMetric::Cosine);
        let mut rng = StdRng::seed_from_u64(42);
        
        for i in 0..*size {
            let vector = generate_random_vector(dim, &mut rng);
            db.insert(i as u64, &vector, Some(vec![1, 2, 3])).unwrap();
        }
        
        group.throughput(Throughput::Elements(*size as u64));
        
        group.bench_with_input(BenchmarkId::new("save_file", size), size, |b, _| {
            b.iter(|| {
                db.save(&temp_path).unwrap();
            });
        });
        
        group.bench_with_input(BenchmarkId::new("load_file", size), size, |b, _| {
            b.iter(|| {
                let _db = VectorDB::load(&temp_path).unwrap();
            });
        });
        
        group.bench_with_input(BenchmarkId::new("save_bytes", size), size, |b, _| {
            b.iter(|| {
                let _bytes = db.save_to_bytes().unwrap();
            });
        });
        
        let bytes = db.save_to_bytes().unwrap();
        group.bench_with_input(BenchmarkId::new("load_bytes", size), size, |b, _| {
            b.iter(|| {
                let _db = VectorDB::load_from_bytes(&bytes).unwrap();
            });
        });
    }
    
    fs::remove_file(&temp_path).ok();
    group.finish();
}

fn bench_concurrent_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("concurrent");
    
    let size = 10_000;
    let dim = 128;
    let db = VectorDB::new(dim, DistanceMetric::Cosine);
    let mut rng = StdRng::seed_from_u64(42);
    
    for i in 0..size {
        let vector = generate_random_vector(dim, &mut rng);
        db.insert(i as u64, &vector, None).unwrap();
    }
    
    let _queries: Vec<Vec<f32>> = (0..100)
        .map(|_| generate_random_vector(dim, &mut rng))
        .collect();
    
    group.bench_function("mixed_read_write", |b| {
        b.iter(|| {
            use std::sync::Arc;
            use std::thread;
            
            let db = Arc::new(VectorDB::new(dim, DistanceMetric::Cosine));
            let mut handles = vec![];
            
            let db_clone = Arc::clone(&db);
            handles.push(thread::spawn(move || {
                let mut rng = StdRng::seed_from_u64(42);
                for i in 0..1000 {
                    let vector = generate_random_vector(dim, &mut rng);
                    db_clone.insert(i, &vector, None).unwrap();
                }
            }));
            
            let db_clone = Arc::clone(&db);
            handles.push(thread::spawn(move || {
                let mut rng = StdRng::seed_from_u64(43);
                for _ in 0..100 {
                    let query = generate_random_vector(dim, &mut rng);
                    let _ = db_clone.search(&query, 10);
                }
            }));
            
            for handle in handles {
                handle.join().unwrap();
            }
        });
    });
    
    group.finish();
}

criterion_group!(
    benches,
    bench_insert,
    bench_insert_batch,
    bench_search,
    bench_search_k_values,
    bench_distance_metrics,
    bench_dimensions,
    bench_batch_search,
    bench_persistence,
    bench_concurrent_operations,
);
criterion_main!(benches);
