#include "vector_db.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <math.h>
#include <time.h>

#define INITIAL_CAPACITY 1024
#define HNSW_MAX_LAYERS 16

// ==================== 向量操作 ====================

Vector* vector_create(uint32_t dim) {
    if (dim == 0) return NULL;
    
    Vector* vec = (Vector*)malloc(sizeof(Vector));
    if (!vec) return NULL;
    
    vec->data = (float*)calloc(dim, sizeof(float));
    if (!vec->data) {
        free(vec);
        return NULL;
    }
    
    vec->dim = dim;
    return vec;
}

void vector_destroy(Vector* vec) {
    if (vec) {
        free(vec->data);
        free(vec);
    }
}

Vector* vector_copy(const Vector* vec) {
    if (!vec) return NULL;
    
    Vector* copy = vector_create(vec->dim);
    if (!copy) return NULL;
    
    memcpy(copy->data, vec->data, vec->dim * sizeof(float));
    return copy;
}

float vector_dot(const Vector* a, const Vector* b) {
    if (!a || !b || a->dim != b->dim) return 0.0f;
    
    float sum = 0.0f;
    for (uint32_t i = 0; i < a->dim; i++) {
        sum += a->data[i] * b->data[i];
    }
    return sum;
}

float vector_norm(const Vector* vec) {
    if (!vec) return 0.0f;
    
    float sum = 0.0f;
    for (uint32_t i = 0; i < vec->dim; i++) {
        sum += vec->data[i] * vec->data[i];
    }
    return sqrtf(sum);
}

float cosine_similarity(const Vector* a, const Vector* b) {
    if (!a || !b || a->dim != b->dim) return -1.0f;
    
    float dot = vector_dot(a, b);
    float norm_a = vector_norm(a);
    float norm_b = vector_norm(b);
    
    if (norm_a == 0.0f || norm_b == 0.0f) return -1.0f;
    
    return dot / (norm_a * norm_b);
}

float euclidean_distance(const Vector* a, const Vector* b) {
    if (!a || !b || a->dim != b->dim) return -1.0f;
    
    float sum = 0.0f;
    for (uint32_t i = 0; i < a->dim; i++) {
        float diff = a->data[i] - b->data[i];
        sum += diff * diff;
    }
    return sqrtf(sum);
}

// ==================== 向量数据库操作 ====================

static int compare_search_result(const void* a, const void* b) {
    const SearchResult* sa = (const SearchResult*)a;
    const SearchResult* sb = (const SearchResult*)b;
    // 降序排列（分数越高越相似）
    if (sa->score > sb->score) return -1;
    if (sa->score < sb->score) return 1;
    return 0;
}

VectorDB* vectordb_create(uint32_t dim, bool use_hnsw) {
    if (dim == 0) return NULL;
    
    VectorDB* db = (VectorDB*)malloc(sizeof(VectorDB));
    if (!db) return NULL;
    
    db->records = (VectorRecord*)calloc(INITIAL_CAPACITY, sizeof(VectorRecord));
    if (!db->records) {
        free(db);
        return NULL;
    }
    
    db->count = 0;
    db->capacity = INITIAL_CAPACITY;
    db->dim = dim;
    db->use_hnsw = use_hnsw;
    
    if (use_hnsw) {
        db->index = hnsw_create(16, 200);
        if (!db->index) {
            free(db->records);
            free(db);
            return NULL;
        }
    } else {
        db->index = NULL;
    }
    
    return db;
}

void vectordb_destroy(VectorDB* db) {
    if (!db) return;
    
    for (uint64_t i = 0; i < db->count; i++) {
        free(db->records[i].vector.data);
        free(db->records[i].metadata);
    }
    
    free(db->records);
    
    if (db->index) {
        hnsw_destroy(db->index);
    }
    
    free(db);
}

static int vectordb_resize(VectorDB* db) {
    if (db->count < db->capacity) return VECTOR_DB_SUCCESS;
    
    uint64_t new_capacity = db->capacity * 2;
    VectorRecord* new_records = (VectorRecord*)realloc(db->records, 
                                                        new_capacity * sizeof(VectorRecord));
    if (!new_records) return VECTOR_DB_MEMORY_ERROR;
    
    db->records = new_records;
    db->capacity = new_capacity;
    return VECTOR_DB_SUCCESS;
}

