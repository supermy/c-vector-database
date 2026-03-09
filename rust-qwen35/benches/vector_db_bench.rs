use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use rand::Rng;
use rust_qwen35::{create_db, create_db_with_capacity, DistanceMetric, VectorDB};
use std::time::Instant;

fn generate_random_vector(dim: usize) -> Vec<f32> {
    let mut rng = rand::thread_rng();
    (0..dim).map(|_| rng.gen::<f32>()).collect()
}

fn benchmark_insert(c: &mut Criterion) {
    let mut group = c.benchmark_group("insert");
    let mut id_counter = 0i64;
    
    for dim in [64, 128, 256, 512, 768].iter() {
        let db = create_db(*dim, DistanceMetric::Cosine);
        
        group.bench_function(BenchmarkId::new("insert", dim), |b| {
            b.iter(|| {
                let vector = generate_random_vector(*dim);
                db.insert(black_box(id_counter), black_box(vector), None).unwrap();
                id_counter += 1;
            })
        });
    }
    
    group.finish();
}

fn benchmark_insert_scale(c: &mut Criterion) {
    let mut group = c.benchmark_group("insert_scale");
    
    for &size in [100, 1000, 10000].iter() {
        for dim in [128, 512].iter() {
            let db = create_db_with_capacity(*dim, DistanceMetric::Cosine, size);
            
            group.bench_function(
                BenchmarkId::new("insert", format!("dim{}_count{}", dim, size)),
                |b| {
                    b.iter(|| {
                        for i in 0..size {
                            let vector = generate_random_vector(*dim);
                            db.insert(i as i64, vector, None).unwrap();
                        }
                    })
                },
            );
        }
    }
    
    group.finish();
}

fn benchmark_search(c: &mut Criterion) {
    let mut group = c.benchmark_group("search");
    
    for &size in [100, 1000, 10000].iter() {
        for dim in [128, 512].iter() {
            let db = create_db_with_capacity(*dim, DistanceMetric::Cosine, size);
            
            for i in 0..size {
                let vector = generate_random_vector(*dim);
                db.insert(i as i64, vector, None).unwrap();
            }
            
            let query = generate_random_vector(*dim);
            
            group.bench_function(
                BenchmarkId::new("search", format!("dim{}_count{}", dim, size)),
                |b| {
                    b.iter(|| {
                        let results = db.search(black_box(&query), black_box(10)).unwrap();
                        black_box(results);
                    })
                },
            );
        }
    }
    
    group.finish();
}

fn benchmark_batch_search(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_search");
    
    let dim = 512;
    let size = 10000;
    let db = create_db_with_capacity(dim, DistanceMetric::Cosine, size);
    
    for i in 0..size {
        let vector = generate_random_vector(dim);
        db.insert(i as i64, vector, None).unwrap();
    }
    
    for &batch_size in [1, 10, 100].iter() {
        let queries: Vec<Vec<f32>> = (0..batch_size)
            .map(|_| generate_random_vector(dim))
            .collect();
        
        group.bench_function(
            BenchmarkId::new("batch_search", batch_size),
            |b| {
                b.iter(|| {
                    let results = db.search_batch(black_box(&queries), black_box(10)).unwrap();
                    black_box(results);
                })
            },
        );
    }
    
    group.finish();
}

fn benchmark_dimensions(c: &mut Criterion) {
    let mut group = c.benchmark_group("dimensions");
    
    for dim in [64, 128, 256, 512, 768, 1024].iter() {
        let db = create_db(*dim, DistanceMetric::Cosine);
        
        let vector = generate_random_vector(*dim);
        db.insert(1, vector, None).unwrap();
        
        let query = generate_random_vector(*dim);
        
        group.bench_function(BenchmarkId::new("dim", dim), |b| {
            b.iter(|| {
                let results = db.search(black_box(&query), black_box(5)).unwrap();
                black_box(results);
            })
        });
    }
    
    group.finish();
}

fn benchmark_distance_metrics(c: &mut Criterion) {
    let mut group = c.benchmark_group("distance_metrics");
    
    let dim = 512;
    let size = 1000;
    
    for metric in [
        DistanceMetric::Cosine,
        DistanceMetric::Euclidean,
        DistanceMetric::DotProduct,
        DistanceMetric::Manhattan,
    ]
    .iter()
    {
        let db = create_db_with_capacity(dim, *metric, size);
        
        for i in 0..size {
            let vector = generate_random_vector(dim);
            db.insert(i as i64, vector, None).unwrap();
        }
        
        let query = generate_random_vector(dim);
        
        group.bench_function(BenchmarkId::new("metric", format!("{:?}", metric)), |b| {
            b.iter(|| {
                let results = db.search(black_box(&query), black_box(10)).unwrap();
                black_box(results);
            })
        });
    }
    
    group.finish();
}

fn benchmark_parallel_insert(c: &mut Criterion) {
    let mut group = c.benchmark_group("parallel_insert");
    
    for &size in [1000, 5000, 10000].iter() {
        let dim = 512;
        
        group.bench_function(BenchmarkId::new("parallel", size), |b| {
            b.iter(|| {
                let db = create_db_with_capacity(dim, DistanceMetric::Cosine, size);
                
                let vectors: Vec<(i64, Vec<f32>)> = (0..size)
                    .map(|i| (i as i64, generate_random_vector(dim)))
                    .collect();
                
                let start = Instant::now();
                
                vectors.iter().for_each(|(id, vector)| {
                    db.insert(*id, vector.clone(), None).ok();
                });
                
                start.elapsed()
            })
        });
    }
    
    group.finish();
}

fn benchmark_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("throughput");
    
    let dim = 512;
    let size = 10000;
    let db = create_db_with_capacity(dim, DistanceMetric::Cosine, size);
    
    for i in 0..size {
        let vector = generate_random_vector(dim);
        db.insert(i as i64, vector, None).unwrap();
    }
    
    let query = generate_random_vector(dim);
    
    group.throughput(Throughput::Elements(1));
    group.bench_function("search_throughput", |b| {
        b.iter(|| {
            let results = db.search(black_box(&query), black_box(10)).unwrap();
            black_box(results);
        })
    });
    
    group.finish();
}

criterion_group!(
    benches,
    benchmark_insert,
    benchmark_insert_scale,
    benchmark_search,
    benchmark_batch_search,
    benchmark_dimensions,
    benchmark_distance_metrics,
    benchmark_parallel_insert,
    benchmark_throughput,
);

criterion_main!(benches);
