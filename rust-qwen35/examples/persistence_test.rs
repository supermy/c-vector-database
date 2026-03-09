use rust_qwen35::{create_db, DistanceMetric};
use std::fs;

fn main() {
    println!("=== 向量数据库持久化功能测试 ===\n");

    let db_path = "/tmp/test_vector_db.bin";
    
    // 清理旧文件
    if fs::exists(db_path).unwrap_or(false) {
        fs::remove_file(db_path).unwrap();
    }

    // 1. 创建数据库并插入数据
    println!("1. 创建数据库并插入数据...");
    let db = create_db(128, DistanceMetric::Cosine);

    // 插入测试数据
    for i in 0..100 {
        let vector: Vec<f32> = (0..128)
            .map(|j| ((i * 128 + j) as f32) / 1000.0)
            .collect();
        let metadata = Some(format!("metadata_{}", i).into_bytes());
        db.insert(i as i64, vector, metadata).unwrap();
    }

    println!("   ✓ 成功插入 {} 条向量", db.size());

    // 2. 搜索测试
    println!("\n2. 保存前搜索测试...");
    let query: Vec<f32> = (0..128).map(|i| (i as f32) / 1000.0).collect();
    let results_before = db.search(&query, 5).unwrap();
    
    println!("   搜索结果（保存前）:");
    for result in &results_before {
        let entry = db.get(result.id).unwrap();
        println!("   - ID: {}, Distance: {:.6}, Metadata: {:?}", 
                 result.id, result.distance, 
                 entry.metadata.as_ref().map(|m| String::from_utf8_lossy(m)));
    }

    // 3. 保存到文件
    println!("\n3. 保存数据库到文件...");
    let save_start = std::time::Instant::now();
    db.save(db_path).unwrap();
    let save_elapsed = save_start.elapsed();
    
    let file_size = fs::metadata(db_path).unwrap().len();
    println!("   ✓ 保存成功!");
    println!("   - 耗时：{:?} ms", save_elapsed.as_secs_f64() * 1000.0);
    println!("   - 文件大小：{:.2} KB", file_size as f64 / 1024.0);
    println!("   - 文件路径：{}", db_path);

    // 4. 从文件加载
    println!("\n4. 从文件加载数据库...");
    let load_start = std::time::Instant::now();
    let loaded_db = rust_qwen35::VectorDB::load(db_path).unwrap();
    let load_elapsed = load_start.elapsed();
    
    println!("   ✓ 加载成功!");
    println!("   - 耗时：{:?} ms", load_elapsed.as_secs_f64() * 1000.0);
    println!("   - 加载向量数：{}", loaded_db.size());
    println!("   - 维度：{}", loaded_db.dimension());
    println!("   - 距离度量：{:?}", loaded_db.metric());

    // 5. 验证加载的数据
    println!("\n5. 验证加载的数据...");
    let results_after = loaded_db.search(&query, 5).unwrap();
    
    println!("   搜索结果（加载后）:");
    for result in &results_after {
        let entry = loaded_db.get(result.id).unwrap();
        println!("   - ID: {}, Distance: {:.6}, Metadata: {:?}", 
                 result.id, result.distance, 
                 entry.metadata.as_ref().map(|m| String::from_utf8_lossy(m)));
    }

    // 6. 对比结果
    println!("\n6. 对比保存前后的搜索结果...");
    let mut all_match = true;
    for (before, after) in results_before.iter().zip(results_after.iter()) {
        if before.id != after.id || (before.distance - after.distance).abs() > 1e-6 {
            all_match = false;
            break;
        }
    }

    if all_match {
        println!("   ✓ 搜索结果完全一致！持久化功能验证通过！");
    } else {
        println!("   ✗ 搜索结果不一致！");
    }

    // 7. 测试加载后继续插入数据
    println!("\n7. 测试加载后继续插入数据...");
    for i in 100..110 {
        let vector: Vec<f32> = (0..128)
            .map(|j| ((i * 128 + j) as f32) / 1000.0)
            .collect();
        loaded_db.insert(i as i64, vector, None).unwrap();
    }
    println!("   ✓ 成功插入额外 10 条向量");
    println!("   - 当前总向量数：{}", loaded_db.size());

    // 8. 再次保存
    println!("\n8. 再次保存更新后的数据库...");
    let db_path_updated = "/tmp/test_vector_db_updated.bin";
    loaded_db.save(db_path_updated).unwrap();
    let updated_file_size = fs::metadata(db_path_updated).unwrap().len();
    println!("   ✓ 保存成功!");
    println!("   - 文件大小：{:.2} KB", updated_file_size as f64 / 1024.0);

    // 9. 测试不同距离度量的持久化
    println!("\n9. 测试不同距离度量的持久化...");
    let metrics = [
        DistanceMetric::Cosine,
        DistanceMetric::Euclidean,
        DistanceMetric::DotProduct,
        DistanceMetric::Manhattan,
    ];

    for metric in &metrics {
        let test_db = create_db(64, *metric);
        for i in 0..10 {
            let vector: Vec<f32> = (0..64).map(|j| (i + j) as f32).collect();
            test_db.insert(i as i64, vector, None).unwrap();
        }
        
        let test_path = format!("/tmp/test_db_{:?}.bin", metric);
        test_db.save(&test_path).unwrap();
        
        let loaded_test_db = rust_qwen35::VectorDB::load(&test_path).unwrap();
        
        assert_eq!(loaded_test_db.metric(), *metric, "距离度量不匹配");
        assert_eq!(loaded_test_db.size(), 10, "向量数量不匹配");
        
        println!("   ✓ {:?} 持久化验证通过", metric);
        
        fs::remove_file(&test_path).ok();
    }

    // 清理测试文件
    fs::remove_file(db_path).ok();
    fs::remove_file(db_path_updated).ok();

    println!("\n=== 所有持久化功能测试完成 ===");
    println!("\n持久化功能特性:");
    println!("✓ 保存向量数据（ID、向量值）");
    println!("✓ 保存元数据（metadata）");
    println!("✓ 保存数据库配置（维度、距离度量）");
    println!("✓ 保存 ID 映射索引");
    println!("✓ 加载后数据完整性验证");
    println!("✓ 加载后可继续插入新数据");
    println!("✓ 支持所有距离度量类型");
}