int vectordb_insert(VectorDB* db, uint64_t id, const Vector* vec,
                    const char* metadata, uint32_t metadata_len) {
    if (!db || !vec) return VECTOR_DB_ERROR;
    if (vec->dim != db->dim) return VECTOR_DB_INVALID_DIM;
    
    // 检查ID是否已存在
    for (uint64_t i = 0; i < db->count; i++) {
        if (db->records[i].id == id) return VECTOR_DB_ERROR;
    }
    
    if (vectordb_resize(db) != VECTOR_DB_SUCCESS) {
        return VECTOR_DB_MEMORY_ERROR;
    }
    
    VectorRecord* record = &db->records[db->count];
    record->id = id;
    record->vector.dim = vec->dim;
    record->vector.data = (float*)malloc(vec->dim * sizeof(float));
    if (!record->vector.data) return VECTOR_DB_MEMORY_ERROR;
    
    memcpy(record->vector.data, vec->data, vec->dim * sizeof(float));
    
    if (metadata && metadata_len > 0) {
        record->metadata = (char*)malloc(metadata_len);
        if (!record->metadata) {
            free(record->vector.data);
            return VECTOR_DB_MEMORY_ERROR;
        }
        memcpy(record->metadata, metadata, metadata_len);
        record->metadata_len = metadata_len;
    } else {
        record->metadata = NULL;
        record->metadata_len = 0;
    }
    
    db->count++;
    
    // 插入到HNSW索引
    if (db->use_hnsw && db->index) {
        hnsw_insert(db->index, id, vec);
    }
    
    return VECTOR_DB_SUCCESS;
}

int vectordb_delete(VectorDB* db, uint64_t id) {
    if (!db) return VECTOR_DB_ERROR;
    
    for (uint64_t i = 0; i < db->count; i++) {
        if (db->records[i].id == id) {
            free(db->records[i].vector.data);
            free(db->records[i].metadata);
            
            // 移动后面的记录
            if (i < db->count - 1) {
                memmove(&db->records[i], &db->records[i + 1], 
                        (db->count - i - 1) * sizeof(VectorRecord));
            }
            
            db->count--;
            
            // 从HNSW索引删除
            if (db->use_hnsw && db->index) {
                hnsw_delete(db->index, id);
            }
            
            return VECTOR_DB_SUCCESS;
        }
    }
    
    return VECTOR_DB_NOT_FOUND;
}

VectorRecord* vectordb_get(VectorDB* db, uint64_t id) {
    if (!db) return NULL;
    
    for (uint64_t i = 0; i < db->count; i++) {
        if (db->records[i].id == id) {
            return &db->records[i];
        }
    }
    
    return NULL;
}

int vectordb_update(VectorDB* db, uint64_t id, const Vector* vec,
                    const char* metadata, uint32_t metadata_len) {
    if (!db) return VECTOR_DB_ERROR;
    
    VectorRecord* record = vectordb_get(db, id);
    if (!record) return VECTOR_DB_NOT_FOUND;
    
    if (vec) {
        if (vec->dim != db->dim) return VECTOR_DB_INVALID_DIM;
        
        free(record->vector.data);
        record->vector.data = (float*)malloc(vec->dim * sizeof(float));
        if (!record->vector.data) return VECTOR_DB_MEMORY_ERROR;
        
        memcpy(record->vector.data, vec->data, vec->dim * sizeof(float));
        record->vector.dim = vec->dim;
    }
    
    if (metadata) {
        free(record->metadata);
        record->metadata = (char*)malloc(metadata_len);
        if (!record->metadata) return VECTOR_DB_MEMORY_ERROR;
        
        memcpy(record->metadata, metadata, metadata_len);
        record->metadata_len = metadata_len;
    }
    
    return VECTOR_DB_SUCCESS;
}

