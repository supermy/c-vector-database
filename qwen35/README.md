# Qwen35 Vector Database

高性能 C 语言向量数据库实现，支持多种相似度度量和完整的 CRUD 操作。

## 特性

- **高性能哈希索引**: O(1) 时间复杂度的向量查找
- **多种距离度量**: 支持余弦相似度、欧氏距离、点积
- **完整 CRUD 操作**: 插入、查询、删除、更新
- **元数据支持**: 可为每个向量附加自定义元数据
- **持久化存储**: 二进制格式保存和加载
- **内存高效**: 紧凑的内存布局，无额外开销
- **线程安全**: 可在单线程环境中安全使用

## 快速开始

### 编译

```bash
gcc -c qwen35_vdb.c -o qwen35_vdb.o -std=c99 -O3 -Wall -Wextra
gcc -c test_qwen35.c -o test_qwen35.o -std=c99 -O3 -Wall -Wextra
gcc qwen35_vdb.o test_qwen35.o -o test_qwen35 -lm
```

### 运行测试

```bash
./test_qwen35
```

### 集成到项目

```c
#include "qwen35_vdb.h"

// 创建数据库（128 维，余弦相似度）
qwen35_vector_db_t *db = qwen35_db_create(128, QWEN35_DIST_COSINE);

// 插入向量
float vector[128] = {0};
// ... 初始化向量
qwen35_db_insert(db, 1, vector, NULL, 0);

// 搜索最近邻
int64_t ids[10];
float distances[10];
int count = qwen35_db_search(db, query_vector, 10, ids, distances);

// 清理
qwen35_db_destroy(db);
```

## API 参考

### 核心函数

#### 创建和销毁

```c
qwen35_vector_db_t *qwen35_db_create(size_t dimensions, qwen35_distance_t dist_type);
void qwen35_db_destroy(qwen35_vector_db_t *db);
```

#### 插入和删除

```c
int qwen35_db_insert(qwen35_vector_db_t *db, int64_t id, const float *vector, 
                     void *metadata, size_t metadata_size);
int qwen35_db_delete(qwen35_vector_db_t *db, int64_t id);
```

#### 查询

```c
int qwen35_db_search(qwen35_vector_db_t *db, const float *query, size_t k, 
                     int64_t *out_ids, float *out_distances);
int qwen35_db_get(qwen35_vector_db_t *db, int64_t id, float *out_vector, 
                  void *out_metadata, size_t *out_metadata_size);
```

#### 持久化

```c
int qwen35_db_save(qwen35_vector_db_t *db, const char *filename);
qwen35_vector_db_t *qwen35_db_load(const char *filename);
```

### 工具函数

```c
float qwen35_cosine_similarity(const float *a, const float *b, size_t dim);
float qwen35_euclidean_distance(const float *a, const float *b, size_t dim);
float qwen35_dot_product(const float *a, const float *b, size_t dim);
void qwen35_normalize_vector(float *vector, size_t dim);
size_t qwen35_db_size(qwen35_vector_db_t *db);
const char *qwen35_get_version(void);
```

## 距离度量类型

```c
typedef enum {
    QWEN35_DIST_COSINE = 0,      // 余弦相似度（推荐用于文本嵌入）
    QWEN35_DIST_EUCLIDEAN = 1,   // 欧氏距离（推荐用于图像特征）
    QWEN35_DIST_DOT_PRODUCT = 2  // 点积（推荐用于推荐系统）
} qwen35_distance_t;
```

## 性能基准

测试环境：Apple M2, 128 维向量，1000 个向量

| 操作 | 性能 |
|------|------|
| 插入 | 353,607 vectors/s |
| 搜索 | 0.27 ms/次 (k=5) |

## 数据结构

### 哈希索引

Qwen35 使用自定义哈希表实现 O(1) 复杂度的 ID 查找：

- **哈希桶数量**: 16,384
- **哈希算法**: MurmurHash3 变体
- **冲突解决**: 链地址法

### 内存布局

