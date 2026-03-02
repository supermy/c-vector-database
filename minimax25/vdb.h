#ifndef VECTOR_DATABASE_H
#define VECTOR_DATABASE_H

#define _POSIX_C_SOURCE 200809L
#define _DEFAULT_SOURCE

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>
#include <pthread.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Platform detection for better compatibility */
#if defined(__APPLE__)
    #include <TargetConditionals.h>
    #define VDB_PLATFORM_MACOS 1
#elif defined(__linux__)
    #define VDB_PLATFORM_LINUX 1
#elif defined(_WIN32)
    #define VDB_PLATFORM_WINDOWS 1
#endif

#define VDB_VERSION "1.3.0-production"
#define VDB_OK              0
#define VDB_ERROR           -1
#define VDB_NOT_FOUND       -2
#define VDB_OUT_OF_MEMORY   -3
#define VDB_INVALID_DIM     -4
#define VDB_DUPLICATE_ID    -5
#define VDB_OBJECT_POOL_SIZE 256

typedef struct Vector {
    float* data;
    uint32_t dim;
} Vector;

typedef struct {
    uint64_t id;
    Vector vector;
    void* metadata;
    uint32_t metadata_size;
} VectorEntry;

typedef struct VectorDatabase VectorDatabase;

typedef struct {
    uint64_t id;
    float distance;
    void* metadata;
    uint32_t metadata_size;
} SearchResult;

typedef enum {
    DISTANCE_COSINE,
    DISTANCE_EUCLIDEAN,
    DISTANCE_DOT_PRODUCT
} DistanceMetric;

typedef struct {
    uint32_t top_k;
    float max_distance;
    DistanceMetric metric;
    bool use_index;
    uint32_t ef_search;
} SearchOptions;

typedef struct {
    void** objects;
    uint32_t size;
    uint32_t capacity;
    size_t object_size;
} VDBObjectPool;

typedef struct {
    uint64_t insert_count;
    uint64_t delete_count;
    uint64_t search_count;
    uint64_t get_count;
    double avg_insert_us;
    double avg_search_ms;
} VDBStats;

Vector* vector_new(uint32_t dim);
void vector_free(Vector* vec);
int vector_set(Vector* vec, const float* data);
int vector_copy(const Vector* src, Vector* dst);
float vector_magnitude(const Vector* vec);
float vector_dot_product(const Vector* a, const Vector* b);
float vector_cosine_similarity(const Vector* a, const Vector* b);
float vector_euclidean_distance(const Vector* a, const Vector* b);
float vector_distance(const Vector* a, const Vector* b, DistanceMetric metric);

VectorDatabase* vdb_create(uint32_t dimension);
void vdb_free(VectorDatabase* db);
int vdb_insert(VectorDatabase* db, uint64_t id, const Vector* vec, 
              const void* metadata, uint32_t metadata_size);
int vdb_delete(VectorDatabase* db, uint64_t id);
int vdb_update(VectorDatabase* db, uint64_t id, const Vector* vec,
               const void* metadata, uint32_t metadata_size);
VectorEntry* vdb_get(VectorDatabase* db, uint64_t id);

SearchResult* vdb_search(VectorDatabase* db, const Vector* query,
                        const SearchOptions* options, uint32_t* result_count);
void vdb_free_results(SearchResult* results, uint32_t count);

void vdb_set_index(VectorDatabase* db, bool enable);
int vdb_build_index(VectorDatabase* db);
void vdb_stats(const VectorDatabase* db);

int vdb_save(const VectorDatabase* db, const char* filename);
VectorDatabase* vdb_load(const char* filename);

int vdb_build_ivf_index(VectorDatabase* db, uint32_t num_clusters);
SearchResult* vdb_search_ivf(VectorDatabase* db, const Vector* query,
                             const SearchOptions* options, uint32_t* result_count);
void vdb_free_ivf_index(VectorDatabase* db);

SearchResult* vdb_batch_search(VectorDatabase* db, const Vector** queries,
                                uint32_t num_queries, const SearchOptions* options,
                                uint32_t* result_counts);

VDBObjectPool* vdb_pool_create(size_t object_size, uint32_t capacity);
void vdb_pool_destroy(VDBObjectPool* pool);
void* vdb_pool_alloc(VDBObjectPool* pool);
void vdb_pool_free(VDBObjectPool* pool, void* obj);

void vdb_enable_stats(VectorDatabase* db, bool enable);
int vdb_get_stats(VectorDatabase* db, VDBStats* stats);
void vdb_reset_stats(VectorDatabase* db);
void vdb_print_stats(VectorDatabase* db);

const char* vdb_get_version(void);

#ifdef __cplusplus
}
#endif

#endif