SearchResult* vectordb_search_exact(VectorDB* db, const Vector* query,
                                    const SearchOptions* options,
                                    uint32_t* result_count) {
    if (!db || !query || !result_count) return NULL;
    if (query->dim != db->dim) return NULL;
    
    uint32_t top_k = options ? options->top_k : 10;
    float threshold = options ? options->threshold : 0.0f;
    
    if (db->count == 0) {
        *result_count = 0;
        return NULL;
    }
    
    SearchResult* results = (SearchResult*)malloc(db->count * sizeof(SearchResult));
    if (!results) return NULL;
    
    uint32_t count = 0;
    for (uint64_t i = 0; i < db->count; i++) {
        float score = cosine_similarity(query, &db->records[i].vector);
        
        if (score >= threshold) {
            results[count].id = db->records[i].id;
            results[count].score = score;
            
            if (db->records[i].metadata && db->records[i].metadata_len > 0) {
                results[count].metadata = (char*)malloc(db->records[i].metadata_len);
                if (results[count].metadata) {
                    memcpy(results[count].metadata, db->records[i].metadata, 
                           db->records[i].metadata_len);
                    results[count].metadata_len = db->records[i].metadata_len;
                }
            } else {
                results[count].metadata = NULL;
                results[count].metadata_len = 0;
            }
            
            count++;
        }
    }
    
    // 排序并截取top_k
    qsort(results, count, sizeof(SearchResult), compare_search_result);
    
    if (count > top_k) {
        for (uint32_t i = top_k; i < count; i++) {
            free(results[i].metadata);
        }
        count = top_k;
    }
    
    *result_count = count;
    return results;
}

SearchResult* vectordb_search(VectorDB* db, const Vector* query,
                              const SearchOptions* options,
                              uint32_t* result_count) {
    if (!db || !query || !result_count) return NULL;
    
    bool use_hnsw = (options && options->use_hnsw) || 
                    (!options && db->use_hnsw);
    
    if (use_hnsw && db->index && db->index->entry_point) {
        uint32_t top_k = options ? options->top_k : 10;
        uint32_t ef_search = options ? options->ef_search : 64;
        return hnsw_search(db->index, query, top_k, ef_search, result_count);
    }
    
    return vectordb_search_exact(db, query, options, result_count);
}

void search_results_destroy(SearchResult* results, uint32_t count) {
    if (!results) return;
    
    for (uint32_t i = 0; i < count; i++) {
        free(results[i].metadata);
    }
    free(results);
}

void vectordb_print_stats(const VectorDB* db) {
    if (!db) return;
    
    printf("VectorDB Statistics:\n");
    printf("  Dimension: %u\n", db->dim);
    printf("  Record count: %llu\n", (unsigned long long)db->count);
    printf("  Capacity: %llu\n", (unsigned long long)db->capacity);
    printf("  Using HNSW: %s\n", db->use_hnsw ? "yes" : "no");
    
    if (db->use_hnsw && db->index) {
        printf("  HNSW node count: %llu\n", (unsigned long long)db->index->node_count);
        printf("  HNSW max layers: %u\n", db->index->max_layers);
    }
}

// ==================== HNSW索引实现 ====================

static uint32_t random_level(uint32_t max_layers) {
    static int seeded = 0;
    if (!seeded) {
        srand((unsigned int)time(NULL));
        seeded = 1;
    }
    
    uint32_t level = 0;
    while (level < max_layers - 1 && (rand() / (double)RAND_MAX) < 0.5) {
        level++;
    }
    return level;
}

HNSWIndex* hnsw_create(uint32_t max_neighbors, uint32_t ef_construction) {
    HNSWIndex* index = (HNSWIndex*)malloc(sizeof(HNSWIndex));
    if (!index) return NULL;
    
    index->layers = (HNSWNode**)calloc(HNSW_MAX_LAYERS, sizeof(HNSWNode*));
    if (!index->layers) {
        free(index);
        return NULL;
    }
    
    index->max_layers = HNSW_MAX_LAYERS;
    index->max_neighbors = max_neighbors;
    index->ef_construction = ef_construction;
    index->entry_point_level = 0;
    index->entry_point = NULL;
    index->node_count = 0;
    
    return index;
}

void hnsw_destroy(HNSWIndex* index) {
    if (!index) return;
    
    // 简化的清理 - 实际应该遍历所有节点
    free(index->layers);
    free(index);
}

