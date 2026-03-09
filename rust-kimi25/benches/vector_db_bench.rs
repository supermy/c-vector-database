use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;

use rust_kimi25::{VectorDB, DistanceMetric, create_db, create_db_with_hnsw};

fn generate_random_vector(dim: usize, rng: &mut StdRng) -> Vec<f32> {
    (0..dim).map(|_| rng.gen::<f32>()).collect()
}

fn generate_normalized_vector(dim: usize, rng: &mut StdRng) -> Vec<f32> {
    let mut vec: Vec<f32> = (0..dim).map(|_| rng.gen::<f32>() - 0.5).collect();
    let norm: f32 = vec.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        vec.iter_mut().for_each(|x| *x /= norm);
    }
    vec
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

fn bench_insert_hnsw(c: &mut Criterion) {
    let mut group = c.benchmark_group("insert_hnsw");

    for size in [100, 1_000, 10_000].iter() {
        let dim = 128;

        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(BenchmarkId::new("insert_hnsw", size), size, |b, size| {
            b.iter(|| {
                let db = create_db_with_hnsw(dim, DistanceMetric::Cosine, 16, 200, 50);
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

fn bench_search_hnsw(c: &mut Criterion) {
    let mut group = c.benchmark_group("search_hnsw");

    for size in [100, 1_000, 10_000, 50_000, 100_000].iter() {
        let dim = 128;
        let db = create_db_with_hnsw(dim, DistanceMetric::Cosine, 16, 200, 50);
        let mut rng = StdRng::seed_from_u64(42);

        for i in 0..*size {
            let vector = generate_random_vector(dim, &mut rng);
            db.insert(i as u64, &vector, None).unwrap();
        }

        let query = generate_random_vector(dim, &mut rng);

        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(BenchmarkId::new("hnsw_search_k10", size), size, |b, _| {
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
    let db = create_db_with_hnsw(dim, DistanceMetric::Cosine, 16, 200, 50);
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

    for metric in [DistanceMetric::Cosine, DistanceMetric::Euclidean, DistanceMetric::DotProduct, DistanceMetric::Manhattan].iter() {
        let db = create_db_with_hnsw(dim, *metric, 16, 200, 50);
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
        let db = create_db_with_hnsw(*dim, DistanceMetric::Cosine, 16, 200, 50);
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
    let db = create_db_with_hnsw(dim, DistanceMetric::Cosine, 16, 200, 50);
    let mut rng = StdRng::seed_from_u64(42);

    for i in 0..size {
        let vector = generate_random_vector(dim, &mut rng);
        db.insert(i as u64, &vector, None).unwrap();
    }

    for batch_size in [1, 10, 50, 100].iter() {
        let queries: Vec<Vec<f32>> = (0..*batch_size)
            .map(|_| generate_random_vector(dim, &mut rng))
            .collect();

        group.bench_with_input(BenchmarkId::new("batch", batch_size), batch_size, |b, _| {
            b.iter(|| {
                db.parallel_batch_search(black_box(&queries), 10);
            });
        });
    }

    group.finish();
}

fn bench_hnsw_vs_bruteforce(c: &mut Criterion) {
    let mut group = c.benchmark_group("hnsw_vs_bruteforce");

    let dim = 128;
    let size = 5_000;

    let mut rng = StdRng::seed_from_u64(42);
    let mut vectors = Vec::new();

    for i in 0..size {
        vectors.push(generate_random_vector(dim, &mut rng));
    }

    let hnsw_db = create_db_with_hnsw(dim, DistanceMetric::Cosine, 16, 200, 50);
    let brute_db = VectorDB::new(dim, DistanceMetric::Cosine);

    for (i, vec) in vectors.iter().enumerate() {
        hnsw_db.insert(i as u64, vec, None).unwrap();
        brute_db.insert(i as u64, vec, None).unwrap();
    }

    let query = generate_random_vector(dim, &mut rng);

    group.bench_function("hnsw", |b| {
        b.iter(|| {
            hnsw_db.search(black_box(&query), 10).unwrap();
        });
    });

    group.bench_function("bruteforce", |b| {
        b.iter(|| {
            brute_db.search(black_box(&query), 10).unwrap();
        });
    });

    group.finish();
}

fn bench_concurrent_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("concurrent");

    let size = 10_000;
    let dim = 128;

    group.bench_function("mixed_read_write", |b| {
        b.iter(|| {
            use std::sync::Arc;
            use std::thread;

            let db = Arc::new(create_db_with_hnsw(dim, DistanceMetric::Cosine, 16, 200, 50));
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

fn bench_hnsw_params(c: &mut Criterion) {
    let mut group = c.benchmark_group("hnsw_params");

    let size = 5_000;
    let dim = 128;

    let params = vec![
        (8, 100, 30),
        (16, 200, 50),
        (32, 400, 100),
    ];

    for (m, ef_construction, ef_search) in params {
        let db = create_db_with_hnsw(dim, DistanceMetric::Cosine, m, ef_construction, ef_search);
        let mut rng = StdRng::seed_from_u64(42);

        for i in 0..size {
            let vector = generate_random_vector(dim, &mut rng);
            db.insert(i as u64, &vector, None).unwrap();
        }

        let query = generate_random_vector(dim, &mut rng);

        group.bench_with_input(
            BenchmarkId::new("params", format!("M{}_ef{}_efs{}", m, ef_construction, ef_search)),
            &(m, ef_construction, ef_search),
            |b, _| {
                b.iter(|| {
                    db.search(black_box(&query), 10).unwrap();
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_insert,
    bench_insert_hnsw,
    bench_search,
    bench_search_hnsw,
    bench_search_k_values,
    bench_distance_metrics,
    bench_dimensions,
    bench_batch_search,
    bench_hnsw_vs_bruteforce,
    bench_concurrent_operations,
    bench_hnsw_params,
);
criterion_main!(benches);
