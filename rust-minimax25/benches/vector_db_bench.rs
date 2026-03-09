use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use rand::Rng;
use rust_minimax25::VectorDB;

fn generate_random_vectors(dim: usize, count: usize) -> Vec<Vec<f32>> {
    let mut rng = rand::thread_rng();
    (0..count)
        .map(|_| (0..dim).map(|_| rng.gen::<f32>()).collect())
        .collect()
}

fn bench_insert(c: &mut Criterion) {
    let mut group = c.benchmark_group("insert");
    
    let sizes = vec![100, 1000, 10000];
    for size in sizes.iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter(|| {
                let db = VectorDB::new(128);
                let vectors = generate_random_vectors(128, size);
                for (i, vec) in vectors.into_iter().enumerate() {
                    db.insert(i as u64, vec, None).unwrap();
                }
            });
        });
    }
    
    group.finish();
}

fn bench_search(c: &mut Criterion) {
    let mut group = c.benchmark_group("search");
    
    let sizes = vec![100, 1000, 10000];
    for size in sizes.iter() {
        let db = VectorDB::new(128);
        let vectors = generate_random_vectors(128, *size);
        for (i, vec) in vectors.into_iter().enumerate() {
            db.insert(i as u64, vec, None).unwrap();
        }
        
        let query = generate_random_vectors(128, 1);
        
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| {
                db.search(black_box(&query[0]), black_box(10));
            });
        });
    }
    
    group.finish();
}

fn bench_par_search(c: &mut Criterion) {
    let mut group = c.benchmark_group("par_search");
    
    let sizes = vec![100, 1000, 10000];
    for size in sizes.iter() {
        let db = VectorDB::new(128);
        let vectors = generate_random_vectors(128, *size);
        for (i, vec) in vectors.into_iter().enumerate() {
            db.insert(i as u64, vec, None).unwrap();
        }
        
        let query = generate_random_vectors(128, 1);
        
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| {
                db.par_search(black_box(&query[0]), black_box(10));
            });
        });
    }
    
    group.finish();
}

fn bench_ivf_search(c: &mut Criterion) {
    let mut group = c.benchmark_group("ivf_search");
    
    let sizes = vec![1000, 10000];
    for size in sizes.iter() {
        let mut db = VectorDB::new(128);
        let vectors = generate_random_vectors(128, *size);
        for (i, vec) in vectors.into_iter().enumerate() {
            db.insert(i as u64, vec, None).unwrap();
        }
        
        db.build_ivf_index(32).unwrap();
        
        let query = generate_random_vectors(128, 1);
        
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| {
                db.search_ivf(black_box(&query[0]), black_box(10), black_box(8));
            });
        });
    }
    
    group.finish();
}

fn bench_distance_metrics(c: &mut Criterion) {
    let mut group = c.benchmark_group("distance_metrics");
    
    let dim = 128;
    let a = generate_random_vectors(dim, 1);
    let b = generate_random_vectors(dim, 1);
    let a_slice = &a[0];
    let b_slice = &b[0];
    
    group.bench_function("cosine", |b| {
        b.iter(|| {
            rust_minimax25::distance::cosine_distance(black_box(a_slice), black_box(b_slice));
        });
    });
    
    group.bench_function("euclidean", |b| {
        b.iter(|| {
            rust_minimax25::distance::euclidean_distance(black_box(a_slice), black_box(b_slice));
        });
    });
    
    group.bench_function("dot_product", |b| {
        b.iter(|| {
            rust_minimax25::distance::dot_product(black_box(a_slice), black_box(b_slice));
        });
    });
    
    group.finish();
}

fn bench_batch_search(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_search");
    
    let sizes = vec![100, 1000, 10000];
    for size in sizes.iter() {
        let db = VectorDB::new(128);
        let vectors = generate_random_vectors(128, *size);
        for (i, vec) in vectors.into_iter().enumerate() {
            db.insert(i as u64, vec, None).unwrap();
        }
        
        let queries = generate_random_vectors(128, 10);
        
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| {
                db.batch_search(black_box(&queries), black_box(10));
            });
        });
    }
    
    group.finish();
}

fn bench_different_dimensions(c: &mut Criterion) {
    let mut group = c.benchmark_group("dimensions");
    
    let dims = vec![64, 128, 256, 512];
    for dim in dims.iter() {
        let db = VectorDB::new(*dim as u32);
        let vectors = generate_random_vectors(*dim, 1000);
        for (i, vec) in vectors.into_iter().enumerate() {
            db.insert(i as u64, vec, None).unwrap();
        }
        
        let query = generate_random_vectors(*dim, 1);
        
        group.bench_with_input(BenchmarkId::from_parameter(dim), dim, |b, _| {
            b.iter(|| {
                db.search(black_box(&query[0]), black_box(10));
            });
        });
    }
    
    group.finish();
}

criterion_group!(
    benches,
    bench_insert,
    bench_search,
    bench_par_search,
    bench_ivf_search,
    bench_distance_metrics,
    bench_batch_search,
    bench_different_dimensions
);
criterion_main!(benches);
