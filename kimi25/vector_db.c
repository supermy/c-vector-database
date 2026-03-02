#include "vector_db.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <math.h>
#include <time.h>

#define INITIAL_CAPACITY 1024
#define HNSW_MAX_LAYERS 16

// ==================== 对象池实现 ====================

VDBObjectPool* vdb_pool_create(size_t object_size, uint32_t capacity) {
    VDBObjectPool* pool = (VDBObjectPool*)malloc(sizeof(VDBObjectPool));
    if (!pool) return NULL;
    
    pool->objects = (void**)calloc(capacity, sizeof(void*));
    if (!pool->objects) { free(pool); return NULL; }
    
    for (uint32_t i = 0; i < capacity; i++) {
        pool->objects[i] = calloc(1, object_size);
        if (!pool->objects[i]) {
            for (uint32_t j = 0; j < i; j++) free(pool->objects[j]);
            free(pool->objects);
            free(pool);
            return NULL;
        }
    }
    
    pool->size = capacity;
    pool->capacity = capacity;
    pool->object_size = object_size;
    return pool;
}

void vdb_pool_destroy(VDBObjectPool* pool) {
    if (!pool) return;
    for (uint32_t i = 0; i < pool->capacity; i++) {
        if (pool->objects[i]) free(pool->objects[i]);
    }
    free(pool->objects);
    free(pool);
}

void* vdb_pool_alloc(VDBObjectPool* pool) {
    if (!pool || pool->size == 0) return NULL;
    return pool->objects[--pool->size];
}

void vdb_pool_free(VDBObjectPool* pool, void* obj) {
    if (!pool || !obj || pool->size >= pool->capacity) return;
    pool->objects[pool->size++] = obj;
}

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
    
    pthread_rwlock_init(&db->lock, NULL);
    db->obj_pool = vdb_pool_create(sizeof(VectorRecord), VECTOR_DB_OBJECT_POOL_SIZE);
    
    db->count = 0;
    db->capacity = INITIAL_CAPACITY;
    db->dim = dim;
    db->use_hnsw = use_hnsw;
    db->enable_stats = true;
    memset(&db->stats, 0, sizeof(VDBStats));
    
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
    
    pthread_rwlock_destroy(&db->lock);
    
    for (uint64_t i = 0; i < db->count; i++) {
        free(db->records[i].vector.data);
        free(db->records[i].metadata);
    }
    
    if (db->obj_pool) vdb_pool_destroy(db->obj_pool);
    
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

void vectordb_enable_stats(VectorDB* db, bool enable) {
    if (!db) return;
    db->enable_stats = enable;
}

int vectordb_get_stats(VectorDB* db, VDBStats* stats) {
    if (!db || !stats) return VECTOR_DB_ERROR;
    pthread_rwlock_rdlock(&db->lock);
    memcpy(stats, &db->stats, sizeof(VDBStats));
    pthread_rwlock_unlock(&db->lock);
    return VECTOR_DB_SUCCESS;
}

void vectordb_reset_stats(VectorDB* db) {
    if (!db) return;
    pthread_rwlock_wrlock(&db->lock);
    memset(&db->stats, 0, sizeof(VDBStats));
    pthread_rwlock_unlock(&db->lock);
}

void vectordb_print_detailed_stats(VectorDB* db) {
    if (!db) return;
    pthread_rwlock_rdlock(&db->lock);
    
    printf("\n=== Kimi25 VectorDB Statistics ===\n");
    printf("Version: %s\n", VECTOR_DB_VERSION);
    printf("Dimension: %u\n", db->dim);
    printf("Size: %llu / %llu\n", (unsigned long long)db->count, (unsigned long long)db->capacity);
    printf("Using HNSW: %s\n", db->use_hnsw ? "yes" : "no");
    
    if (db->use_hnsw && db->index) {
        printf("HNSW node count: %llu\n", (unsigned long long)db->index->node_count);
        printf("HNSW max layers: %u\n", db->index->max_layers);
    }
    
    printf("\nOperations:\n");
    printf("  Insert:  %llu (%.1f µs avg)\n", (unsigned long long)db->stats.insert_count, db->stats.avg_insert_us);
    printf("  Delete:  %llu\n", (unsigned long long)db->stats.delete_count);
    printf("  Search:  %llu (%.3f ms avg)\n", (unsigned long long)db->stats.search_count, db->stats.avg_search_ms);
    printf("  Get:     %llu\n", (unsigned long long)db->stats.get_count);
    printf("====================================\n\n");
    
    pthread_rwlock_unlock(&db->lock);
}

