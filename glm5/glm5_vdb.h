#ifndef GLM5_VECTOR_DB_H
#define GLM5_VECTOR_DB_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

#define GLM5_VDB_OK                0
#define GLM5_VDB_ERR              -1
#define GLM5_VDB_NOT_FOUND        -2
#define GLM5_VDB_NO_MEMORY        -3
#define GLM5_VDB_INVALID_DIM      -4
#define GLM5_VDB_DUP_ID           -5
#define GLM5_VDB_INVALID_PARAM    -6

#define GLM5_CACHE_LINE_SIZE      64
#define GLM5_ALIGN_SIZE           64

typedef enum {
    METRIC_COSINE,
    METRIC_EUCLIDEAN,
    METRIC_DOT
} DistanceMetric;

typedef struct {
    float* values;
    uint32_t dimension;
    uint8_t _padding[4];
} Vector;

typedef struct {
    uint64_t id;
    Vector vec;
    void* meta;
    uint32_t meta_len;
    uint8_t _padding[4];
} VecEntry;

typedef struct VecDB VecDB;

typedef struct {
    uint64_t id;
    float dist;
    void* meta;
    uint32_t meta_len;
} QueryResult;

typedef struct {
    uint32_t k;
    float radius;
    DistanceMetric metric;
    uint32_t ef;
} QueryOpts;

Vector* vec_new(uint32_t dim);
Vector* vec_new_aligned(uint32_t dim);
void vec_free(Vector* v);
int vec_copy_from(Vector* dst, const float* src, uint32_t dim);
int vec_clone(const Vector* src, Vector* dst);
void vec_normalize(Vector* v);

float vec_dot(const Vector* a, const Vector* b);
float vec_l2(const Vector* a, const Vector* b);
float vec_norm(const Vector* v);
float vec_cosine(const Vector* a, const Vector* b);
float vec_cosine_normalized(const Vector* a, const Vector* b);
float vec_distance(const Vector* a, const Vector* b, DistanceMetric m);

VecDB* vdb_new(uint32_t dim);
void vdb_free(VecDB* db);

int vdb_add(VecDB* db, uint64_t id, const Vector* v, const void* meta, uint32_t meta_len);
int vdb_del(VecDB* db, uint64_t id);
int vdb_set(VecDB* db, uint64_t id, const Vector* v, const void* meta, uint32_t meta_len);
VecEntry* vdb_get(VecDB* db, uint64_t id);
uint64_t vdb_count(VecDB* db);

QueryResult* vdb_query(VecDB* db, const Vector* q, const QueryOpts* opts, uint32_t* n);
QueryResult* vdb_batch_query(VecDB* db, const Vector** queries, uint32_t num_queries, 
                              const QueryOpts* opts, uint32_t* counts);
void vdb_free_results(QueryResult* r, uint32_t n);

int vdb_save(VecDB* db, const char* path);
VecDB* vdb_load(const char* path);

void vdb_info(VecDB* db);
void vdb_build_index(VecDB* db, uint32_t num_clusters);
QueryResult* vdb_query_indexed(VecDB* db, const Vector* q, const QueryOpts* opts, uint32_t* n);

#ifdef __cplusplus
}
#endif

#endif
