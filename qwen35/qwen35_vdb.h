#ifndef QWEN35_VDB_H
#define QWEN35_VDB_H

#include <stdint.h>
#include <stddef.h>
#include <pthread.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Platform detection for better compatibility */
#if defined(__APPLE__)
    #include <TargetConditionals.h>
    #define QWEN35_PLATFORM_MACOS 1
#elif defined(__linux__)
    #define QWEN35_PLATFORM_LINUX 1
#elif defined(_WIN32)
    #define QWEN35_PLATFORM_WINDOWS 1
#endif

#define QWEN35_VDB_VERSION "1.2.0-production"
#define QWEN35_DEFAULT_CAPACITY 1024
#define QWEN35_DEFAULT_HASH_BUCKETS 16384
#define QWEN35_MAX_DIMENSIONS 4096
#define QWEN35_CACHE_LINE_SIZE 64
#define QWEN35_SIMD_WIDTH 16
#define QWEN35_OBJECT_POOL_SIZE 256

typedef enum {
    QWEN35_DIST_COSINE = 0,
    QWEN35_DIST_EUCLIDEAN = 1,
    QWEN35_DIST_DOT_PRODUCT = 2
} qwen35_distance_t;

typedef struct {
    int64_t id;
    float *vector;
    size_t dim;
    void *metadata;
    size_t metadata_size;
} qwen35_entry_t;

typedef struct qwen35_hash_node {
    int64_t id;
    size_t entry_index;
    struct qwen35_hash_node *next;
} qwen35_hash_node_t;

typedef struct {
    qwen35_hash_node_t **buckets;
    size_t num_buckets;
    size_t size;
} qwen35_hashmap_t;

typedef struct {
    void **objects;
    size_t size;
    size_t capacity;
    size_t object_size;
} qwen35_object_pool_t;

typedef struct {
    size_t insert_count;
    size_t delete_count;
    size_t search_count;
    size_t get_count;
    size_t cache_hits;
    size_t cache_misses;
    double avg_search_time_ms;
    double avg_insert_time_us;
} qwen35_stats_t;

typedef struct {
    qwen35_entry_t *entries;
    size_t size;
    size_t capacity;
    qwen35_hashmap_t *id_map;
    size_t dimensions;
    qwen35_distance_t distance_type;
    int is_normalized;
    pthread_rwlock_t lock;
    qwen35_object_pool_t *entry_pool;
    qwen35_stats_t stats;
    int enable_stats;
} qwen35_vector_db_t;

qwen35_vector_db_t *qwen35_db_create(size_t dimensions, qwen35_distance_t dist_type);
void qwen35_db_destroy(qwen35_vector_db_t *db);
int qwen35_db_insert(qwen35_vector_db_t *db, int64_t id, const float *vector, void *metadata, size_t metadata_size);
int qwen35_db_delete(qwen35_vector_db_t *db, int64_t id);
int qwen35_db_search(qwen35_vector_db_t *db, const float *query, size_t k, int64_t *out_ids, float *out_distances);
int qwen35_db_search_batch(qwen35_vector_db_t *db, const float **queries, size_t num_queries, 
                           size_t k, int64_t **out_ids, float **out_distances);
int qwen35_db_get(qwen35_vector_db_t *db, int64_t id, float *out_vector, void *out_metadata, size_t *out_metadata_size);
size_t qwen35_db_size(qwen35_vector_db_t *db);
int qwen35_db_save(qwen35_vector_db_t *db, const char *filename);
qwen35_vector_db_t *qwen35_db_load(const char *filename);
float qwen35_cosine_similarity(const float *a, const float *b, size_t dim);
float qwen35_euclidean_distance(const float *a, const float *b, size_t dim);
float qwen35_dot_product(const float *a, const float *b, size_t dim);
float qwen35_cosine_simd(const float *a, const float *b, size_t dim);
float qwen35_euclidean_simd(const float *a, const float *b, size_t dim);
void qwen35_normalize_vector(float *vector, size_t dim);
const char *qwen35_get_version(void);

qwen35_object_pool_t *qwen35_pool_create(size_t object_size, size_t capacity);
void qwen35_pool_destroy(qwen35_object_pool_t *pool);
void *qwen35_pool_alloc(qwen35_object_pool_t *pool);
void qwen35_pool_free(qwen35_object_pool_t *pool, void *obj);

void qwen35_db_enable_stats(qwen35_vector_db_t *db, int enable);
int qwen35_db_get_stats(qwen35_vector_db_t *db, qwen35_stats_t *stats);
void qwen35_db_reset_stats(qwen35_vector_db_t *db);
void qwen35_db_print_stats(qwen35_vector_db_t *db);

#ifdef __cplusplus
}
#endif

#endif /* QWEN35_VDB_H */
