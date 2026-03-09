use rust_minimax25::VectorDB;

fn main() {
    let test_file = "/tmp/test_vdb.json";
    
    println!("=== 创建向量数据库 ===");
    let mut db = VectorDB::new(4);
    
    for i in 0..100 {
        db.insert(i, vec![i as f32, i as f32 * 0.5, i as f32 * 0.25, i as f32 * 0.1], None).unwrap();
    }
    
    println!("插入 100 条向量成功");
    println!("数据库大小: {}", db.len());
    
    println!("\n=== 构建 IVF 索引 ===");
    db.build_ivf_index(10).unwrap();
    println!("IVF 索引构建成功");
    
    println!("\n=== 保存到文件 ===");
    db.save(test_file).unwrap();
    println!("保存成功: {}", test_file);
    
    println!("\n=== 从文件加载 ===");
    let loaded_db = VectorDB::load(test_file).unwrap();
    println!("加载成功!");
    println!("加载后数据库大小: {}", loaded_db.len());
    println!("维度: {}", loaded_db.dimension());
    
    println!("\n=== 验证搜索功能 ===");
    let results = loaded_db.search(&[50.0, 25.0, 12.5, 5.0], 5);
    println!("搜索 top 5 结果:");
    for r in &results {
        println!("  id: {}, distance: {:.4}", r.id, r.distance);
    }
    
    println!("\n=== 测试 IVF 搜索 ===");
    let ivf_results = loaded_db.search_ivf(&[50.0, 25.0, 12.5, 5.0], 5, 3);
    println!("IVF 搜索 top 5 结果:");
    for r in &ivf_results {
        println!("  id: {}, distance: {:.4}", r.id, r.distance);
    }
    
    println!("\n=== 持久化测试完成 ===");
}
