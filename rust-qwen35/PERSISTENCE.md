# 向量数据库持久化功能

## 功能概述

Rust Qwen35 向量数据库支持完整的持久化功能，可以将数据库状态保存到磁盘，并在需要时重新加载。

## 持久化特性

✅ **完整数据保存**
- 向量数据（ID 和向量值）
- 元数据（metadata）
- 数据库配置（维度、距离度量）
- ID 映射索引

✅ **高性能**
- 保存 100 条向量（128 维）：~1.37ms
- 加载 100 条向量（128 维）：~0.44ms
- 二进制格式存储，高效紧凑

✅ **数据完整性**
- 保存前后数据完全一致
- 搜索结果完全匹配
- 支持所有距离度量类型

✅ **灵活使用**
- 加载后可继续插入新数据
- 支持多次保存和加载
- 跨会话数据持久化

## 使用方法

### 保存数据库

```rust
use rust_qwen35::{create_db, DistanceMetric};

// 创建数据库并插入数据
let db = create_db(128, DistanceMetric::Cosine);

for i in 0..1000 {
    let vector = generate_vector(i);
    db.insert(i as i64, vector, None).unwrap();
}

// 保存到文件
db.save("/path/to/vector_db.bin").unwrap();
println!("数据库已保存！");
```

### 加载数据库

```rust
use rust_qwen35::VectorDB;

// 从文件加载
let loaded_db = VectorDB::load("/path/to/vector_db.bin").unwrap();

println!("加载了 {} 条向量", loaded_db.size());
println!("维度：{}", loaded_db.dimension());
println!("距离度量：{:?}", loaded_db.metric());

// 可以立即使用加载的数据库
let query = vec![0.1, 0.2, ..., 0.9];
let results = loaded_db.search(&query, 10).unwrap();
```

### 完整示例

```rust
use rust_qwen35::{create_db, DistanceMetric, VectorDB};

fn main() {
    // 1. 创建数据库
    let db = create_db(128, DistanceMetric::Cosine);
    
    // 2. 插入数据
    for i in 0..100 {
        let vector: Vec<f32> = (0..128)
            .map(|j| ((i * 128 + j) as f32) / 1000.0)
            .collect();
        let metadata = Some(format!("user_{}", i).into_bytes());
        db.insert(i as i64, vector, metadata).unwrap();
    }
    
    // 3. 保存
    db.save("my_vectors.bin").unwrap();
    
    // ... 程序结束 ...
    
    // 4. 重新加载
    let loaded_db = VectorDB::load("my_vectors.bin").unwrap();
    
    // 5. 继续使用
    let query = vec![0.1; 128];
    let results = loaded_db.search(&query, 5).unwrap();
    
    for result in results {
        println!("找到向量 ID: {}, 距离：{}", result.id, result.distance);
    }
}
```

## 文件格式

持久化文件采用二进制格式，结构如下：

```
+------------------+
| 元数据长度 (4B)   |
+------------------+
| 元数据 (JSON)    |
| - dimension      |
| - metric         |
| - size           |
+------------------+
| 向量数据         |
| - ID (8B)        |
| - 维度长度 (4B)   |
| - 向量值 (N*4B)  |
| - 元数据长度 (4B) |
| - 元数据 (可变)   |
+------------------+
| ID 映射表        |
| - ID (8B)        |
| - 索引 (4B)      |
+------------------+
```

## 性能测试

### 测试环境
- CPU: Apple Silicon
- 数据量：100 条向量
- 维度：128

### 测试结果

| 操作 | 耗时 |
|------|------|
| 保存 | 1.37ms |
| 加载 | 0.44ms |
| 文件大小 | 53.85 KB |

### 不同数据量性能

| 向量数 | 维度 | 保存耗时 (ms) | 加载耗时 (ms) | 文件大小 (KB) |
|--------|------|--------------|--------------|---------------|
| 100    | 128  | 1.37         | 0.44         | 53.85         |
| 1,000  | 128  | ~13          | ~4           | ~538          |
| 10,000 | 128  | ~130         | ~40          | ~5,380        |
| 100    | 512  | ~5.5         | ~1.8         | ~215          |

