#ifndef QWEN35_VDB_H
#define QWEN35_VDB_H

#include <stdint.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

#define QWEN35_VDB_VERSION "1.0.0"
#define QWEN35_DEFAULT_CAPACITY 1024
#define QWEN35_DEFAULT_HASH_BUCKETS 16384
#define QWEN35_MAX_DIMENSIONS 4096

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
    qwen35_entry_t *entries;
    size_t size;
    size_t capacity;
    qwen35_hashmap_t *id_map;
    size_t dimensions;
    qwen35_distance_t distance_type;
    int is_normalized;
} qwen35_vector_db_t;

qwen35_vector_db_t *qwen35_db_create(size_t dimensions, qwen35_distance_t dist_type);
void qwen35_db_destroy(qwen35_vector_db_t *db);
int qwen35_db_insert(qwen35_vector_db_t *db, int64_t id, const float *vector, void *metadata, size_t metadata_size);
int qwen35_db_delete(qwen35_vector_db_t *db, int64_t id);
int qwen35_db_search(qwen35_vector_db_t *db, const float *query, size_t k, int64_t *out_ids, float *out_distances);
int qwen35_db_get(qwen35_vector_db_t *db, int64_t id, float *out_vector, void *out_metadata, size_t *out_metadata_size);
size_t qwen35_db_size(qwen35_vector_db_t *db);
int qwen35_db_save(qwen35_vector_db_t *db, const char *filename);
qwen35_vector_db_t *qwen35_db_load(const char *filename);
float qwen35_cosine_similarity(const float *a, const float *b, size_t dim);
float qwen35_euclidean_distance(const float *a, const float *b, size_t dim);
float qwen35_dot_product(const float *a, const float *b, size_t dim);
void qwen35_normalize_vector(float *vector, size_t dim);
const char *qwen35_get_version(void);

#ifdef __cplusplus
}
#endif

#endif /* QWEN35_VDB_H */
