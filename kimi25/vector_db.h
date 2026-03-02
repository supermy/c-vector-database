#ifndef VECTOR_DB_H
#define VECTOR_DB_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>
#include <pthread.h>

#ifdef __cplusplus
extern "C" {
#endif

#define VECTOR_DB_VERSION "1.3.0-production"
#define VECTOR_DB_SUCCESS 0
#define VECTOR_DB_ERROR -1
#define VECTOR_DB_NOT_FOUND -2
#define VECTOR_DB_MEMORY_ERROR -3
#define VECTOR_DB_INVALID_DIM -4
#define VECTOR_DB_OBJECT_POOL_SIZE 256

// 向量结构
typedef struct {
    float* data;
    uint32_t dim;
} Vector;

// 向量记录（带ID和元数据）
typedef struct {
    uint64_t id;
    Vector vector;
    char* metadata;
    uint32_t metadata_len;
} VectorRecord;

// HNSW节点
typedef struct HNSWNode {
    uint64_t id;
    Vector vector;
    struct HNSWNode** neighbors;
    uint32_t neighbor_count;
    uint32_t level;
} HNSWNode;

// HNSW索引
typedef struct {
    HNSWNode** layers;
    uint32_t max_layers;
    uint32_t max_neighbors;
    uint32_t ef_construction;
    uint32_t entry_point_level;
    HNSWNode* entry_point;
    uint64_t node_count;
} HNSWIndex;

// 对象池
typedef struct {
    void** objects;
    uint32_t size;
    uint32_t capacity;
    size_t object_size;
} VDBObjectPool;

// 统计信息
typedef struct {
    uint64_t insert_count;
    uint64_t delete_count;
    uint64_t search_count;
    uint64_t get_count;
    double avg_insert_us;
    double avg_search_ms;
} VDBStats;

// 向量数据库
typedef struct {
    VectorRecord* records;
    uint64_t count;
    uint64_t capacity;
    uint32_t dim;
    HNSWIndex* index;
    bool use_hnsw;
    pthread_rwlock_t lock;
    VDBObjectPool* obj_pool;
    VDBStats stats;
    bool enable_stats;
} VectorDB;

// 搜索结果
typedef struct {
    uint64_t id;
    float score;
    char* metadata;
    uint32_t metadata_len;
} SearchResult;

// 搜索选项
typedef struct {
    uint32_t top_k;
    float threshold;
    bool use_hnsw;
    uint32_t ef_search;
} SearchOptions;

// 向量操作
Vector* vector_create(uint32_t dim);
void vector_destroy(Vector* vec);
Vector* vector_copy(const Vector* vec);
float vector_dot(const Vector* a, const Vector* b);
float vector_norm(const Vector* vec);
float cosine_similarity(const Vector* a, const Vector* b);
float euclidean_distance(const Vector* a, const Vector* b);

// 数据库操作
VectorDB* vectordb_create(uint32_t dim, bool use_hnsw);
void vectordb_destroy(VectorDB* db);
int vectordb_insert(VectorDB* db, uint64_t id, const Vector* vec, 
                    const char* metadata, uint32_t metadata_len);
int vectordb_delete(VectorDB* db, uint64_t id);
VectorRecord* vectordb_get(VectorDB* db, uint64_t id);
int vectordb_update(VectorDB* db, uint64_t id, const Vector* vec,
                    const char* metadata, uint32_t metadata_len);

// 搜索操作
SearchResult* vectordb_search(VectorDB* db, const Vector* query, 
                              const SearchOptions* options, 
                              uint32_t* result_count);
SearchResult* vectordb_search_exact(VectorDB* db, const Vector* query,
                                    const SearchOptions* options,
                                    uint32_t* result_count);
void search_results_destroy(SearchResult* results, uint32_t count);

// HNSW索引操作
HNSWIndex* hnsw_create(uint32_t max_neighbors, uint32_t ef_construction);
void hnsw_destroy(HNSWIndex* index);
int hnsw_insert(HNSWIndex* index, uint64_t id, const Vector* vec);
int hnsw_delete(HNSWIndex* index, uint64_t id);
SearchResult* hnsw_search(HNSWIndex* index, const Vector* query,
                          uint32_t top_k, uint32_t ef_search,
                          uint32_t* result_count);

// 工具函数
void vectordb_print_stats(const VectorDB* db);
int vectordb_save(const VectorDB* db, const char* filename);
VectorDB* vectordb_load(const char* filename);

// 对象池操作
VDBObjectPool* vdb_pool_create(size_t object_size, uint32_t capacity);
void vdb_pool_destroy(VDBObjectPool* pool);
void* vdb_pool_alloc(VDBObjectPool* pool);
void vdb_pool_free(VDBObjectPool* pool, void* obj);

// 统计操作
void vectordb_enable_stats(VectorDB* db, bool enable);
int vectordb_get_stats(VectorDB* db, VDBStats* stats);
void vectordb_reset_stats(VectorDB* db);
void vectordb_print_detailed_stats(VectorDB* db);

// 版本
const char* vectordb_get_version(void);

#ifdef __cplusplus
}
#endif

#endif // VECTOR_DB_H