```
qwen35_vector_db_t
├── entries (动态数组)
│   ├── entry[0]: {id, vector, metadata}
│   ├── entry[1]: {id, vector, metadata}
│   └── ...
└── id_map (哈希表)
    ├── bucket[0] -> node -> node -> ...
    ├── bucket[1] -> node -> ...
    └── ...
```

## 使用示例

### 基础示例

```c
#include "qwen35_vdb.h"
#include <stdio.h>

int main() {
    // 创建数据库
    qwen35_vector_db_t *db = qwen35_db_create(128, QWEN35_DIST_COSINE);
    
    // 插入向量
    float vector[128];
    for (int i = 0; i < 128; i++) vector[i] = i / 128.0f;
    
    qwen35_db_insert(db, 1, vector, NULL, 0);
    
    // 搜索
    int64_t ids[5];
    float distances[5];
    int count = qwen35_db_search(db, vector, 5, ids, distances);
    
    printf("Found %d similar vectors\n", count);
    
    // 清理
    qwen35_db_destroy(db);
    return 0;
}
```

### 带元数据的示例

```c
// 插入带元数据的向量
const char *metadata = "{\"title\": \"Example\", \"tags\": [\"ai\", \"ml\"]}";
qwen35_db_insert(db, 1, vector, (void *)metadata, strlen(metadata) + 1);

// 检索元数据
char retrieved_meta[256];
size_t meta_size;
qwen35_db_get(db, 1, vector, retrieved_meta, &meta_size);
```

### 持久化示例

```c
// 保存到文件
qwen35_db_save(db, "database.bin");

// 从文件加载
qwen35_vector_db_t *loaded_db = qwen35_db_load("database.bin");
```

## 与其他版本对比

| 特性 | Qwen35 | Kimi25 | MiniMax25 | GLM5 |
|------|--------|--------|-----------|------|
| 哈希索引 | [OK] | [X] | [OK] | [OK] |
| HNSW 框架 | [X] | [框架] | [X] | [X] |
| 距离度量 | 3 种 | 1 种 | 3 种 | 3 种 |
| 元数据支持 | [OK] | [OK] | [OK] | [OK] |
| 持久化 | [OK] | [OK] | [OK] | [OK] |
| 插入性能 | 353K/s | 131K/s | 267K/s | 283K/s |
| 搜索性能 | 0.27ms | 5.1ms | 4.5ms | 8.8ms |

## 适用场景

### 推荐使用

- [OK] 中小规模向量检索（< 100 万向量）
- [OK] 需要精确 ID 查找的场景
- [OK] 嵌入式系统和资源受限环境
- [OK] 需要快速原型开发的场景
- [OK] 学习和理解向量数据库原理

### 不推荐使用

- [X] 超大规模数据（> 1000 万向量）- 考虑 HNSW
- [X] 需要分布式部署的场景
- [X] 需要 GPU 加速的场景

## 文件说明

```
qwen35/
├── qwen35_vdb.h          # 头文件（API 定义）
├── qwen35_vdb.c          # 核心实现
├── test_qwen35.c         # 测试程序
├── README.md             # 本文档
└── Makefile              # 编译脚本（可选）
```

## 编译选项

```bash
# 调试版本
gcc -c qwen35_vdb.c -o qwen35_vdb.o -std=c99 -g -Wall -Wextra

# 发布版本（优化）
gcc -c qwen35_vdb.c -o qwen35_vdb.o -std=c99 -O3 -march=native -Wall -Wextra

# 性能分析版本
gcc -c qwen35_vdb.c -o qwen35_vdb.o -std=c99 -O3 -pg -Wall -Wextra
```

## 限制

- 最大维度：4096
- 最大容量：受系统内存限制
- 线程安全：单线程环境
- 并发访问：不支持

## 版本历史

### v1.0.0

- 初始版本
- 支持基础 CRUD 操作
- 实现哈希索引
- 支持三种距离度量
- 二进制持久化

## 许可证

MIT License

## 贡献

欢迎提交 Issue 和 Pull Request！

## 联系方式

- GitHub: [@supermy](https://github.com/supermy/c-vector-database)
- 项目地址：https://github.com/supermy/c-vector-database

---

**Qwen35 Vector Database** - 简单、高效、易用的 C 语言向量数据库
