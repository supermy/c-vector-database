# 向量数据库 (Vector Database)

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
└── benchmark.c       # 性能对比测试
```

## 版本对比

### 性能指标

| 指标 | kimi25 | minimax25 | glm5 |
|------|--------|-----------|------|
| 插入速度 | 131,503 vec/s | 267,294 vec/s | 283,134 vec/s |
| 搜索速度 | 5.1 ms/query | 4.5 ms/query | 8.8 ms/query |
| ID查找 | O(n) 线性 | O(1) 哈希 | O(1) 哈希 |

### 功能特性

| 功能 | kimi25 | minimax25 | glm5 |
|------|:------:|:---------:|:----:|
| 向量CRUD | ✅ | ✅ | ✅ |
| Top-K搜索 | ✅ | ✅ | ✅ |
| 余弦相似度 | ✅ | ✅ | ✅ |
| 欧氏距离 | ✅ | ✅ | ✅ |
| 点积距离 | ❌ | ✅ | ✅ |
| 距离度量切换 | ❌ | ✅ | ✅ |
| 哈希索引 | ❌ | ✅ | ✅ |
| HNSW框架 | ✅ | ❌ | ❌ |
| 持久化 | ✅ | ✅ | ✅ |
| 重复ID检测 | ❌ | ✅ | ✅ |

### 适用场景

| 版本 | 适用场景 |
|------|----------|
| **kimi25** | 小数据量、需要扩展HNSW索引 |
| **minimax25** | 中等数据量、通用场景、需要快速搜索 |
| **glm5** | 大数据量、内存敏感、需要快速插入 |

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

## minimax25 版本

### 特点
- 哈希索引，O(1) ID查找
- 支持多种距离度量
- 重复ID检测

### 编译运行

```bash
cd minimax25
gcc -O2 -o test_vdb vdb.c test_vdb.c -lm
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

// 清理
vdb_free_results(results, count);
vdb_free(db);
```

---

## glm5 版本

### 特点
- 代码精简（~350行）
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

1. **大数据量**: 使用 glm5 版本，哈希桶更多
2. **频繁搜索**: 使用 minimax25 版本，搜索更快
3. **需要近似搜索**: 扩展 kimi25 的 HNSW 实现

---

## 依赖

- C99 标准库
- 数学库 (`-lm`)

---

## License

MIT License
