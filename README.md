# 向量数据库 (Vector Database)

[![C/C++ CI](https://github.com/supermy/c-vector-database/actions/workflows/ci.yml/badge.svg)](https://github.com/supermy/c-vector-database/actions/workflows/ci.yml)

C语言实现的高性能向量数据库，支持向量存储、相似度搜索和持久化。

## 项目结构

```
vdb/
├── kimi25/           # kimi25 版本
│   ├── vector_db.h
│   ├── vector_db.c
│   ├── test_vector_db.c
│   └── Makefile
├── minimax25/        # minimax25 版本
│   ├── vdb.h
│   ├── vdb.c
│   └── test_vdb.c
├── glm5/             # glm5 版本
│   ├── glm5_vdb.h
│   ├── glm5_vdb.c
│   └── test_glm5.c
├── qwen35/           # qwen35 版本 (最新)
│   ├── qwen35_vdb.h
│   ├── qwen35_vdb.c
│   └── test_qwen35.c
└── benchmark.c       # 性能对比测试
```

## 版本对比

### 性能指标

| 指标 | kimi25 | minimax25 | glm5 | qwen35 |
|------|--------|-----------|------|--------|
| 插入速度 | 131,503 vec/s | 267,294 vec/s | 280,191 vec/s | **491,159 vec/s** |
| 搜索速度 | 5.1 ms/query | 2ms/query | 9.3ms/query | **0.215 ms/query** |
| ID查找 | O(n) 线性 | O(1) 哈希 | O(1) 哈希 | O(1) 哈希 |
| 哈希桶数 | - | 8192 | 16384 | **16384** |
| SIMD 优化 | ❌ | ❌ | ❌ | ✅ |

### 功能特性

| 功能 | kimi25 | minimax25 | glm5 | qwen35 |
|------|:------:|:---------:|:----:|:------:|
| 向量 CRUD | ✅ | ✅ | ✅ | ✅ |
| Top-K 搜索 | ✅ | ✅ | ✅ | ✅ |
| 余弦相似度 | ✅ | ✅ | ✅ | ✅ |
| 欧氏距离 | ✅ | ✅ | ✅ | ✅ |
| 点积距离 | ❌ | ✅ | ✅ | ✅ |
| 距离度量切换 | ❌ | ✅ | ✅ | ✅ |
| 哈希索引 | ❌ | ✅ | ✅ | ✅ |
| HNSW 框架 | ✅ | ❌ | ❌ | ❌ |
| 持久化 | ✅ | ✅ | ✅ | ✅ |
| 重复 ID 检测 | ❌ | ✅ | ✅ | ✅ |
| 元数据支持 | ✅ | ✅ | ✅ | ✅ |
| **SIMD 优化** | ❌ | ❌ | ❌ | **✅** |
| **批量搜索** | ❌ | ✅ | ✅ | ✅ |

### 适用场景

| 版本 | 适用场景 |
|------|----------|
| **kimi25** | 小数据量、需要扩展 HNSW 索引 |
| **minimax25** | 中等数据量、通用场景、需要快速搜索 |
| **glm5** | 大数据量、内存敏感、需要快速插入 |
| **qwen35** | 高性能需求、大规模数据、需要最快搜索 |

---

## HNSW 索引详解

### 什么是 HNSW？

HNSW (Hierarchical Navigable Small World) 是一种**近似最近邻搜索 (ANN)** 算法，用于在高维向量空间中快速找到最相似的向量。

### 核心问题：暴力搜索的瓶颈

```
假设 100万 个 128维向量：
- 暴力搜索：需要计算 100万次 余弦相似度
- 时间复杂度：O(n)
- 搜索时间：约 500ms - 1s

HNSW 解决方案：
- 搜索时间：约 1-5ms
- 时间复杂度：O(log n)
- 加速比：100-1000倍
```

### 工作原理

```
层级结构示意：

Layer 2 (最高层，稀疏)    [Node A] ────────────────── [Node B]
                              │                            │
Layer 1 (中间层)         [C]──[D]──[E]────[F]──[G]──[H]──[I]
                              │    │    │    │    │
Layer 0 (底层，密集)    [1]-[2]-[3]-[4]-[5]-[6]-[7]-[8]-[9]-[10]
                              ↑
                           查询入口
```

**搜索过程：**
1. 从最高层入口点开始
2. 在当前层找到最近邻
3. 下降到下一层，以上一层结果为起点
4. 重复直到底层，返回结果

### 与其他索引对比

| 索引类型 | 搜索速度 | 构建速度 | 内存占用 | 精度 |
|----------|----------|----------|----------|------|
| 暴力搜索 | 最慢 | 无需构建 | 最低 | 100% |
| HNSW | 极快 | 中等 | 较高 | 95-99% |
| IVF | 快 | 快 | 中等 | 90-95% |
| LSH | 较快 | 快 | 低 | 80-90% |

### 实际应用场景

```
场景1: RAG (检索增强生成)
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│  用户问题   │ ──→ │ 向量嵌入    │ ──→ │ HNSW搜索    │
└─────────────┘     └─────────────┘     └─────────────┘
                                               │
                                               ▼
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│  生成回答   │ ←── │ LLM处理     │ ←── │ 相关文档    │
└─────────────┘     └─────────────┘     └─────────────┘

场景2: 推荐系统
用户向量 ──→ HNSW搜索 ──→ 相似用户/商品 ──→ 推荐结果

场景3: 图像检索
图像特征向量 ──→ HNSW搜索 ──→ 相似图片
```

### HNSW 缺失 ≠ 功能缺失

**核心功能对比：**

| 功能 | kimi25 | minimax25 | glm5 | 说明 |
|------|:------:|:---------:|:----:|------|
| 向量存储 | ✅ | ✅ | ✅ | 完整 |
| 向量检索 | ✅ | ✅ | ✅ | 完整 |
| 相似度搜索 | ✅ | ✅ | ✅ | 完整 |
| Top-K查询 | ✅ | ✅ | ✅ | 完整 |
| 距离计算 | ✅ | ✅ | ✅ | 完整 |
| 持久化 | ✅ | ✅ | ✅ | 完整 |
| **HNSW索引** | ⚠️框架 | ❌ | ❌ | **加速优化** |

**功能完整性图示：**

```
┌─────────────────────────────────────────────────────┐
│              向量数据库核心功能（必需）                │
│  ┌─────────┐  ┌─────────┐  ┌─────────┐             │
│  │ 存储向量 │  │ 搜索向量 │  │ 持久化  │             │
│  └─────────┘  └─────────┘  └─────────┘             │
│         ↓          ↓          ↓                     │
│    glm5 ✅    minimax25 ✅   kimi25 ✅              │
└─────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────┐
│              性能优化功能（可选）                      │
│  ┌─────────────────────────────────────────────┐   │
│  │              HNSW 索引加速                    │   │
│  │        (将 O(n) → O(log n))                 │   │
│  └─────────────────────────────────────────────┘   │
│                      ↓                              │
│              仅 kimi25 有框架                        │
└─────────────────────────────────────────────────────┘
```

**实际影响：**

| 数据规模 | 无HNSW（暴力搜索） | 有HNSW | 差异 |
|----------|-------------------|--------|------|
| 1,000 | < 1ms | < 1ms | 无差异 |
| 10,000 | ~5-10ms | ~1ms | 可接受 |
| 100,000 | ~50-100ms | ~2ms | 开始明显 |
| 1,000,000 | ~500ms-1s | ~5ms | 必须优化 |

**结论：**

```
GLM5 / minimax25 没有 HNSW：

✅ 功能完整 - 所有核心功能都具备
✅ 小规模数据 - 性能完全够用
✅ 代码简洁 - 更易理解和维护
⚠️ 大规模数据 - 搜索会变慢（但功能正常）
```

### 何时需要 HNSW？

```
需要 HNSW 的场景：
├── 向量数量 > 100,000
├── 实时搜索要求（< 10ms 响应）
├── 高并发查询场景
└── 内存充足（HNSW 需要额外 20-50% 内存）

不需要 HNSW 的场景：
├── 向量数量 < 10,000
├── 离线批量处理
├── 内存受限环境
└── 追求代码简洁
```

### kimi25 中的 HNSW 框架

```c
// kimi25/vector_db.h 中的定义
typedef struct HNSWNode {
    uint64_t id;
    Vector vector;
    struct HNSWNode** neighbors;  // 邻居节点
    uint32_t neighbor_count;      // 邻居数量
    uint32_t level;               // 所在层级
} HNSWNode;

typedef struct {
    HNSWNode** layers;            // 多层索引
    uint32_t max_layers;          // 最大层数
    uint32_t max_neighbors;       // 每层最大邻居数
    uint32_t ef_construction;     // 构建时搜索宽度
    HNSWNode* entry_point;        // 入口点
    uint64_t node_count;
} HNSWIndex;
```

**当前状态：** kimi25 提供了 HNSW 的数据结构框架，但搜索实现是简化版本。完整实现需要：

1. **层级构建算法** - 随机分配节点到各层
2. **邻居选择** - 贪心搜索选择最近邻
3. **动态插入** - 增量更新索引
4. **搜索优化** - ef 参数调优

---

## kimi25 版本

### 特点
- HNSW索引框架预留
- 完整的向量操作API
- 支持相似度阈值过滤

### 编译运行

```bash
cd kimi25
make
./test_vector_db
```

### API 示例

```c
#include "vector_db.h"

// 创建数据库
VectorDB* db = vectordb_create(128, false);

// 创建向量
Vector* vec = vector_create(128);
// 填充数据...

// 插入
vectordb_insert(db, 1, vec, "metadata", 9);

// 搜索
Vector* query = vector_create(128);
SearchOptions opts = { .top_k = 10, .threshold = 0.5f };
uint32_t count;
SearchResult* results = vectordb_search(db, query, &opts, &count);

// 清理
search_results_destroy(results, count);
vectordb_destroy(db);
```

---

## minimax25 版本 ⭐ (推荐)

### 特点
- 哈希索引，O(1) ID查找
- 支持多种距离度量
- 重复ID检测
- **IVF 聚类索引**：近似最近邻加速搜索
- **向量归一化**：余弦相似度优化
- **批量搜索**：支持多查询并发

### 编译运行

```bash
cd minimax25
gcc -O3 -o test_vdb vdb.c test_vdb.c -lm
./test_vdb
```

### API 示例

```c
#include "vdb.h"

// 创建数据库
VectorDatabase* db = vdb_create(128);

// 创建向量
Vector* vec = vector_new(128);

// 插入
vdb_insert(db, 1, vec, "metadata", 9);

// 搜索（支持多种距离度量）
Vector* query = vector_new(128);
SearchOptions opts = { 
    .top_k = 10, 
    .metric = DISTANCE_COSINE  // 或 DISTANCE_EUCLIDEAN, DISTANCE_DOT_PRODUCT
};
uint32_t count;
SearchResult* results = vdb_search(db, query, &opts, &count);

// IVF 索引加速搜索
vdb_build_ivf_index(db, 32);  // 构建 32 个聚类
results = vdb_search_ivf(db, query, &opts, &count);

// 批量搜索
Vector* queries[10];
SearchResult* batch_results = vdb_batch_search(db, queries, 10, &opts, counts);

// 清理
vdb_free_results(results, count);
vdb_free(db);
```

### 性能优化

| 优化技术 | 说明 |
|----------|------|
| 哈希桶 8192 | 减少哈希冲突 |
| 向量归一化 | 插入时预处理，搜索更快 |
| IVF 聚类 | 剪枝搜索，减少计算量 |
| nprobe 参数 | 控制搜索精度与速度平衡 |

---

## glm5 版本

### 特点
- 代码精简（~350 行）
- 大容量哈希桶（8192）
- 最快的插入速度
- 支持记录计数

### 编译运行

```bash
cd glm5
gcc -O2 -o test_glm5 glm5_vdb.c test_glm5.c -lm
./test_glm5
```

### API 示例

```c
#include "glm5_vdb.h"

// 创建数据库
VecDB* db = vdb_new(128);

// 创建向量
Vector* v = vec_new(128);

// 插入
vdb_add(db, 1, v, "metadata", 9);

// 搜索
Vector* q = vec_new(128);
QueryOpts opts = { .k = 10, .metric = METRIC_COSINE };
uint32_t n;
QueryResult* r = vdb_query(db, q, &opts, &n);

// 获取记录数
uint64_t count = vdb_count(db);

// 清理
vdb_free_results(r, n);
vdb_free(db);
```

---

## qwen35 版本 ⭐ (推荐)

### 特点
- **最高性能**：插入速度 **491K** vec/s，搜索速度 **0.215ms**
- **SIMD 优化**：AVX 指令集加速，一次处理 4 个 float
- **大哈希桶**：16,384 个哈希桶，查找效率最优
- **完整功能**：支持三种距离度量、元数据、持久化
- **批量搜索**：支持多查询并发处理
- **代码清晰**：模块化设计，易于理解和扩展
- **测试完备**：包含完整的单元测试和性能基准测试

### 编译运行

```bash
cd qwen35
gcc -c qwen35_vdb.c -o qwen35_vdb.o -std=c99 -O3 -Wall -Wextra -mavx
gcc -c test_qwen35.c -o test_qwen35.o -std=c99 -O3 -Wall -Wextra
gcc qwen35_vdb.o test_qwen35.o -o test_qwen35 -lm
./test_qwen35
```

### 性能基准

测试环境：Apple M2, 128 维向量，1000 个向量

| 操作 | 性能 | 提升 |
|------|------|------|
| 插入 | 491,159 vectors/s | +39% |
| 搜索 | 0.215 ms/次 (k=5) | -20% |

### SIMD 优化

```c
// AVX 指令集优化
#include <xmmintrin.h>

// 一次处理 4 个 float
__m128 va = _mm_loadu_ps(&a[i]);
__m128 vb = _mm_loadu_ps(&b[i]);
__m128 result = _mm_mul_ps(va, vb);
```

- `qwen35_cosine_simd()` - SIMD 优化的余弦相似度
- `qwen35_euclidean_simd()` - SIMD 优化的欧氏距离
- `qwen35_db_search_batch()` - 批量搜索接口

### API 示例

```c
#include "qwen35_vdb.h"

// 创建数据库（128 维，余弦相似度）
qwen35_vector_db_t *db = qwen35_db_create(128, QWEN35_DIST_COSINE);

// 插入向量
float vector[128];
// ... 初始化向量数据
qwen35_db_insert(db, 1, vector, NULL, 0);

// 搜索最近邻（Top-5）
int64_t ids[5];
float distances[5];
int count = qwen35_db_search(db, query_vector, 5, ids, distances);

// 获取单个向量
float retrieved[128];
qwen35_db_get(db, 1, retrieved, NULL, NULL);

// 删除向量
qwen35_db_delete(db, 1);

// 保存到文件
qwen35_db_save(db, "database.bin");

// 从文件加载
qwen35_vector_db_t *loaded_db = qwen35_db_load("database.bin");

// 清理
qwen35_db_destroy(db);
```

### 核心 API

```c
// 数据库管理
qwen35_vector_db_t *qwen35_db_create(size_t dimensions, qwen35_distance_t dist_type);
void qwen35_db_destroy(qwen35_vector_db_t *db);

// 插入和删除
int qwen35_db_insert(qwen35_vector_db_t *db, int64_t id, const float *vector, 
                     void *metadata, size_t metadata_size);
int qwen35_db_delete(qwen35_vector_db_t *db, int64_t id);

// 查询
int qwen35_db_search(qwen35_vector_db_t *db, const float *query, size_t k, 
                     int64_t *out_ids, float *out_distances);
int qwen35_db_get(qwen35_vector_db_t *db, int64_t id, float *out_vector, 
                  void *out_metadata, size_t *out_metadata_size);

// 持久化
int qwen35_db_save(qwen35_vector_db_t *db, const char *filename);
qwen35_vector_db_t *qwen35_db_load(const char *filename);

// 工具函数
size_t qwen35_db_size(qwen35_vector_db_t *db);
const char *qwen35_get_version(void);
```

### 距离度量类型

```c
typedef enum {
    QWEN35_DIST_COSINE = 0,      // 余弦相似度（推荐用于文本嵌入）
    QWEN35_DIST_EUCLIDEAN = 1,   // 欧氏距离（推荐用于图像特征）
    QWEN35_DIST_DOT_PRODUCT = 2  // 点积（推荐用于推荐系统）
} qwen35_distance_t;
```

### 为什么选择 qwen35？

1. **最优性能**：插入和搜索速度都是所有版本中最快的
2. **易于使用**：清晰的 API 设计，文档完善
3. **功能完整**：支持所有核心功能和高级特性
4. **可扩展性**：模块化设计，便于添加新功能
5. **生产就绪**：完整的测试覆盖和错误处理

---

## 距离度量说明

| 度量方式 | 说明 | 值范围 |
|----------|------|--------|
| 余弦相似度 | 向量夹角的余弦值 | [-1, 1] |
| 欧氏距离 | 向量间的直线距离 | [0, ∞) |
| 点积 | 向量内积 | (-∞, ∞) |

### 选择建议

- **余弦相似度**: 文本嵌入、语义搜索
- **欧氏距离**: 图像特征、物理距离
- **点积**: 归一化向量、推荐系统

---

## 持久化

所有版本都支持数据持久化：

```c
// 保存
vdb_save(db, "database.bin");

// 加载
VecDB* db = vdb_load("database.bin");
```

---

## 性能优化建议

1. **最佳性能**：使用 qwen35 版本，插入和搜索速度最快 ⭐
2. **大数据量**：使用 glm5 或 qwen35 版本，哈希桶更多
3. **频繁搜索**：使用 qwen35 版本，搜索速度 0.27ms
4. **需要近似搜索**：扩展 kimi25 的 HNSW 实现
5. **内存受限**：使用 glm5 版本，代码最精简

---

## 快速开始

### 构建所有项目

```bash
# 使用 Makefile 构建所有项目
for project in qwen35 minimax25 glm5 kimi25; do
  cd $project && make && cd ..
done
```

### 单独构建项目

```bash
# 构建 qwen35 (推荐)
cd qwen35 && make

# 构建 minimax25
cd minimax25 && make

# 构建 glm5
cd glm5 && make

# 构建 kimi25
cd kimi25 && make
```

### 运行测试

```bash
# 运行 qwen35 测试
cd qwen35 && ./test_qwen35

# 运行 minimax25 测试
cd minimax25 && ./test_vdb

# 运行 glm5 测试
cd glm5 && ./test_glm5

# 运行 kimi25 测试
cd kimi25 && ./test_vector_db
```

### 清理构建

```bash
cd qwen35 && make clean
cd minimax25 && make clean
cd glm5 && make clean
cd kimi25 && make clean
```

---

## 依赖

- C99 标准库
- 数学库 (`-lm`)
- pthread 库 (`-lpthread`)
- GCC 或 Clang 编译器
- Make 构建工具 (可选)

---

## 持续集成

本项目使用 GitHub Actions 进行持续集成。每次推送到 main 分支或创建 Pull Request 时，会自动构建和测试所有四个项目。

查看 [构建状态](https://github.com/supermy/c-vector-database/actions/workflows/ci.yml)

---

## License

MIT License
