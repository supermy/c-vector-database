use rust_qwen35::{create_db, create_db_with_capacity, DistanceMetric};
use std::time::Instant;

fn generate_random_vector(dim: usize) -> Vec<f32> {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    (0..dim).map(|_| rng.gen::<f32>()).collect()
}

fn main() {
    println!("=== Rust Qwen35 Vector Database Performance Test ===\n");

    let dimensions = [128, 512, 768];
    let sizes = [1000, 10000, 50000];

    for dim in &dimensions {
        println!("Dimension: {}", dim);
        println!("{}", "-".repeat(50));

        for &size in &sizes {
            let db = create_db_with_capacity(*dim, DistanceMetric::Cosine, size);

            // 测试插入性能
            let insert_start = Instant::now();
            for i in 0..size {
                let vector = generate_random_vector(*dim);
                db.insert(i as i64, vector, None).unwrap();
            }
            let insert_elapsed = insert_start.elapsed();
            let insert_per_sec = size as f64 / insert_elapsed.as_secs_f64();

            println!(
                "  Size: {:6} | Insert: {:8.2} vec/s ({:.2} ms total)",
                size,
                insert_per_sec,
                insert_elapsed.as_secs_f64() * 1000.0
            );

            // 测试搜索性能
            let query = generate_random_vector(*dim);
            let mut search_times = Vec::new();

            for _ in 0..10 {
                let search_start = Instant::now();
                let _results = db.search(&query, 10).unwrap();
                search_times.push(search_start.elapsed().as_secs_f64());
            }

            let avg_search = search_times.iter().sum::<f64>() / search_times.len() as f64;
            let search_per_sec = 1.0 / avg_search;

            println!(
                "  Size: {:6} | Search: {:8.2} queries/s (avg {:.3} ms)",
                size, search_per_sec, avg_search * 1000.0
            );
        }
        println!();
    }

    // 测试不同距离度量
    println!("Distance Metrics Comparison (dim=512, size=10000)");
    println!("{}", "-".repeat(50));

    let dim = 512;
    let size = 10000;
    let metrics = [
        DistanceMetric::Cosine,
        DistanceMetric::Euclidean,
        DistanceMetric::DotProduct,
        DistanceMetric::Manhattan,
    ];

    for metric in &metrics {
        let db = create_db_with_capacity(dim, *metric, size);

        for i in 0..size {
            let vector = generate_random_vector(dim);
            db.insert(i as i64, vector, None).unwrap();
        }

        let query = generate_random_vector(dim);
        let start = Instant::now();
        for _ in 0..100 {
            let _results = db.search(&query, 10).unwrap();
        }
        let elapsed = start.elapsed().as_secs_f64() / 100.0;

        println!(
            "  {:12} | Avg search: {:.3} ms ({:.2} queries/s)",
            format!("{:?}", metric),
            elapsed * 1000.0,
            1.0 / elapsed
        );
    }

    println!("\n=== Test Complete ===");
}
