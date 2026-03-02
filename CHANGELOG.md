# Changelog

所有版本的重要更新、优化和修复都会记录在这里。

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
- **哈希桶**: 8192
- **插入性能**: 283,134 vectors/s

#### kimi25 版本
- HNSW 索引框架预留

### 文档更新
- 创建完整 README.md
- 添加性能对比表格
- 添加适用场景说明
- HNSW 索引详解

---

## 性能对比

| 版本 | 插入速度 | 搜索速度 | 索引方式 |
|------|---------|---------|----------|
| qwen35 | 353K/s | 0.27ms | 哈希 (16K桶) |
| minimax25 | 247K/s | 2ms | 哈希 (8K桶) + IVF |
| glm5 | 283K/s | 8.8ms | 哈希 (8K桶) |
| kimi25 | 131K/s | 5.1ms | 线性查找 |

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
| IVF 聚类 | ❌ | ✅ | ❌ | ❌ |
| HNSW 框架 | ❌ | ❌ | ❌ | ✅ |
| 持久化 | ✅ | ✅ | ✅ | ✅ |
| 重复 ID 检测 | ✅ | ✅ | ✅ | ❌ |

---

## 技术亮点

### IVF 聚类索引
参考 Milvus/Faiss 实现的倒排索引：
- K-Means 聚类将向量分组
- 搜索时只扫描相关聚类 (nprobe 参数)
- 大幅减少搜索计算量

### 向量归一化优化
- 插入时对向量进行 L2 归一化
- 搜索时使用快速点积计算
- 避免重复的模长计算

---

## 仓库信息

- **GitHub**: https://github.com/supermy/c-vector-database
- **License**: MIT
- **语言**: C99

---

## 提交历史

- `b8f2562` - Update README.md: Document minimax25 IVF optimization
- `1ca6d78` - Add IVF clustering index and batch search optimization
- `85cf460` - Optimize minimax25: increase hash buckets to 8192
- `6fdd057` - Add qwen35 vector database implementation
- `6ee6141` - Update .gitignore and cleanup build artifacts
- `980d48f` - Initial commit: C语言向量数据库实现