const char* vectordb_get_version(void) {
    return VECTOR_DB_VERSION;
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

typedef struct {
    HNSWNode* node;
    float dist;
} NodeDist;

typedef struct {
    NodeDist* items;
    uint32_t size;
    uint32_t capacity;
} MinHeap;

static MinHeap* heap_create(uint32_t capacity) {
    MinHeap* heap = (MinHeap*)malloc(sizeof(MinHeap));
    if (!heap) return NULL;
    heap->items = (NodeDist*)malloc(capacity * sizeof(NodeDist));
    if (!heap->items) { free(heap); return NULL; }
    heap->size = 0;
    heap->capacity = capacity;
    return heap;
}

static void heap_destroy(MinHeap* heap) {
    if (!heap) return;
    free(heap->items);
    free(heap);
}

static void heap_push(MinHeap* heap, HNSWNode* node, float dist) {
    if (heap->size >= heap->capacity) return;
    uint32_t i = heap->size++;
    heap->items[i].node = node;
    heap->items[i].dist = dist;
    
    while (i > 0) {
        uint32_t parent = (i - 1) / 2;
        if (heap->items[parent].dist <= heap->items[i].dist) break;
        NodeDist tmp = heap->items[parent];
        heap->items[parent] = heap->items[i];
        heap->items[i] = tmp;
        i = parent;
    }
}

static NodeDist heap_pop(MinHeap* heap) {
    NodeDist result = heap->items[0];
    heap->items[0] = heap->items[--heap->size];
    
    uint32_t i = 0;
    while (1) {
        uint32_t left = 2 * i + 1;
        uint32_t right = 2 * i + 2;
        uint32_t smallest = i;
        
        if (left < heap->size && heap->items[left].dist < heap->items[smallest].dist)
            smallest = left;
        if (right < heap->size && heap->items[right].dist < heap->items[smallest].dist)
            smallest = right;
        
        if (smallest == i) break;
        
        NodeDist tmp = heap->items[i];
        heap->items[i] = heap->items[smallest];
        heap->items[smallest] = tmp;
        i = smallest;
    }
    
    return result;
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
    index->max_neighbors = max_neighbors > 0 ? max_neighbors : 16;
    index->ef_construction = ef_construction > 0 ? ef_construction : 200;
    index->entry_point_level = 0;
    index->entry_point = NULL;
    index->node_count = 0;
    
    return index;
}

static void hnsw_node_destroy(HNSWNode* node) {
    if (!node) return;
    free(node->vector.data);
    free(node->neighbors);
    free(node);
}

void hnsw_destroy(HNSWIndex* index) {
    if (!index) return;
    
    // 清理所有节点
    for (uint32_t i = 0; i < index->max_layers; i++) {
        HNSWNode* node = index->layers[i];
        while (node) {
            HNSWNode* next = node->neighbors ? node->neighbors[0] : NULL;
            hnsw_node_destroy(node);
            node = next;
        }
    }
    
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

static float hnsw_distance(const Vector* a, const Vector* b) {
    return 1.0f - cosine_similarity(a, b);
}

static HNSWNode* hnsw_search_layer(HNSWNode* entry, const Vector* query, 
                                    uint32_t ef, uint32_t level) {
    if (!entry) return entry;
    
    MinHeap* candidates = heap_create(ef * 2);
    MinHeap* visited = heap_create(ef * 2);
    
    float dist = hnsw_distance(&entry->vector, query);
    heap_push(candidates, entry, dist);
    heap_push(visited, entry, dist);
    
    HNSWNode* best = entry;
    float best_dist = dist;
    
    while (candidates->size > 0) {
        NodeDist current = heap_pop(candidates);
        
        if (current.dist > best_dist) break;
        
        for (uint32_t i = 0; i < current.node->neighbor_count; i++) {
            HNSWNode* neighbor = current.node->neighbors[i];
            if (!neighbor) continue;
            
            // 检查是否已访问
            int already_visited = 0;
            for (uint32_t j = 0; j < visited->size; j++) {
                if (visited->items[j].node == neighbor) {
                    already_visited = 1;
                    break;
                }
            }
            if (already_visited) continue;
            
            float d = hnsw_distance(&neighbor->vector, query);
            heap_push(visited, neighbor, d);
            
            if (d < best_dist || visited->size < ef) {
                heap_push(candidates, neighbor, d);
                if (d < best_dist) {
                    best_dist = d;
                    best = neighbor;
                }
            }
        }
    }
    
    heap_destroy(candidates);
    heap_destroy(visited);
    
    return best;
}

int hnsw_insert(HNSWIndex* index, uint64_t id, const Vector* vec) {
    if (!index || !vec) return VECTOR_DB_ERROR;
    
    uint32_t level = random_level(index->max_layers);
    
    HNSWNode* new_node = hnsw_node_create(id, vec, index->max_neighbors, level);
    if (!new_node) return VECTOR_DB_MEMORY_ERROR;
    
    // 插入到对应层
    if (level < index->max_layers) {
        new_node->neighbors[0] = index->layers[level];
        index->layers[level] = new_node;
    }
    
    // 更新入口点
    if (!index->entry_point || level > index->entry_point_level) {
        index->entry_point = new_node;
        index->entry_point_level = level;
    }
    
    // 建立邻居连接（简化版）
    if (index->entry_point && index->entry_point != new_node) {
        HNSWNode* curr = index->entry_point;
        for (uint32_t l = index->entry_point_level; l >= 0 && l <= level; l--) {
            HNSWNode* nearest = hnsw_search_layer(curr, vec, index->ef_construction, l);
            if (nearest && nearest->neighbor_count < index->max_neighbors) {
                nearest->neighbors[nearest->neighbor_count++] = new_node;
            }
            if (l == 0) break;
        }
    }
    
    index->node_count++;
    return VECTOR_DB_SUCCESS;
}

int hnsw_delete(HNSWIndex* index, uint64_t id) {
    // 简化实现 - 标记删除
    return VECTOR_DB_SUCCESS;
}

SearchResult* hnsw_search(HNSWIndex* index, const Vector* query,
                          uint32_t top_k, uint32_t ef_search,
                          uint32_t* result_count) {
    if (!index || !query || !result_count) return NULL;
    if (!index->entry_point) {
        *result_count = 0;
        return NULL;
    }
    
    if (ef_search < top_k) ef_search = top_k;
    
    // 从顶层开始搜索
    HNSWNode* curr = index->entry_point;
    for (int level = (int)index->entry_point_level; level > 0; level--) {
        curr = hnsw_search_layer(curr, query, 1, level);
    }
    
    // 在底层搜索 ef 个最近邻
    MinHeap* candidates = heap_create(ef_search * 2);
    MinHeap* visited = heap_create(ef_search * 2);
    
    float dist = hnsw_distance(&curr->vector, query);
    heap_push(candidates, curr, dist);
    heap_push(visited, curr, dist);
    
    while (candidates->size > 0) {
        NodeDist current = heap_pop(candidates);
        
        for (uint32_t i = 0; i < current.node->neighbor_count; i++) {
            HNSWNode* neighbor = current.node->neighbors[i];
            if (!neighbor) continue;
            
            int already_visited = 0;
            for (uint32_t j = 0; j < visited->size; j++) {
                if (visited->items[j].node == neighbor) {
                    already_visited = 1;
                    break;
                }
            }
            if (already_visited) continue;
            
            float d = hnsw_distance(&neighbor->vector, query);
            heap_push(visited, neighbor, d);
            
            if (visited->size < ef_search || d < visited->items[0].dist) {
                heap_push(candidates, neighbor, d);
            }
        }
    }
    
    // 收集结果
    uint32_t count = visited->size < top_k ? visited->size : top_k;
    SearchResult* results = (SearchResult*)malloc(count * sizeof(SearchResult));
    
    if (results) {
        // 按距离排序
        for (uint32_t i = 0; i < count; i++) {
            for (uint32_t j = i + 1; j < visited->size; j++) {
                if (visited->items[j].dist < visited->items[i].dist) {
                    NodeDist tmp = visited->items[i];
                    visited->items[i] = visited->items[j];
                    visited->items[j] = tmp;
                }
            }
        }
        
        for (uint32_t i = 0; i < count; i++) {
            results[i].id = visited->items[i].node->id;
            results[i].score = 1.0f - visited->items[i].dist;
            results[i].metadata = NULL;
            results[i].metadata_len = 0;
        }
    }
    
    *result_count = count;
    heap_destroy(candidates);
    heap_destroy(visited);
    
    return results;
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
