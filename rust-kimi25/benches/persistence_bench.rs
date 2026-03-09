use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;
use tempfile::TempDir;

use rust_kimi25::{
    VectorDB, DistanceMetric,
    Persistence, PersistenceConfig, CompressionType
};

fn generate_random_vector(dim: usize, rng: &mut StdRng) -> Vec<f32> {
    (0..dim).map(|_| rng.gen::<f32>()).collect()
}

fn bench_save(c: &mut Criterion) {
    let mut group = c.benchmark_group("persistence_save");

    for size in [1_000, 10_000, 50_000].iter() {
        let dim = 128;

        group.throughput(Throughput::Elements(*size as u64));

        // Benchmark uncompressed save
        group.bench_with_input(BenchmarkId::new("uncompressed", size), size, |b, size| {
            let temp_dir = TempDir::new().unwrap();
            let file_path = temp_dir.path().join("test.db");

            let db = VectorDB::new(dim, DistanceMetric::Cosine);
            let mut rng = StdRng::seed_from_u64(42);
            for i in 0..*size {
                let vector = generate_random_vector(dim, &mut rng);
                db.insert(i as u64, &vector, None).unwrap();
            }

            let config = PersistenceConfig {
                compression: CompressionType::None,
                use_mmap: true,
                verify_checksum: false,
            };

            b.iter(|| {
                Persistence::save(black_box(&db), &file_path, &config).unwrap();
            });
        });

        // Benchmark LZ4 compressed save
        group.bench_with_input(BenchmarkId::new("lz4", size), size, |b, size| {
            let temp_dir = TempDir::new().unwrap();
            let file_path = temp_dir.path().join("test.db");

            let db = VectorDB::new(dim, DistanceMetric::Cosine);
            let mut rng = StdRng::seed_from_u64(42);
            for i in 0..*size {
                let vector = generate_random_vector(dim, &mut rng);
                db.insert(i as u64, &vector, None).unwrap();
            }

            let config = PersistenceConfig {
                compression: CompressionType::Lz4,
                use_mmap: true,
                verify_checksum: false,
            };

            b.iter(|| {
                Persistence::save(black_box(&db), &file_path, &config).unwrap();
            });
        });
    }

    group.finish();
}

fn bench_load(c: &mut Criterion) {
    let mut group = c.benchmark_group("persistence_load");

    for size in [1_000, 10_000, 50_000].iter() {
        let dim = 128;

        group.throughput(Throughput::Elements(*size as u64));

        // Benchmark buffered load
        group.bench_with_input(BenchmarkId::new("buffered", size), size, |b, size| {
            let temp_dir = TempDir::new().unwrap();
            let file_path = temp_dir.path().join("test.db");

            // Create and save database
            let db = VectorDB::new(dim, DistanceMetric::Cosine);
            let mut rng = StdRng::seed_from_u64(42);
            for i in 0..*size {
                let vector = generate_random_vector(dim, &mut rng);
                db.insert(i as u64, &vector, None).unwrap();
            }

            let save_config = PersistenceConfig {
                compression: CompressionType::Lz4,
                use_mmap: false,
                verify_checksum: false,
            };
            Persistence::save(&db, &file_path, &save_config).unwrap();

            let load_config = PersistenceConfig {
                compression: CompressionType::Lz4,
                use_mmap: false,
                verify_checksum: false,
            };

            b.iter(|| {
                Persistence::load(black_box(&file_path), &load_config).unwrap();
            });
        });

        // Benchmark mmap load
        group.bench_with_input(BenchmarkId::new("mmap", size), size, |b, size| {
            let temp_dir = TempDir::new().unwrap();
            let file_path = temp_dir.path().join("test.db");

            // Create and save database
            let db = VectorDB::new(dim, DistanceMetric::Cosine);
            let mut rng = StdRng::seed_from_u64(42);
            for i in 0..*size {
                let vector = generate_random_vector(dim, &mut rng);
                db.insert(i as u64, &vector, None).unwrap();
            }

            let save_config = PersistenceConfig {
                compression: CompressionType::Lz4,
                use_mmap: true,
                verify_checksum: false,
            };
            Persistence::save(&db, &file_path, &save_config).unwrap();

            let load_config = PersistenceConfig {
                compression: CompressionType::Lz4,
                use_mmap: true,
                verify_checksum: false,
            };

            b.iter(|| {
                Persistence::load(black_box(&file_path), &load_config).unwrap();
            });
        });
    }

    group.finish();
}

fn bench_roundtrip(c: &mut Criterion) {
    let mut group = c.benchmark_group("persistence_roundtrip");

    for size in [1_000, 10_000].iter() {
        let dim = 128;

        group.throughput(Throughput::Elements(*size as u64));

        group.bench_with_input(BenchmarkId::new("lz4", size), size, |b, size| {
            let temp_dir = TempDir::new().unwrap();
            let file_path = temp_dir.path().join("test.db");

            let db = VectorDB::new(dim, DistanceMetric::Cosine);
            let mut rng = StdRng::seed_from_u64(42);
            for i in 0..*size {
                let vector = generate_random_vector(dim, &mut rng);
                db.insert(i as u64, &vector, None).unwrap();
            }

            let config = PersistenceConfig {
                compression: CompressionType::Lz4,
                use_mmap: true,
                verify_checksum: false,
            };

            b.iter(|| {
                Persistence::save(black_box(&db), &file_path, &config).unwrap();
                Persistence::load(black_box(&file_path), &config).unwrap();
            });
        });
    }

    group.finish();
}

fn bench_compression_ratio(c: &mut Criterion) {
    let mut group = c.benchmark_group("compression_ratio");

    for size in [1_000, 10_000, 50_000].iter() {
        let dim = 128;

        group.bench_with_input(BenchmarkId::new("measure", size), size, |b, size| {
            let temp_dir = TempDir::new().unwrap();
            let uncompressed_path = temp_dir.path().join("uncompressed.db");
            let lz4_path = temp_dir.path().join("lz4.db");

            let db = VectorDB::new(dim, DistanceMetric::Cosine);
            let mut rng = StdRng::seed_from_u64(42);
            for i in 0..*size {
                let vector = generate_random_vector(dim, &mut rng);
                db.insert(i as u64, &vector, Some(vec![0u8; 50])).unwrap();
            }

            b.iter(|| {
                // Save uncompressed
                let no_compression = PersistenceConfig {
                    compression: CompressionType::None,
                    use_mmap: true,
                    verify_checksum: false,
                };
                Persistence::save(&db, &uncompressed_path, &no_compression).unwrap();
                let uncompressed_size = std::fs::metadata(&uncompressed_path).unwrap().len();

                // Save with LZ4
                let lz4_config = PersistenceConfig {
                    compression: CompressionType::Lz4,
                    use_mmap: true,
                    verify_checksum: false,
                };
                Persistence::save(&db, &lz4_path, &lz4_config).unwrap();
                let lz4_size = std::fs::metadata(&lz4_path).unwrap().len();

                let ratio = 1.0 - (lz4_size as f64 / uncompressed_size as f64);
                black_box(ratio);
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_save,
    bench_load,
    bench_roundtrip,
    bench_compression_ratio,
);
criterion_main!(benches);
