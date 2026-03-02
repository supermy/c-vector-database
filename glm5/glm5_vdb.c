#include "glm5_vdb.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <math.h>
#include <time.h>

// Windows needs malloc.h for _aligned_malloc
#if defined(_WIN32) || defined(_WIN64)
#include <malloc.h>
#endif

#define INIT_CAP 4096
#define HASH_BUCKETS 16384
#define MEM_POOL_BLOCK_SIZE (1024 * 1024)

// Cross-platform aligned memory allocation
static inline void* glm5_aligned_alloc(size_t alignment, size_t size) {
#if defined(_WIN32) || defined(_WIN64)
    return _aligned_malloc(size, alignment);
#else
    return aligned_alloc(alignment, size);
#endif
}

static inline void glm5_aligned_free(void* ptr) {
#if defined(_WIN32) || defined(_WIN64)
    _aligned_free(ptr);
#else
    free(ptr);
#endif
}

typedef struct HashNode {
    uint64_t key;
    VecEntry* val;
    struct HashNode* next;
} HashNode;

typedef struct MemPool {
    uint8_t* block;
    size_t offset;
    size_t size;
    struct MemPool* next;
} MemPool;

typedef struct Cluster {
    Vector center;
    uint64_t* entry_indices;
    uint64_t count;
    uint64_t capacity;
} Cluster;

struct VecDB {
    VecEntry* entries;
    uint64_t cnt;
    uint64_t cap;
    uint32_t dim;
    HashNode** buckets;
    uint64_t bucket_cnt;
    MemPool* pool;
    Cluster* clusters;
    uint32_t num_clusters;
    int index_built;
#ifndef GLM5_NO_PTHREAD
    pthread_rwlock_t lock;
#endif
    GLM5ObjectPool* obj_pool;
    GLM5Stats stats;
    bool enable_stats;
};

