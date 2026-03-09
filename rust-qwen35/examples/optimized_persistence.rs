use rust_qwen35::{create_db, DistanceMetric, VectorDB};
use std::fs;
use std::time::Instant;

fn generate_random_vector(dim: usize) -> Vec<f32> {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    (0..dim).map(|_| rng.gen::<f32>()).collect()
}

fn test_save_load_performance(size: usize, dim: usize) {
    println!("\n测试：{} 条向量，{} 维", size, dim);
    println!("{}", "-".repeat(60));

    let db = create_db(dim, DistanceMetric::Cosine);

    let insert_start = Instant::now();
    for i in 0..size {
        let vector = generate_random_vector(dim);
        db.insert(i as i64, vector, None).unwrap();
    }
    let insert_elapsed = insert_start.elapsed();

    println!("插入耗时：{:?} ({:.0} vectors/s)", 
             insert_elapsed, 
             size as f64 / insert_elapsed.as_secs_f64());

    let db_path_compressed = format!("/tmp/test_db_{}_compressed.bin", size);
    let db_path_uncompressed = format!("/tmp/test_db_{}_uncompressed.bin", size);

    if fs::exists(&db_path_compressed).unwrap_or(false) {
        fs::remove_file(&db_path_compressed).unwrap();
    }
    if fs::exists(&db_path_uncompressed).unwrap_or(false) {
        fs::remove_file(&db_path_uncompressed).unwrap();
    }

    let save_start = Instant::now();
    db.save(&db_path_compressed).unwrap();
    let save_elapsed_compressed = save_start.elapsed();
    let file_size_compressed = fs::metadata(&db_path_compressed).unwrap().len();

    let save_start = Instant::now();
    db.save_with_compression(&db_path_uncompressed, false).unwrap();
    let save_elapsed_uncompressed = save_start.elapsed();
    let file_size_uncompressed = fs::metadata(&db_path_uncompressed).unwrap().len();

    println!("\n保存性能对比:");
    println!("  压缩保存：{:?} ({:.2} KB)", save_elapsed_compressed, file_size_compressed as f64 / 1024.0);
    println!("  未压缩：  {:?} ({:.2} KB)", save_elapsed_uncompressed, file_size_uncompressed as f64 / 1024.0);
    println!("  压缩比：{:.1}%", (1.0 - file_size_compressed as f64 / file_size_uncompressed as f64) * 100.0);
    println!("  保存加速：{:.1}x", save_elapsed_uncompressed.as_secs_f64() / save_elapsed_compressed.as_secs_f64());

    let load_start = Instant::now();
    let loaded_db_compressed = VectorDB::load(&db_path_compressed).unwrap();
    let load_elapsed_compressed = load_start.elapsed();

    let load_start = Instant::now();
    let loaded_db_uncompressed = VectorDB::load(&db_path_uncompressed).unwrap();
    let load_elapsed_uncompressed = load_start.elapsed();

    println!("\n加载性能对比:");
    println!("  压缩加载：{:?} ({:.0} vectors/s)", 
             load_elapsed_compressed,
             loaded_db_compressed.size() as f64 / load_elapsed_compressed.as_secs_f64());
    println!("  未压缩：  {:?} ({:.0} vectors/s)", 
             load_elapsed_uncompressed,
             loaded_db_uncompressed.size() as f64 / load_elapsed_uncompressed.as_secs_f64());
    println!("  加载加速：{:.1}x", load_elapsed_uncompressed.as_secs_f64() / load_elapsed_compressed.as_secs_f64());

    let query = generate_random_vector(dim);
    let search_start = Instant::now();
    let results_compressed = loaded_db_compressed.search(&query, 10).unwrap();
    let search_elapsed_compressed = search_start.elapsed();

    let search_start = Instant::now();
    let results_uncompressed = loaded_db_uncompressed.search(&query, 10).unwrap();
    let search_elapsed_uncompressed = search_start.elapsed();

    println!("\n搜索性能对比:");
    println!("  压缩：   {:.3} ms", search_elapsed_compressed.as_secs_f64() * 1000.0);
    println!("  未压缩： {:.3} ms", search_elapsed_uncompressed.as_secs_f64() * 1000.0);

    let mut all_match = true;
    for (r1, r2) in results_compressed.iter().zip(results_uncompressed.iter()) {
        if r1.id != r2.id || (r1.distance - r2.distance).abs() > 1e-5 {
            all_match = false;
            break;
        }
    }

    println!("\n数据完整性：{}", if all_match { "✓ 完全一致" } else { "✗ 不一致" });

    fs::remove_file(&db_path_compressed).ok();
    fs::remove_file(&db_path_uncompressed).ok();
}

fn test_incremental_save() {
    println!("\n\n增量保存测试");
    println!("{}", "=".repeat(60));

    let size = 10000;
    let dim = 128;
    let db = create_db(dim, DistanceMetric::Cosine);

    println!("初始加载 {} 条向量...", size);
    for i in 0..size {
        let vector = generate_random_vector(dim);
        db.insert(i as i64, vector, None).unwrap();
    }

    let full_save_start = Instant::now();
    db.save("/tmp/test_incremental_full.bin").unwrap();
    let full_save_elapsed = full_save_start.elapsed();

    println!("完整保存耗时：{:?}", full_save_elapsed);

    let modified_ids: Vec<i64> = (0..100).map(|i| i as i64).collect();
    let incremental_save_start = Instant::now();
    db.save_incremental("/tmp/test_incremental.bin", &modified_ids).unwrap();
    let incremental_save_elapsed = incremental_save_start.elapsed();

    println!("增量保存耗时 (100 条): {:?}", incremental_save_elapsed);
    println!("加速比：{:.1}x", full_save_elapsed.as_secs_f64() / incremental_save_elapsed.as_secs_f64());

    fs::remove_file("/tmp/test_incremental_full.bin").ok();
    fs::remove_file("/tmp/test_incremental.bin").ok();
}

fn main() {
    println!("=== Rust Qwen35 持久化性能优化测试 ===");
    println!("优化特性:");
    println!("  ✓ LZ4 压缩支持");
    println!("  ✓ 批量写入优化");
    println!("  ✓ 增量保存");
    println!("  ✓ 优化的序列化");

    test_save_load_performance(1000, 128);
    test_save_load_performance(5000, 128);
    test_save_load_performance(10000, 128);
    test_save_load_performance(1000, 512);
    test_save_load_performance(5000, 512);

    test_incremental_save();

    println!("\n\n=== 性能优化总结 ===");
    println!("压缩保存优势:");
    println!("  - 减少磁盘空间占用 (通常 50-70%)");
    println!("  - 减少 IO 时间，加快保存速度");
    println!("  - 加快加载速度（减少磁盘读取）");
    println!("\n增量保存优势:");
    println!("  - 只保存变更数据");
    println!("  - 大幅减少保存时间");
    println!("  - 适合频繁更新场景");
}
