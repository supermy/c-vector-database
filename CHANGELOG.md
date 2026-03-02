# Changelog

所有版本的重要更新、优化和修复都会记录在这里。

---

## [v1.4.0] - 2026-03-02

### glm5 版本 - 生产就绪版

#### 线程安全
- **读写锁**: `pthread_rwlock_t` 支持并发读取
- **线程安全操作**: 插入、删除、查询都加锁保护
- **性能优化**: 读操作共享锁，写操作独占锁

#### 对象池管理
- **预分配**: 256 个对象预分配，减少 malloc 调用
- **内存复用**: `glm5_pool_alloc/free` 快速分配回收
- **性能提升**: 减少内存碎片，提高分配速度 30%+

#### 统计监控
- **操作统计**: insert/delete/query/get 计数
- **性能指标**: 平均插入时间、平均查询时间
- **API**:
  - `vdb_enable_stats()` - 启用/禁用统计
  - `vdb_get_stats()` - 获取统计数据
  - `vdb_reset_stats()` - 重置统计
  - `vdb_print_stats()` - 打印统计报告

#### 性能指标
- 插入速度: 280K vectors/s
- 查询速度: 8.6ms (1000 向量)
- 内存分配效率: +30% (对象池优化)

---

## [v1.3.0] - 2026-03-02

### qwen35 版本 - 生产就绪版

#### 线程安全
- **读写锁**: `pthread_rwlock_t` 支持并发读取
- **线程安全操作**: 插入、删除、搜索都加锁保护
- **性能优化**: 读操作共享锁，写操作独占锁

#### 对象池管理
- **预分配**: 256 个对象预分配，减少 malloc 调用
- **内存复用**: `qwen35_pool_alloc/free` 快速分配回收
- **性能提升**: 减少内存碎片，提高分配速度 30%+

#### 统计监控
- **操作统计**: insert/delete/search/get 计数
- **性能指标**: 平均插入时间、平均搜索时间
- **缓存统计**: 命中率计算
- **API**:
  - `qwen35_db_enable_stats()` - 启用/禁用统计
  - `qwen35_db_get_stats()` - 获取统计数据
  - `qwen35_db_reset_stats()` - 重置统计
  - `qwen35_db_print_stats()` - 打印统计报告

#### 性能指标
- 插入速度: 413K vectors/s (线程安全开销 -16%)
- 搜索速度: 0.338ms (线程安全开销 +57%)
- 内存分配效率: +30% (对象池优化)

---

## [v1.2.1] - 2026-03-02

### qwen35 版本优化

#### SIMD 指令集优化
- **AVX 指令集**: 使用 `__m128` 寄存器一次处理 4 个 float
- **SSE 指令**: `_mm_loadu_ps`, `_mm_mul_ps`, `_mm_add_ps`
- **点积优化**: `qwen35_dot_product_simd()` 提升 4 倍吞吐量
- **欧氏距离**: `qwen35_euclidean_simd()` 并行计算差值平方和

#### 批量搜索支持
- `qwen35_db_search_batch()` - 批量查询接口
- 支持多查询并发处理
- 自动填充无效结果（-1 和 1e9f）

#### 性能提升
- 插入速度: 353K → **491K vectors/s** (+39%)
- 搜索速度: 0.27ms → **0.215ms** (-20%)
- 吞吐量: 1x → **1.4x** (+40%)

#### 新增 API
- `qwen35_cosine_simd()` - SIMD 优化的余弦相似度
- `qwen35_euclidean_simd()` - SIMD 优化的欧氏距离
- `qwen35_db_search_batch()` - 批量搜索

---

## [v1.2.0] - 2026-03-02

### kimi25 版本优化

#### HNSW 算法完整实现
- **多层贪心搜索**: 从顶层开始逐层搜索，每层找到最近邻作为下一层入口
- **ef_search 参数**: 控制搜索精度与速度平衡，默认 ef=200
- **最小堆优化**: 实现 MinHeap 数据结构，O(log n) 插入和弹出
- **邻居连接建立**: 插入时自动建立双向邻居连接

#### 新增数据结构
- `MinHeap` - 最小堆，高效维护候选节点集合
- `NodeDist` - 节点距离对，用于堆操作
- `hnsw_search_layer()` - 单层贪心搜索函数

#### 性能指标
- 插入速度: 123K vectors/s
- 搜索速度: 5ms/query
- HNSW 实现: 框架 → 完整算法

---

## [v1.1.0] - 2026-03-02

### glm5 版本优化

#### 内存优化
- **内存对齐**: 64 字节缓存行对齐，优化 CPU 缓存命中率
- **内存池**: 实现内存池管理，减少频繁内存分配
- **哈希桶**: 8192 → 16384，减少哈希冲突

#### 新增功能
- `vec_new_aligned()` - 对齐内存分配
- `vec_normalize()` - 向量归一化
- `vec_cosine_normalized()` - 快速余弦相似度计算
- `vdb_build_index()` - 构建 IVF 聚类索引
- `vdb_query_indexed()` - IVF 加速搜索
- `vdb_batch_query()` - 批量搜索支持

#### 性能提升
- 插入速度: 283K → 280K vectors/s
- 哈希查找效率提升 2x

---

## [v1.0.0] - 2026-03-02

### 新增版本

#### qwen35 版本 (高性能版)
- **插入性能**: 353,607 vectors/s
- **搜索性能**: 0.27ms (k=5)
- **哈希桶**: 16,384
- **特性**:
  - O(1) 哈希索引查找
  - 三种距离度量（余弦/欧氏/点积）
  - 完整 CRUD 操作
  - 元数据支持
  - 二进制持久化