*注：大尺寸数据性能呈线性增长*

## 支持的元数据类型

```rust
// 文本元数据
let metadata = Some("用户名称".as_bytes().to_vec());

// JSON 元数据
let json_meta = serde_json::json!({
    "name": "张三",
    "age": 25,
    "tags": ["vip", "active"]
});
let metadata = Some(json_meta.to_string().into_bytes());

// 二进制元数据
let metadata = Some(vec![0x01, 0x02, 0x03, 0x04]);

// 无元数据
let metadata = None;
```

## 错误处理

```rust
use rust_qwen35::{VectorDB, Error};

match VectorDB::load("database.bin") {
    Ok(db) => {
        println!("成功加载数据库");
    }
    Err(Error::IoError(e)) => {
        eprintln!("文件 IO 错误：{}", e);
    }
    Err(Error::SerializationError(msg)) => {
        eprintln!("序列化错误：{}", msg);
    }
    Err(Error::InvalidDistanceMetric) => {
        eprintln!("无效的距离度量");
    }
    Err(e) => {
        eprintln!("其他错误：{}", e);
    }
}
```

## 最佳实践

### 1. 定期保存

```rust
// 每插入 1000 条向量保存一次
for i in 0..10000 {
    db.insert(i as i64, vector, None).unwrap();
    
    if i % 1000 == 0 {
        db.save("checkpoint.bin").unwrap();
    }
}
```

### 2. 使用检查点

```rust
// 启动时尝试加载检查点
let db = if std::path::Path::new("checkpoint.bin").exists() {
    VectorDB::load("checkpoint.bin").unwrap()
} else {
    create_db(512, DistanceMetric::Cosine)
};
```

### 3. 原子保存

```rust
// 使用临时文件确保原子性
fn safe_save(db: &VectorDB, path: &str) -> Result<(), Error> {
    let temp_path = format!("{}.tmp", path);
    db.save(&temp_path)?;
    std::fs::rename(&temp_path, path)?;
    Ok(())
}
```

### 4. 版本管理

```rust
// 保存多个版本
let timestamp = std::time::SystemTime::now()
    .duration_since(std::time::UNIX_EPOCH)
    .unwrap()
    .as_secs();
    
let path = format!("db_backup_{}.bin", timestamp);
db.save(&path).unwrap();
```

## 运行持久化测试

```bash
# 运行完整的持久化功能测试
cargo run --release --example persistence_test

# 运行性能测试
cargo run --release --example perf_test
```

## 与其他功能配合使用

### 批量搜索

```rust
let loaded_db = VectorDB::load("database.bin").unwrap();

let queries = vec![
    vec![0.1; 128],
    vec![0.2; 128],
    vec![0.3; 128],
];

let results = loaded_db.search_batch(&queries, 10).unwrap();
```

### 统计信息

```rust
let loaded_db = VectorDB::load("database.bin").unwrap();
loaded_db.enable_stats(true);

// 使用数据库...

loaded_db.print_stats();
```

## 注意事项

1. **文件路径**：确保保存路径有写权限
2. **磁盘空间**：大数据库需要足够磁盘空间
3. **版本兼容**：不同版本的数据库格式可能不兼容
4. **并发安全**：避免同时读写同一文件
5. **错误恢复**：加载失败时准备好回退方案

## 技术实现

### 序列化

- 元数据：使用 Serde JSON 序列化
- 向量数据：二进制格式（小端序）
- ID 映射：保持 O(1) 查找性能

### 压缩优化（未来）

计划支持的压缩算法：
- LZ4：快速压缩/解压
- Zstandard：高压缩比
- Snappy：平衡性能

### 内存映射（未来）

对于超大数据库，计划支持：
- 内存映射文件（mmap）
- 延迟加载
- 分页缓存

## 总结

Rust Qwen35 向量数据库的持久化功能提供了：

- ✅ 完整的数据持久化
- ✅ 高性能的保存/加载
- ✅ 数据完整性保证
- ✅ 灵活的使用方式
- ✅ 完善的错误处理
- ✅ 支持元数据
- ✅ 所有距离度量兼容

通过持久化功能，您可以轻松实现数据的长期存储、跨会话使用和备份恢复。
