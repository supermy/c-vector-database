use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;

use rust_ds20code::{VectorDB, DistanceMetric};

fn generate_random_vector(rng: &mut StdRng, dim: usize) -> Vec<f32> {
    (0..dim).map(|_| rng.gen::<f32>()).collect()
}

fn generate_random_vectors(count: usize, dim: usize) -> Vec<Vec<f32>> {
    let mut rng = StdRng::seed_from_u64(42);
    (0..count).map(|_| generate_random_vector(&mut rng, dim)).collect()
}

fn bench_insert(c: &mut Criterion) {
    let mut group = c.benchmark_group("insert");
    
    for size in [100, 1000, 10000].iter() {
        group.throughput(Throughput::Elements(*size as u64));
        
        let vectors = generate_random_vectors(*size, 128);
        
        group.bench_with_input(BenchmarkId::new("flat", size), &size, |b, _| {
            b.iter(|| {
                let db = VectorDB::new(128, DistanceMetric::Cosine);
                for (i, vec) in vectors.iter().enumerate() {
                    db.insert(i as u64, vec, None).unwrap();
                }
                black_box(db);
            });
        });
        
        group.bench_with_input(BenchmarkId::new("hnsw", size), &size, |b, _| {
            b.iter(|| {
                let db = VectorDB::with_hnsw(128, DistanceMetric::Cosine);
                for (i, vec) in vectors.iter().enumerate() {
                    db.insert(i as u64, vec, None).unwrap();
                }
                black_box(db);
            });
        });
    }
    
    group.finish();
}

fn bench_search(c: &mut Criterion) {
    let mut group = c.benchmark_group("search");
    
    for size in [100, 1000, 10000].iter() {
        let vectors = generate_random_vectors(*size, 128);
        let query = generate_random_vector(&mut StdRng::seed_from_u64(123), 128);
        
        let db_flat = VectorDB::new(128, DistanceMetric::Cosine);
        for (i, vec) in vectors.iter().enumerate() {
            db_flat.insert(i as u64, vec, None).unwrap();
        }
        
        let db_hnsw = VectorDB::with_hnsw(128, DistanceMetric::Cosine);
        for (i, vec) in vectors.iter().enumerate() {
            db_hnsw.insert(i as u64, vec, None).unwrap();
        }
        
        group.throughput(Throughput::Elements(1));
        
        group.bench_with_input(BenchmarkId::new("flat", size), &size, |b, _| {
            b.iter(|| {
                black_box(db_flat.search(&query, 10).unwrap());
            });
        });
        
        group.bench_with_input(BenchmarkId::new("hnsw", size), &size, |b, _| {
            b.iter(|| {
                black_box(db_hnsw.search(&query, 10).unwrap());
            });
        });
    }
    
    group.finish();
}

fn bench_batch_search(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_search");
    
    let vectors = generate_random_vectors(10000, 128);
    let queries: Vec<Vec<f32>> = generate_random_vectors(100, 128);
    let query_refs: Vec<&[f32]> = queries.iter().map(|q| q.as_slice()).collect();
    
    let db_flat = VectorDB::new(128, DistanceMetric::Cosine);
    for (i, vec) in vectors.iter().enumerate() {
        db_flat.insert(i as u64, vec, None).unwrap();
    }
    
    let db_hnsw = VectorDB::with_hnsw(128, DistanceMetric::Cosine);
    for (i, vec) in vectors.iter().enumerate() {
        db_hnsw.insert(i as u64, vec, None).unwrap();
    }
    
    group.throughput(Throughput::Elements(100));
    
    group.bench_function("flat_10k", |b| {
        b.iter(|| {
            black_box(db_flat.batch_search(&query_refs, 10).unwrap());
        });
    });
    
    group.bench_function("hnsw_10k", |b| {
        b.iter(|| {
            black_box(db_hnsw.batch_search(&query_refs, 10).unwrap());
        });
    });
    
    group.finish();
}

fn bench_distance_metrics(c: &mut Criterion) {
    let mut group = c.benchmark_group("distance_metrics");
    
    let vectors = generate_random_vectors(10000, 128);
    let query = generate_random_vector(&mut StdRng::seed_from_u64(456), 128);
    
    for metric in [DistanceMetric::Cosine, DistanceMetric::Euclidean, DistanceMetric::DotProduct] {
        let db = VectorDB::new(128, metric);
        for (i, vec) in vectors.iter().enumerate() {
            db.insert(i as u64, vec, None).unwrap();
        }
        
        group.bench_function(format!("{:?}", metric), |b| {
            b.iter(|| {
                black_box(db.search(&query, 10).unwrap());
            });
        });
    }
    
    group.finish();
}

fn bench_dimensions(c: &mut Criterion) {
    let mut group = c.benchmark_group("dimensions");
    
    for dim in [64, 128, 256, 512, 768, 1024].iter() {
        let vectors = generate_random_vectors(10000, *dim);
        let query = generate_random_vector(&mut StdRng::seed_from_u64(789), *dim);
        
        let db = VectorDB::new(*dim, DistanceMetric::Cosine);
        for (i, vec) in vectors.iter().enumerate() {
            db.insert(i as u64, vec, None).unwrap();
        }
        
        group.throughput(Throughput::Bytes(*dim as u64 * 4));
        group.bench_with_input(BenchmarkId::from_parameter(dim), &dim, |b, _| {
            b.iter(|| {
                black_box(db.search(&query, 10).unwrap());
            });
        });
    }
    
    group.finish();
}

fn bench_hnsw_params(c: &mut Criterion) {
    let mut group = c.benchmark_group("hnsw_params");
    
    let vectors = generate_random_vectors(10000, 128);
    let query = generate_random_vector(&mut StdRng::seed_from_u64(999), 128);
    
    for m in [8, 16, 32, 64].iter() {
        let db = VectorDB::with_hnsw(128, DistanceMetric::Cosine);
        for (i, vec) in vectors.iter().enumerate() {
            db.insert(i as u64, vec, None).unwrap();
        }
        
        group.bench_with_input(BenchmarkId::new("m", m), &m, |b, _| {
            b.iter(|| {
                black_box(db.search(&query, 10).unwrap());
            });
        });
    }
    
    group.finish();
}

criterion_group! {
    name = benches;
    config = Criterion::default().sample_size(50);
    targets = bench_insert,
              bench_search,
              bench_batch_search,
              bench_distance_metrics,
              bench_dimensions,
              bench_hnsw_params,
}

criterion_main!(benches);