GLM5ObjectPool* glm5_pool_create(size_t object_size, uint32_t capacity) {
    GLM5ObjectPool* pool = (GLM5ObjectPool*)malloc(sizeof(GLM5ObjectPool));
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

void glm5_pool_destroy(GLM5ObjectPool* pool) {
    if (!pool) return;
    for (uint32_t i = 0; i < pool->capacity; i++) {
        if (pool->objects[i]) free(pool->objects[i]);
    }
    free(pool->objects);
    free(pool);
}

void* glm5_pool_alloc(GLM5ObjectPool* pool) {
    if (!pool || pool->size == 0) return NULL;
    return pool->objects[--pool->size];
}

void glm5_pool_free(GLM5ObjectPool* pool, void* obj) {
    if (!pool || !obj || pool->size >= pool->capacity) return;
    pool->objects[pool->size++] = obj;
}

static MemPool* pool_create(size_t size) {
    MemPool* p = (MemPool*)malloc(sizeof(MemPool));
    if (!p) return NULL;
    p->block = (uint8_t*)glm5_aligned_alloc(GLM5_ALIGN_SIZE, size);
    if (!p->block) { free(p); return NULL; }
    p->offset = 0;
    p->size = size;
    p->next = NULL;
    return p;
}

static void* pool_alloc(MemPool** pool, size_t size) {
    size = (size + GLM5_ALIGN_SIZE - 1) & ~(GLM5_ALIGN_SIZE - 1);
    
    if (!*pool || (*pool)->offset + size > (*pool)->size) {
        size_t block_size = size > MEM_POOL_BLOCK_SIZE ? size : MEM_POOL_BLOCK_SIZE;
        MemPool* new_pool = pool_create(block_size);
        if (!new_pool) return NULL;
        new_pool->next = *pool;
        *pool = new_pool;
    }
    
    void* ptr = (*pool)->block + (*pool)->offset;
    (*pool)->offset += size;
    return ptr;
}

static void pool_destroy(MemPool* pool) {
    while (pool) {
        MemPool* next = pool->next;
        glm5_aligned_free(pool->block);
        free(pool);
        pool = next;
    }
}

static uint64_t hash64(uint64_t x) {
    x = (x ^ (x >> 30)) * 0xbf58476d1ce4e5b9ULL;
    x = (x ^ (x >> 27)) * 0x94d049bb133111ebULL;
    x = x ^ (x >> 31);
    return x;
}

static HashNode** hash_init(uint64_t n) {
    HashNode** b = (HashNode**)calloc(n, sizeof(HashNode*));
    return b;
}

static void hash_free(HashNode** b, uint64_t n) {
    for (uint64_t i = 0; i < n; i++) {
        HashNode* p = b[i];
        while (p) {
            HashNode* t = p->next;
            free(p);
            p = t;
        }
    }
    free(b);
}

static int hash_put(HashNode** b, uint64_t n, uint64_t k, VecEntry* v) {
    uint64_t i = hash64(k) % n;
    HashNode* p = b[i];
    while (p) {
        if (p->key == k) { p->val = v; return GLM5_VDB_OK; }
        p = p->next;
    }
    HashNode* node = (HashNode*)malloc(sizeof(HashNode));
    if (!node) return GLM5_VDB_NO_MEMORY;
    node->key = k;
    node->val = v;
    node->next = b[i];
    b[i] = node;
    return GLM5_VDB_OK;
}

static VecEntry* hash_get(HashNode** b, uint64_t n, uint64_t k) {
    uint64_t i = hash64(k) % n;
    HashNode* p = b[i];
    while (p) {
        if (p->key == k) return p->val;
        p = p->next;
    }
    return NULL;
}

static int hash_del(HashNode** b, uint64_t n, uint64_t k) {
    uint64_t i = hash64(k) % n;
    HashNode* p = b[i], *prev = NULL;
    while (p) {
        if (p->key == k) {
            if (prev) prev->next = p->next;
            else b[i] = p->next;
            free(p);
            return GLM5_VDB_OK;
        }
        prev = p;
        p = p->next;
    }
    return GLM5_VDB_NOT_FOUND;
}

Vector* vec_new(uint32_t dim) {
    if (dim == 0) return NULL;
    Vector* v = (Vector*)malloc(sizeof(Vector));
    if (!v) return NULL;
    v->values = (float*)calloc(dim, sizeof(float));
    if (!v->values) { free(v); return NULL; }
    v->dimension = dim;
    return v;
}

Vector* vec_new_aligned(uint32_t dim) {
    if (dim == 0) return NULL;
    Vector* v = (Vector*)glm5_aligned_alloc(GLM5_ALIGN_SIZE, sizeof(Vector));
    if (!v) return NULL;
    size_t alloc_size = ((dim * sizeof(float)) + GLM5_ALIGN_SIZE - 1) & ~(GLM5_ALIGN_SIZE - 1);
    v->values = (float*)glm5_aligned_alloc(GLM5_ALIGN_SIZE, alloc_size);
    if (!v->values) { free(v); return NULL; }
    memset(v->values, 0, dim * sizeof(float));
    v->dimension = dim;
    return v;
}

void vec_free(Vector* v) {
    if (v) {
        glm5_aligned_free(v->values);
        glm5_aligned_free(v);
    }
}

int vec_copy_from(Vector* dst, const float* src, uint32_t dim) {
    if (!dst || !src || dim != dst->dimension) return GLM5_VDB_INVALID_PARAM;
    memcpy(dst->values, src, dim * sizeof(float));
    return GLM5_VDB_OK;
}

int vec_clone(const Vector* src, Vector* dst) {
    if (!src || !dst || src->dimension != dst->dimension) return GLM5_VDB_INVALID_PARAM;
    memcpy(dst->values, src->values, src->dimension * sizeof(float));
    return GLM5_VDB_OK;
}

float vec_dot(const Vector* a, const Vector* b) {
    if (!a || !b || a->dimension != b->dimension) return 0.0f;
    float s = 0.0f;
    for (uint32_t i = 0; i < a->dimension; i++) {
        s += a->values[i] * b->values[i];
    }
    return s;
}

float vec_l2(const Vector* a, const Vector* b) {
    if (!a || !b || a->dimension != b->dimension) return -1.0f;
    float s = 0.0f;
    for (uint32_t i = 0; i < a->dimension; i++) {
        float d = a->values[i] - b->values[i];
        s += d * d;
    }
    return sqrtf(s);
}

float vec_norm(const Vector* v) {
    if (!v) return 0.0f;
    float s = 0.0f;
    for (uint32_t i = 0; i < v->dimension; i++) {
        s += v->values[i] * v->values[i];
    }
    return sqrtf(s);
}

float vec_cosine(const Vector* a, const Vector* b) {
    if (!a || !b || a->dimension != b->dimension) return -1.0f;
    float na = vec_norm(a), nb = vec_norm(b);
    if (na < 1e-10f || nb < 1e-10f) return -1.0f;
    return vec_dot(a, b) / (na * nb);
}

void vec_normalize(Vector* v) {
    if (!v || !v->values) return;
    float n = vec_norm(v);
    if (n > 1e-10f) {
        for (uint32_t i = 0; i < v->dimension; i++) {
            v->values[i] /= n;
        }
    }
}

float vec_cosine_normalized(const Vector* a, const Vector* b) {
    if (!a || !b || a->dimension != b->dimension) return -1.0f;
    return vec_dot(a, b);
}

float vec_distance(const Vector* a, const Vector* b, DistanceMetric m) {
    switch (m) {
        case METRIC_COSINE: return 1.0f - vec_cosine(a, b);
        case METRIC_EUCLIDEAN: return vec_l2(a, b);
        case METRIC_DOT: return -vec_dot(a, b);
        default: return vec_l2(a, b);
    }
}

VecDB* vdb_new(uint32_t dim) {
    if (dim == 0) return NULL;
    VecDB* db = (VecDB*)malloc(sizeof(VecDB));
    if (!db) return NULL;
    
    db->entries = (VecEntry*)calloc(INIT_CAP, sizeof(VecEntry));
    if (!db->entries) { free(db); return NULL; }
    
    db->buckets = hash_init(HASH_BUCKETS);
    if (!db->buckets) { free(db->entries); free(db); return NULL; }
    
#ifndef GLM5_NO_PTHREAD
    pthread_rwlock_init(&db->lock, NULL);
#endif
    db->obj_pool = glm5_pool_create(sizeof(VecEntry), GLM5_OBJECT_POOL_SIZE);
    
    db->cnt = 0;
    db->cap = INIT_CAP;
    db->dim = dim;
    db->bucket_cnt = HASH_BUCKETS;
    db->pool = NULL;
    db->clusters = NULL;
    db->num_clusters = 0;
    db->index_built = 0;
    db->enable_stats = true;
    memset(&db->stats, 0, sizeof(GLM5Stats));
    
    return db;
}

static void free_entry(VecEntry* e) {
    if (e) {
        free(e->vec.values);
        free(e->meta);
    }
}

void vdb_free(VecDB* db) {
    if (!db) return;
    
#ifndef GLM5_NO_PTHREAD
    pthread_rwlock_destroy(&db->lock);
#endif
    
    for (uint64_t i = 0; i < db->cnt; i++) {
        free_entry(&db->entries[i]);
    }
    free(db->entries);
    hash_free(db->buckets, db->bucket_cnt);
    pool_destroy(db->pool);
    if (db->obj_pool) glm5_pool_destroy(db->obj_pool);
    if (db->clusters) {
        for (uint32_t i = 0; i < db->num_clusters; i++) {
            free(db->clusters[i].center.values);
            free(db->clusters[i].entry_indices);
        }
        free(db->clusters);
    }
    free(db);
}

static int ensure_cap(VecDB* db) {
    if (db->cnt < db->cap) return GLM5_VDB_OK;
    uint64_t new_cap = db->cap * 2;
    VecEntry* new_e = (VecEntry*)realloc(db->entries, new_cap * sizeof(VecEntry));
    if (!new_e) return GLM5_VDB_NO_MEMORY;
    db->entries = new_e;
    db->cap = new_cap;
    return GLM5_VDB_OK;
}

int vdb_add(VecDB* db, uint64_t id, const Vector* v, const void* meta, uint32_t meta_len) {
    if (!db || !v) return GLM5_VDB_ERR;
    if (v->dimension != db->dim) return GLM5_VDB_INVALID_DIM;
    if (hash_get(db->buckets, db->bucket_cnt, id)) return GLM5_VDB_DUP_ID;
    
    int r = ensure_cap(db);
    if (r != GLM5_VDB_OK) return r;
    
    VecEntry* e = &db->entries[db->cnt];
    e->id = id;
    e->vec.dimension = v->dimension;
    e->vec.values = (float*)malloc(v->dimension * sizeof(float));
    if (!e->vec.values) return GLM5_VDB_NO_MEMORY;
    memcpy(e->vec.values, v->values, v->dimension * sizeof(float));
    
    if (meta && meta_len > 0) {
        e->meta = malloc(meta_len);
        if (!e->meta) { free(e->vec.values); return GLM5_VDB_NO_MEMORY; }
        memcpy(e->meta, meta, meta_len);
        e->meta_len = meta_len;
    } else {
        e->meta = NULL;
        e->meta_len = 0;
    }
    
    hash_put(db->buckets, db->bucket_cnt, id, e);
    db->cnt++;
    
    return GLM5_VDB_OK;
}

int vdb_del(VecDB* db, uint64_t id) {
    if (!db) return GLM5_VDB_ERR;
    
    VecEntry* e = hash_get(db->buckets, db->bucket_cnt, id);
    if (!e) return GLM5_VDB_NOT_FOUND;
    
    uint64_t idx = e - db->entries;
    free_entry(e);
    
    if (idx < db->cnt - 1) {
        memmove(&db->entries[idx], &db->entries[idx + 1], 
                (db->cnt - idx - 1) * sizeof(VecEntry));
        for (uint64_t i = idx; i < db->cnt - 1; i++) {
            hash_put(db->buckets, db->bucket_cnt, db->entries[i].id, &db->entries[i]);
        }
    }
    
    hash_del(db->buckets, db->bucket_cnt, id);
    db->cnt--;
    
    return GLM5_VDB_OK;
}

int vdb_set(VecDB* db, uint64_t id, const Vector* v, const void* meta, uint32_t meta_len) {
    if (!db) return GLM5_VDB_ERR;
    
    VecEntry* e = hash_get(db->buckets, db->bucket_cnt, id);
    if (!e) return GLM5_VDB_NOT_FOUND;
    
    if (v) {
        if (v->dimension != db->dim) return GLM5_VDB_INVALID_DIM;
        free(e->vec.values);
        e->vec.values = (float*)malloc(v->dimension * sizeof(float));
        if (!e->vec.values) return GLM5_VDB_NO_MEMORY;
        memcpy(e->vec.values, v->values, v->dimension * sizeof(float));
        e->vec.dimension = v->dimension;
    }
    
    if (meta && meta_len > 0) {
        free(e->meta);
        e->meta = malloc(meta_len);
        if (!e->meta) return GLM5_VDB_NO_MEMORY;
        memcpy(e->meta, meta, meta_len);
        e->meta_len = meta_len;
    }
    
    return GLM5_VDB_OK;
}

VecEntry* vdb_get(VecDB* db, uint64_t id) {
    if (!db) return NULL;
    return hash_get(db->buckets, db->bucket_cnt, id);
}

uint64_t vdb_count(VecDB* db) {
    return db ? db->cnt : 0;
}

static int cmp_result(const void* a, const void* b) {
    float da = ((const QueryResult*)a)->dist;
    float db = ((const QueryResult*)b)->dist;
    return (da < db) ? -1 : ((da > db) ? 1 : 0);
}

QueryResult* vdb_query(VecDB* db, const Vector* q, const QueryOpts* opts, uint32_t* n) {
    if (!db || !q || !n) return NULL;
    if (q->dimension != db->dim) return NULL;
    if (db->cnt == 0) { *n = 0; return NULL; }
    
    uint32_t k = opts ? opts->k : 10;
    float radius = opts ? opts->radius : 1e9f;
    DistanceMetric m = opts ? opts->metric : METRIC_COSINE;
    
    QueryResult* r = (QueryResult*)malloc(db->cnt * sizeof(QueryResult));
    if (!r) return NULL;
    
    uint32_t cnt = 0;
    for (uint64_t i = 0; i < db->cnt; i++) {
        float d = vec_distance(q, &db->entries[i].vec, m);
        if (d <= radius) {
            r[cnt].id = db->entries[i].id;
            r[cnt].dist = d;
            if (db->entries[i].meta && db->entries[i].meta_len > 0) {
                r[cnt].meta = malloc(db->entries[i].meta_len);
                if (r[cnt].meta) {
                    memcpy(r[cnt].meta, db->entries[i].meta, db->entries[i].meta_len);
                    r[cnt].meta_len = db->entries[i].meta_len;
                }
            } else {
                r[cnt].meta = NULL;
                r[cnt].meta_len = 0;
            }
            cnt++;
        }
    }
    
    qsort(r, cnt, sizeof(QueryResult), cmp_result);
    
    if (cnt > k) {
        for (uint32_t i = k; i < cnt; i++) free(r[i].meta);
        cnt = k;
    }
    
    *n = cnt;
    return r;
}

void vdb_enable_stats(VecDB* db, bool enable) {
    if (!db) return;
    db->enable_stats = enable;
}

int vdb_get_stats(VecDB* db, GLM5Stats* stats) {
    if (!db || !stats) return GLM5_VDB_ERR;
#ifndef GLM5_NO_PTHREAD
    pthread_rwlock_rdlock(&db->lock);
#endif
    memcpy(stats, &db->stats, sizeof(GLM5Stats));
#ifndef GLM5_NO_PTHREAD
    pthread_rwlock_unlock(&db->lock);
#endif
    return GLM5_VDB_OK;
}

void vdb_reset_stats(VecDB* db) {
    if (!db) return;
#ifndef GLM5_NO_PTHREAD
    pthread_rwlock_wrlock(&db->lock);
#endif
    memset(&db->stats, 0, sizeof(GLM5Stats));
#ifndef GLM5_NO_PTHREAD
    pthread_rwlock_unlock(&db->lock);
#endif
}

void vdb_print_stats(VecDB* db) {
    if (!db) return;
#ifndef GLM5_NO_PTHREAD
    pthread_rwlock_rdlock(&db->lock);
#endif
    
    printf("\n=== GLM5 VectorDB Statistics ===\n");
    printf("Version: %s\n", GLM5_VDB_VERSION);
    printf("Dimensions: %u\n", db->dim);
    printf("Size: %llu / %llu\n", (unsigned long long)db->cnt, (unsigned long long)db->cap);
    printf("Hash Buckets: %llu\n", (unsigned long long)db->bucket_cnt);
    printf("Index Built: %s\n", db->index_built ? "yes" : "no");
    printf("Clusters: %u\n", db->num_clusters);
    printf("\nOperations:\n");
    printf("  Insert:  %llu (%.1f µs avg)\n", (unsigned long long)db->stats.insert_count, db->stats.avg_insert_us);
    printf("  Delete:  %llu\n", (unsigned long long)db->stats.delete_count);
    printf("  Query:   %llu (%.3f ms avg)\n", (unsigned long long)db->stats.query_count, db->stats.avg_query_ms);
    printf("  Get:     %llu\n", (unsigned long long)db->stats.get_count);
    printf("================================\n\n");
    
#ifndef GLM5_NO_PTHREAD
    pthread_rwlock_unlock(&db->lock);
#endif
}

const char* glm5_get_version(void) {
    return GLM5_VDB_VERSION;
}

void vdb_free_results(QueryResult* r, uint32_t n) {
    if (!r) return;
    for (uint32_t i = 0; i < n; i++) free(r[i].meta);
    free(r);
}

int vdb_save(VecDB* db, const char* path) {
    if (!db || !path) return GLM5_VDB_ERR;
    
    FILE* f = fopen(path, "wb");
    if (!f) return GLM5_VDB_ERR;
    
    fwrite(&db->dim, sizeof(uint32_t), 1, f);
    fwrite(&db->cnt, sizeof(uint64_t), 1, f);
    
    for (uint64_t i = 0; i < db->cnt; i++) {
        fwrite(&db->entries[i].id, sizeof(uint64_t), 1, f);
        fwrite(&db->entries[i].meta_len, sizeof(uint32_t), 1, f);
        fwrite(db->entries[i].vec.values, sizeof(float), db->dim, f);
        if (db->entries[i].meta_len > 0) {
            fwrite(db->entries[i].meta, 1, db->entries[i].meta_len, f);
        }
    }
    
    fclose(f);
    return GLM5_VDB_OK;
}

VecDB* vdb_load(const char* path) {
    if (!path) return NULL;
    
    FILE* f = fopen(path, "rb");
    if (!f) return NULL;
    
    uint32_t dim;
    uint64_t cnt;
    
    if (fread(&dim, sizeof(uint32_t), 1, f) != 1) { fclose(f); return NULL; }
    fread(&cnt, sizeof(uint64_t), 1, f);
    
    VecDB* db = vdb_new(dim);
    if (!db) { fclose(f); return NULL; }
    
    for (uint64_t i = 0; i < cnt; i++) {
        uint64_t id;
        uint32_t meta_len;
        fread(&id, sizeof(uint64_t), 1, f);
        fread(&meta_len, sizeof(uint32_t), 1, f);
        
        Vector* v = vec_new(dim);
        if (!v) { vdb_free(db); fclose(f); return NULL; }
        fread(v->values, sizeof(float), dim, f);
        
        void* meta = NULL;
        if (meta_len > 0) {
            meta = malloc(meta_len);
            fread(meta, 1, meta_len, f);
        }
        
        vdb_add(db, id, v, meta, meta_len);
        vec_free(v);
        free(meta);
    }
    
    fclose(f);
    return db;
}

void vdb_info(VecDB* db) {
    if (!db) return;
    printf("GLM5 VectorDB:\n");
    printf("  Dimension: %u\n", db->dim);
    printf("  Entries: %llu\n", (unsigned long long)db->cnt);
    printf("  Capacity: %llu\n", (unsigned long long)db->cap);
    printf("  Hash Buckets: %llu\n", (unsigned long long)db->bucket_cnt);
    printf("  Index Built: %s\n", db->index_built ? "yes" : "no");
    printf("  Clusters: %u\n", db->num_clusters);
}

QueryResult* vdb_batch_query(VecDB* db, const Vector** queries, uint32_t num_queries, 
                              const QueryOpts* opts, uint32_t* counts) {
    if (!db || !queries || num_queries == 0 || !counts) return NULL;
    
    QueryResult* all_results = (QueryResult*)malloc(num_queries * sizeof(QueryResult));
    if (!all_results) return NULL;
    
    for (uint32_t i = 0; i < num_queries; i++) {
        counts[i] = 0;
        QueryResult* r = vdb_query(db, queries[i], opts, &counts[i]);
        all_results[i].id = (uint64_t)(uintptr_t)r;
        all_results[i].dist = (float)counts[i];
        all_results[i].meta = r;
        all_results[i].meta_len = counts[i];
    }
    
    return all_results;
}

void vdb_build_index(VecDB* db, uint32_t num_clusters) {
    if (!db || db->cnt == 0) return;
    if (num_clusters == 0) num_clusters = 32;
    if (num_clusters > db->cnt) num_clusters = (uint32_t)db->cnt;
    
    if (db->clusters) {
        for (uint32_t i = 0; i < db->num_clusters; i++) {
            free(db->clusters[i].center.values);
            free(db->clusters[i].entry_indices);
        }
        free(db->clusters);
    }
    
    db->num_clusters = num_clusters;
    db->clusters = (Cluster*)calloc(num_clusters, sizeof(Cluster));
    if (!db->clusters) return;
    
    for (uint32_t i = 0; i < num_clusters; i++) {
        db->clusters[i].center.dimension = db->dim;
        db->clusters[i].center.values = (float*)calloc(db->dim, sizeof(float));
        uint64_t idx = (i * db->cnt) / num_clusters;
        memcpy(db->clusters[i].center.values, db->entries[idx].vec.values, db->dim * sizeof(float));
        db->clusters[i].entry_indices = (uint64_t*)malloc(db->cnt * sizeof(uint64_t));
        db->clusters[i].count = 0;
        db->clusters[i].capacity = db->cnt;
    }
    
    for (int iter = 0; iter < 10; iter++) {
        for (uint32_t i = 0; i < num_clusters; i++) {
            db->clusters[i].count = 0;
        }
        
        for (uint64_t i = 0; i < db->cnt; i++) {
            float min_dist = 1e9f;
            uint32_t best = 0;
            
            for (uint32_t j = 0; j < num_clusters; j++) {
                float d = vec_l2(&db->entries[i].vec, &db->clusters[j].center);
                if (d < min_dist) {
                    min_dist = d;
                    best = j;
                }
            }
            
            if (db->clusters[best].count < db->clusters[best].capacity) {
                db->clusters[best].entry_indices[db->clusters[best].count++] = i;
            }
        }
        
        for (uint32_t i = 0; i < num_clusters; i++) {
            if (db->clusters[i].count > 0) {
                memset(db->clusters[i].center.values, 0, db->dim * sizeof(float));
                for (uint64_t j = 0; j < db->clusters[i].count; j++) {
                    for (uint32_t k = 0; k < db->dim; k++) {
                        db->clusters[i].center.values[k] += db->entries[db->clusters[i].entry_indices[j]].vec.values[k];
                    }
                }
                for (uint32_t k = 0; k < db->dim; k++) {
                    db->clusters[i].center.values[k] /= db->clusters[i].count;
                }
            }
        }
    }
    
    db->index_built = 1;
}

QueryResult* vdb_query_indexed(VecDB* db, const Vector* q, const QueryOpts* opts, uint32_t* n) {
    if (!db || !q || !n) return NULL;
    if (!db->index_built) return vdb_query(db, q, opts, n);
    
    uint32_t k = opts ? opts->k : 10;
    DistanceMetric m = opts ? opts->metric : METRIC_COSINE;
    
    uint32_t nprobe = db->num_clusters / 4;
    if (nprobe < 1) nprobe = 1;
    
    typedef struct { uint32_t id; float dist; } CDist;
    CDist* cd = (CDist*)malloc(db->num_clusters * sizeof(CDist));
    if (!cd) return vdb_query(db, q, opts, n);
    
    for (uint32_t i = 0; i < db->num_clusters; i++) {
        cd[i].id = i;
        cd[i].dist = vec_l2(q, &db->clusters[i].center);
    }
    
    for (uint32_t i = 0; i < nprobe && i < db->num_clusters; i++) {
        for (uint32_t j = i + 1; j < db->num_clusters; j++) {
            if (cd[j].dist < cd[i].dist) {
                CDist tmp = cd[i]; cd[i] = cd[j]; cd[j] = tmp;
            }
        }
    }
    
    uint64_t total = 0;
    for (uint32_t i = 0; i < nprobe && i < db->num_clusters; i++) {
        total += db->clusters[cd[i].id].count;
    }
    
    QueryResult* r = (QueryResult*)malloc(total * sizeof(QueryResult));
    if (!r) { free(cd); return vdb_query(db, q, opts, n); }
    
    uint32_t cnt = 0;
    for (uint32_t i = 0; i < nprobe && i < db->num_clusters; i++) {
        Cluster* c = &db->clusters[cd[i].id];
        for (uint64_t j = 0; j < c->count; j++) {
            VecEntry* e = &db->entries[c->entry_indices[j]];
            r[cnt].id = e->id;
            r[cnt].dist = vec_distance(q, &e->vec, m);
            if (e->meta && e->meta_len > 0) {
                r[cnt].meta = malloc(e->meta_len);
                if (r[cnt].meta) {
                    memcpy(r[cnt].meta, e->meta, e->meta_len);
                    r[cnt].meta_len = e->meta_len;
                }
            } else {
                r[cnt].meta = NULL;
                r[cnt].meta_len = 0;
            }
            cnt++;
        }
    }
    
    free(cd);
    
    qsort(r, cnt, sizeof(QueryResult), cmp_result);
    
    if (cnt > k) {
        for (uint32_t i = k; i < cnt; i++) free(r[i].meta);
        cnt = k;
    }
    
    *n = cnt;
    return r;
}