static HNSWNode* hnsw_node_create(uint64_t id, const Vector* vec, 
                                   uint32_t max_neighbors, uint32_t level) {
    HNSWNode* node = (HNSWNode*)malloc(sizeof(HNSWNode));
    if (!node) return NULL;
    
    node->id = id;
    node->vector.data = (float*)malloc(vec->dim * sizeof(float));
    if (!node->vector.data) {
        free(node);
        return NULL;
    }
    memcpy(node->vector.data, vec->data, vec->dim * sizeof(float));
    node->vector.dim = vec->dim;
    
    node->neighbors = (HNSWNode**)calloc(max_neighbors, sizeof(HNSWNode*));
    if (!node->neighbors) {
        free(node->vector.data);
        free(node);
        return NULL;
    }
    
    node->neighbor_count = 0;
    node->level = level;
    
    return node;
}

static void hnsw_node_destroy(HNSWNode* node) {
    if (!node) return;
    free(node->vector.data);
    free(node->neighbors);
    free(node);
}

int hnsw_insert(HNSWIndex* index, uint64_t id, const Vector* vec) {
    if (!index || !vec) return VECTOR_DB_ERROR;
    
    uint32_t level = random_level(index->max_layers);
    
    HNSWNode* new_node = hnsw_node_create(id, vec, index->max_neighbors, level);
    if (!new_node) return VECTOR_DB_MEMORY_ERROR;
    
    // 简化的插入 - 实际HNSW需要更复杂的逻辑
    if (!index->entry_point) {
        index->entry_point = new_node;
        index->entry_point_level = level;
    }
    
    index->node_count++;
    return VECTOR_DB_SUCCESS;
}

int hnsw_delete(HNSWIndex* index, uint64_t id) {
    // 简化实现
    return VECTOR_DB_SUCCESS;
}

SearchResult* hnsw_search(HNSWIndex* index, const Vector* query,
                          uint32_t top_k, uint32_t ef_search,
                          uint32_t* result_count) {
    // 简化实现 - 返回空结果
    *result_count = 0;
    return NULL;
}

// ==================== 持久化 ====================

int vectordb_save(const VectorDB* db, const char* filename) {
    if (!db || !filename) return VECTOR_DB_ERROR;
    
    FILE* fp = fopen(filename, "wb");
    if (!fp) return VECTOR_DB_ERROR;
    
    // 写入头部信息
    fwrite(&db->dim, sizeof(uint32_t), 1, fp);
    fwrite(&db->count, sizeof(uint64_t), 1, fp);
    fwrite(&db->use_hnsw, sizeof(bool), 1, fp);
    
    // 写入记录
    for (uint64_t i = 0; i < db->count; i++) {
        fwrite(&db->records[i].id, sizeof(uint64_t), 1, fp);
        fwrite(&db->records[i].metadata_len, sizeof(uint32_t), 1, fp);
        fwrite(db->records[i].vector.data, sizeof(float), db->dim, fp);
        if (db->records[i].metadata_len > 0) {
            fwrite(db->records[i].metadata, 1, db->records[i].metadata_len, fp);
        }
    }
    
    fclose(fp);
    return VECTOR_DB_SUCCESS;
}

VectorDB* vectordb_load(const char* filename) {
    if (!filename) return NULL;
    
    FILE* fp = fopen(filename, "rb");
    if (!fp) return NULL;
    
    uint32_t dim;
    uint64_t count;
    bool use_hnsw;
    
    if (fread(&dim, sizeof(uint32_t), 1, fp) != 1) {
        fclose(fp);
        return NULL;
    }
    fread(&count, sizeof(uint64_t), 1, fp);
    fread(&use_hnsw, sizeof(bool), 1, fp);
    
    VectorDB* db = vectordb_create(dim, use_hnsw);
    if (!db) {
        fclose(fp);
        return NULL;
    }
    
    for (uint64_t i = 0; i < count; i++) {
        uint64_t id;
        uint32_t metadata_len;
        
        fread(&id, sizeof(uint64_t), 1, fp);
        fread(&metadata_len, sizeof(uint32_t), 1, fp);
        
        Vector* vec = vector_create(dim);
        if (!vec) {
            vectordb_destroy(db);
            fclose(fp);
            return NULL;
        }
        
        fread(vec->data, sizeof(float), dim, fp);
        
        char* metadata = NULL;
        if (metadata_len > 0) {
            metadata = (char*)malloc(metadata_len);
            fread(metadata, 1, metadata_len, fp);
        }
        
        vectordb_insert(db, id, vec, metadata, metadata_len);
        
        vector_destroy(vec);
        free(metadata);
    }
    
    fclose(fp);
    return db;
}