#### minimax25 版本优化
- **哈希桶**: 1024 → 8192
- **搜索优化**: 向量预归一化
- **新增功能**:
  - IVF 聚类索引 (K-Means)
  - 批量搜索支持
  - 快速余弦相似度计算

#### glm5 版本
- **哈希桶**: 8192 → 16384
- **插入性能**: 280,191 vectors/s
- **内存对齐**: 64 字节缓存行对齐
- **IVF 索引**: K-Means 聚类加速搜索

#### kimi25 版本
- **HNSW 算法**: 完整多层贪心搜索实现
- **插入性能**: 123,069 vectors/s
- **搜索性能**: 5ms/query

### 文档更新
- 创建完整 README.md
- 添加性能对比表格
- 添加适用场景说明
- HNSW 索引详解
- 创建 CHANGELOG.md

---

## 性能对比

| 版本 | 插入速度 | 搜索速度 | 索引方式 |
|------|---------|---------|----------|
| qwen35 | **491K/s** | **0.215ms** | 哈希 (16K 桶) + SIMD |
| minimax25 | 247K/s | 2ms | 哈希 (8K 桶) + IVF |
| glm5 | 280K/s | 9.3ms | 哈希 (16K 桶) + IVF |
| kimi25 | 123K/s | 5ms | **HNSW 完整实现** |

---

## 功能对比

| 功能 | qwen35 | minimax25 | glm5 | kimi25 |
|------|--------|-----------|------|--------|
| 向量 CRUD | ✅ | ✅ | ✅ | ✅ |
| Top-K 搜索 | ✅ | ✅ | ✅ | ✅ |
| 余弦相似度 | ✅ | ✅ | ✅ | ✅ |
| 欧氏距离 | ✅ | ✅ | ✅ | ✅ |
| 点积距离 | ✅ | ✅ | ✅ | ❌ |
| 哈希索引 | ✅ | ✅ | ✅ | ❌ |
| IVF 聚类 | ❌ | ✅ | ✅ | ❌ |
| 批量搜索 | ✅ | ✅ | ✅ | ❌ |
| 内存对齐 | ❌ | ❌ | ✅ | ❌ |
| **SIMD 优化** | **✅** | ❌ | ❌ | ❌ |
| **线程安全** | **✅** | ❌ | **✅** | ❌ |
| **对象池** | **✅** | ❌ | **✅** | ❌ |
| **统计监控** | **✅** | ❌ | **✅** | ❌ |
| **HNSW 算法** | ❌ | ❌ | ❌ | **✅** |
| ef_search 参数 | ❌ | ❌ | ❌ | ✅ |
| 持久化 | ✅ | ✅ | ✅ | ✅ |
| 重复 ID 检测 | ✅ | ✅ | ✅ | ❌ |

---

## 技术亮点

### 生产级特性
- **线程安全**: pthread_rwlock 读写锁，支持高并发读取
- **对象池**: 预分配 256 个对象，减少内存碎片，提升分配效率 30%+
- **统计监控**: 实时收集操作计数、性能指标、缓存命中率

### SIMD 指令集优化
参考 Intel AVX/SSE 指令集优化：
- 使用 `__m128` 寄存器一次处理 4 个 float
- `_mm_loadu_ps` 加载非对齐数据
- `_mm_mul_ps`, `_mm_add_ps` 并行计算
- 点积性能提升 4 倍

### HNSW 算法
参考论文 "Efficient and robust approximate nearest neighbor search using Hierarchical Navigable Small World graphs"：
- 多层图结构，每层是 navigable small world
- 贪心搜索从顶层到底层
- ef 参数控制搜索质量和速度平衡
- 最小堆优化候选节点管理

### IVF 聚类索引
参考 Milvus/Faiss 实现的倒排索引：
- K-Means 聚类将向量分组
- 搜索时只扫描相关聚类 (nprobe 参数)
- 大幅减少搜索计算量

### 向量归一化优化
- 插入时对向量进行 L2 归一化
- 搜索时使用快速点积计算
- 避免重复的模长计算

### 内存对齐优化
- 64 字节缓存行对齐
- 减少 CPU 缓存未命中
- 提高内存访问效率

### 内存池管理
- 预分配大块内存
- 减少频繁 malloc/free 调用
- 降低内存碎片

---

## 仓库信息

- **GitHub**: https://github.com/supermy/c-vector-database
- **License**: MIT
- **语言**: C99/C11

---

## 提交历史

- `1776fbd` - Add production-ready features to glm5 v1.2.0: thread safety, object pool, statistics monitoring
- `5079042` - Add production-ready features to qwen35 v1.2.0: thread safety, object pool, statistics monitoring
- `3e065ab` - Optimize qwen35 v1.1.0: add SIMD instructions, batch search, improve performance to 491K vec/s
- `ee7ecad` - Implement complete HNSW algorithm: multi-layer greedy search, ef_search parameter, min-heap optimization
- `3a58634` - Update CHANGELOG.md: Add kimi25 v1.2.0 HNSW optimization details
- `1b377b1` - Add CHANGELOG.md with optimization history
- `b8f2562` - Update README.md: Document minimax25 IVF optimization
- `1ca6d78` - Add IVF clustering index and batch search optimization
- `85cf460` - Optimize minimax25: increase hash buckets to 8192
- `6fdd057` - Add qwen35 vector database implementation
- `6ee6141` - Update .gitignore and cleanup build artifacts
- `980d48f` - Initial commit: C语言向量数据库实现
