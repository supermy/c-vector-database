#include "qwen35_vdb.h"
#include <stdlib.h>
#include <string.h>
#include <math.h>
#include <stdio.h>
#include <xmmintrin.h>

static uint64_t hash_int(int64_t id) {
    uint64_t h = (uint64_t)id;
    h ^= h >> 33;
    h *= 0xff51afd7ed558ccdULL;
    h ^= h >> 33;
    h *= 0xc4ceb9fe1a85ec53ULL;
    h ^= h >> 33;
    return h;
}

static float qwen35_dot_product_simd(const float *a, const float *b, size_t dim) {
    float sum = 0.0f;
    size_t i = 0;
    
    #ifdef __AVX__
    __m128 sum_vec = _mm_setzero_ps();
    
    for (; i + 3 < dim; i += 4) {
        __m128 va = _mm_loadu_ps(&a[i]);
        __m128 vb = _mm_loadu_ps(&b[i]);
        sum_vec = _mm_add_ps(sum_vec, _mm_mul_ps(va, vb));
    }
    
    sum += _mm_cvtss_f32(sum_vec);
    float tmp[4];
    _mm_storeu_ps(tmp, sum_vec);
    sum = tmp[0] + tmp[1] + tmp[2] + tmp[3];
    #endif
    
    for (; i < dim; i++) {
        sum += a[i] * b[i];
    }
    
    return sum;
}

static float qwen35_euclidean_distance_simd(const float *a, const float *b, size_t dim) {
    float sum = 0.0f;
    size_t i = 0;
    
    #ifdef __AVX__
    __m128 sum_vec = _mm_setzero_ps();
    
    for (; i + 3 < dim; i += 4) {
        __m128 va = _mm_loadu_ps(&a[i]);
        __m128 vb = _mm_loadu_ps(&b[i]);
        __m128 diff = _mm_sub_ps(va, vb);
        sum_vec = _mm_add_ps(sum_vec, _mm_mul_ps(diff, diff));
    }
    
    float tmp[4];
    _mm_storeu_ps(tmp, sum_vec);
    sum = tmp[0] + tmp[1] + tmp[2] + tmp[3];
    #endif
    
    for (; i < dim; i++) {
        float diff = a[i] - b[i];
        sum += diff * diff;
    }
    
    return sqrtf(sum);
}

float qwen35_cosine_simd(const float *a, const float *b, size_t dim) {
    float dot = qwen35_dot_product_simd(a, b, dim);
    float norm_a = 0.0f, norm_b = 0.0f;
    
    for (size_t i = 0; i < dim; i++) {
        norm_a += a[i] * a[i];
        norm_b += b[i] * b[i];
    }
    
    if (norm_a == 0.0f || norm_b == 0.0f) {
        return 0.0f;
    }
    
    return dot / (sqrtf(norm_a) * sqrtf(norm_b));
}

float qwen35_euclidean_simd(const float *a, const float *b, size_t dim) {
    return qwen35_euclidean_distance_simd(a, b, dim);
}

static qwen35_hashmap_t *hashmap_create(size_t num_buckets) {
    qwen35_hashmap_t *map = (qwen35_hashmap_t *)malloc(sizeof(qwen35_hashmap_t));
    if (!map) return NULL;
    
    map->buckets = (qwen35_hash_node_t **)calloc(num_buckets, sizeof(qwen35_hash_node_t *));
    if (!map->buckets) {
        free(map);
        return NULL;
    }
    
    map->num_buckets = num_buckets;
    map->size = 0;
    return map;
}

static void hashmap_destroy(qwen35_hashmap_t *map) {
    if (!map) return;
    
    for (size_t i = 0; i < map->num_buckets; i++) {
        qwen35_hash_node_t *node = map->buckets[i];
        while (node) {
            qwen35_hash_node_t *next = node->next;
            free(node);
            node = next;
        }
    }
    
    free(map->buckets);
    free(map);
}

static int hashmap_insert(qwen35_hashmap_t *map, int64_t id, size_t entry_index) {
    qwen35_hash_node_t *node = (qwen35_hash_node_t *)malloc(sizeof(qwen35_hash_node_t));
    if (!node) return -1;
    
    uint64_t hash = hash_int(id);
    size_t bucket = hash % map->num_buckets;
    
    node->id = id;
    node->entry_index = entry_index;
    node->next = map->buckets[bucket];
    map->buckets[bucket] = node;
    map->size++;
    
    return 0;
}

static qwen35_hash_node_t *hashmap_find(qwen35_hashmap_t *map, int64_t id) {
    uint64_t hash = hash_int(id);
    size_t bucket = hash % map->num_buckets;
    
    qwen35_hash_node_t *node = map->buckets[bucket];
    while (node) {
        if (node->id == id) {
            return node;
        }
        node = node->next;
    }
    
    return NULL;
}

static int hashmap_remove(qwen35_hashmap_t *map, int64_t id) {
    uint64_t hash = hash_int(id);
    size_t bucket = hash % map->num_buckets;
    
    qwen35_hash_node_t *prev = NULL;
    qwen35_hash_node_t *curr = map->buckets[bucket];
    
    while (curr) {
        if (curr->id == id) {
            if (prev) {
                prev->next = curr->next;
            } else {
                map->buckets[bucket] = curr->next;
            }
            free(curr);
            map->size--;
            return 0;
        }
        prev = curr;
        curr = curr->next;
    }
    
    return -1;
}

float qwen35_cosine_similarity(const float *a, const float *b, size_t dim) {
    float dot = 0.0f;
    float norm_a = 0.0f;
    float norm_b = 0.0f;
    
    for (size_t i = 0; i < dim; i++) {
        dot += a[i] * b[i];
        norm_a += a[i] * a[i];
        norm_b += b[i] * b[i];
    }
    
    if (norm_a == 0.0f || norm_b == 0.0f) {
        return 0.0f;
    }
    
    return dot / (sqrtf(norm_a) * sqrtf(norm_b));
}

float qwen35_euclidean_distance(const float *a, const float *b, size_t dim) {
    float sum = 0.0f;
    
    for (size_t i = 0; i < dim; i++) {
        float diff = a[i] - b[i];
        sum += diff * diff;
    }
    
    return sqrtf(sum);
}

float qwen35_dot_product(const float *a, const float *b, size_t dim) {
    float dot = 0.0f;
    
    for (size_t i = 0; i < dim; i++) {
        dot += a[i] * b[i];
    }
    
    return dot;
}

void qwen35_normalize_vector(float *vector, size_t dim) {
    float norm = 0.0f;
    
    for (size_t i = 0; i < dim; i++) {
        norm += vector[i] * vector[i];
    }
    
    if (norm > 0.0f) {
        norm = sqrtf(norm);
        for (size_t i = 0; i < dim; i++) {
            vector[i] /= norm;
        }
    }
}

static float compute_distance(qwen35_vector_db_t *db, const float *a, const float *b) {
    switch (db->distance_type) {
        case QWEN35_DIST_COSINE:
            return 1.0f - qwen35_cosine_similarity(a, b, db->dimensions);
        case QWEN35_DIST_EUCLIDEAN:
            return qwen35_euclidean_distance(a, b, db->dimensions);
        case QWEN35_DIST_DOT_PRODUCT:
            return -qwen35_dot_product(a, b, db->dimensions);
        default:
            return 0.0f;
    }
}

typedef struct {
    int64_t id;
    float distance;
} qwen35_result_t;

static int compare_results(const void *a, const void *b) {
    const qwen35_result_t *ra = (const qwen35_result_t *)a;
    const qwen35_result_t *rb = (const qwen35_result_t *)b;
    
    if (ra->distance < rb->distance) return -1;
    if (ra->distance > rb->distance) return 1;
    return 0;
}

qwen35_vector_db_t *qwen35_db_create(size_t dimensions, qwen35_distance_t dist_type) {
    if (dimensions == 0 || dimensions > QWEN35_MAX_DIMENSIONS) {
        return NULL;
    }
    
    qwen35_vector_db_t *db = (qwen35_vector_db_t *)calloc(1, sizeof(qwen35_vector_db_t));
    if (!db) return NULL;
    
    db->entries = (qwen35_entry_t *)calloc(QWEN35_DEFAULT_CAPACITY, sizeof(qwen35_entry_t));
    if (!db->entries) {
        free(db);
        return NULL;
    }
    
    db->id_map = hashmap_create(QWEN35_DEFAULT_HASH_BUCKETS);
    if (!db->id_map) {
        free(db->entries);
        free(db);
        return NULL;
    }
    
    db->capacity = QWEN35_DEFAULT_CAPACITY;
    db->size = 0;
    db->dimensions = dimensions;
    db->distance_type = dist_type;
    db->is_normalized = 0;
    
    return db;
}

void qwen35_db_destroy(qwen35_vector_db_t *db) {
    if (!db) return;
    
    for (size_t i = 0; i < db->size; i++) {
        if (db->entries[i].vector) {
            free(db->entries[i].vector);
        }
        if (db->entries[i].metadata) {
            free(db->entries[i].metadata);
        }
    }
    
    hashmap_destroy(db->id_map);
    free(db->entries);
    free(db);
}

static int db_expand(qwen35_vector_db_t *db) {
    size_t new_capacity = db->capacity * 2;
    qwen35_entry_t *new_entries = (qwen35_entry_t *)realloc(db->entries, 
                                                            new_capacity * sizeof(qwen35_entry_t));
    if (!new_entries) return -1;
    
    db->entries = new_entries;
    db->capacity = new_capacity;
    return 0;
}

int qwen35_db_insert(qwen35_vector_db_t *db, int64_t id, const float *vector, 
                     void *metadata, size_t metadata_size) {
    if (!db || !vector) return -1;
    
    if (hashmap_find(db->id_map, id) != NULL) {
        return -1;
    }
    
    if (db->size >= db->capacity) {
        if (db_expand(db) != 0) {
            return -1;
        }
    }
    
    qwen35_entry_t *entry = &db->entries[db->size];
    entry->vector = (float *)malloc(db->dimensions * sizeof(float));
    if (!entry->vector) return -1;
    
    memcpy(entry->vector, vector, db->dimensions * sizeof(float));
    
    if (db->distance_type == QWEN35_DIST_COSINE) {
        qwen35_normalize_vector(entry->vector, db->dimensions);
    }
    
    entry->id = id;
    entry->dim = db->dimensions;
    
    if (metadata && metadata_size > 0) {
        entry->metadata = malloc(metadata_size);
        if (!entry->metadata) {
            free(entry->vector);
            return -1;
        }
        memcpy(entry->metadata, metadata, metadata_size);
        entry->metadata_size = metadata_size;
    } else {
        entry->metadata = NULL;
        entry->metadata_size = 0;
    }
    
    if (hashmap_insert(db->id_map, id, db->size) != 0) {
        free(entry->vector);
        if (entry->metadata) free(entry->metadata);
        return -1;
    }
    
    db->size++;
    return 0;
}

int qwen35_db_delete(qwen35_vector_db_t *db, int64_t id) {
    if (!db) return -1;
    
    qwen35_hash_node_t *node = hashmap_find(db->id_map, id);
    if (!node) return -1;
    
    size_t idx = node->entry_index;
    
    free(db->entries[idx].vector);
    if (db->entries[idx].metadata) {
        free(db->entries[idx].metadata);
    }
    
    if (idx < db->size - 1) {
        memcpy(&db->entries[idx], &db->entries[db->size - 1], sizeof(qwen35_entry_t));
        
        qwen35_hash_node_t *moved_node = hashmap_find(db->id_map, db->entries[idx].id);
        if (moved_node) {
            moved_node->entry_index = idx;
        }
    }
    
    hashmap_remove(db->id_map, id);
    db->size--;
    
    return 0;
}

int qwen35_db_search(qwen35_vector_db_t *db, const float *query, size_t k, 
                     int64_t *out_ids, float *out_distances) {
    if (!db || !query || k == 0) return -1;
    
    if (db->size == 0) return 0;
    
    float *normalized_query = NULL;
    if (db->distance_type == QWEN35_DIST_COSINE) {
        normalized_query = (float *)malloc(db->dimensions * sizeof(float));
        if (!normalized_query) return -1;
        memcpy(normalized_query, query, db->dimensions * sizeof(float));
        qwen35_normalize_vector(normalized_query, db->dimensions);
    }
    
    const float *search_query = normalized_query ? normalized_query : query;
    
    size_t result_count = (k < db->size) ? k : db->size;
    qwen35_result_t *results = (qwen35_result_t *)malloc(db->size * sizeof(qwen35_result_t));
    if (!results) {
        if (normalized_query) free(normalized_query);
        return -1;
    }
    
    for (size_t i = 0; i < db->size; i++) {
        results[i].id = db->entries[i].id;
        results[i].distance = compute_distance(db, search_query, db->entries[i].vector);
    }
    
    qsort(results, db->size, sizeof(qwen35_result_t), compare_results);
    
    for (size_t i = 0; i < result_count; i++) {
        if (out_ids) out_ids[i] = results[i].id;
        if (out_distances) out_distances[i] = results[i].distance;
    }
    
    free(results);
    if (normalized_query) free(normalized_query);
    
    return (int)result_count;
}

int qwen35_db_search_batch(qwen35_vector_db_t *db, const float **queries, size_t num_queries, 
                           size_t k, int64_t **out_ids, float **out_distances) {
    if (!db || !queries || num_queries == 0 || k == 0) return -1;
    
    for (size_t q = 0; q < num_queries; q++) {
        size_t result_count;
        int64_t* ids = out_ids ? out_ids[q] : NULL;
        float* dists = out_distances ? out_distances[q] : NULL;
        
        result_count = qwen35_db_search(db, queries[q], k, ids, dists);
        
        if (out_ids && out_ids[q]) {
            for (size_t i = result_count; i < k; i++) {
                out_ids[q][i] = -1;
            }
        }
        if (out_distances && out_distances[q]) {
            for (size_t i = result_count; i < k; i++) {
                out_distances[q][i] = 1e9f;
            }
        }
    }
    
    return (int)num_queries;
}

int qwen35_db_get(qwen35_vector_db_t *db, int64_t id, float *out_vector, 
                  void *out_metadata, size_t *out_metadata_size) {
    if (!db) return -1;
    
    qwen35_hash_node_t *node = hashmap_find(db->id_map, id);
    if (!node) return -1;
    
    qwen35_entry_t *entry = &db->entries[node->entry_index];
    
    if (out_vector) {
        memcpy(out_vector, entry->vector, entry->dim * sizeof(float));
    }
    
    if (out_metadata && entry->metadata && entry->metadata_size > 0) {
        memcpy(out_metadata, entry->metadata, entry->metadata_size);
    }
    
    if (out_metadata_size) {
        *out_metadata_size = entry->metadata_size;
    }
    
    return 0;
}

size_t qwen35_db_size(qwen35_vector_db_t *db) {
    return db ? db->size : 0;
}

int qwen35_db_save(qwen35_vector_db_t *db, const char *filename) {
    if (!db || !filename) return -1;
    
    FILE *fp = fopen(filename, "wb");
    if (!fp) return -1;
    
    uint32_t magic = 0x5157454E;
    fwrite(&magic, sizeof(uint32_t), 1, fp);
    
    uint32_t version = 1;
    fwrite(&version, sizeof(uint32_t), 1, fp);
    
    fwrite(&db->dimensions, sizeof(size_t), 1, fp);
    fwrite(&db->distance_type, sizeof(qwen35_distance_t), 1, fp);
    fwrite(&db->size, sizeof(size_t), 1, fp);
    
    for (size_t i = 0; i < db->size; i++) {
        qwen35_entry_t *entry = &db->entries[i];
        
        fwrite(&entry->id, sizeof(int64_t), 1, fp);
        fwrite(entry->vector, sizeof(float), db->dimensions, fp);
        fwrite(&entry->metadata_size, sizeof(size_t), 1, fp);
        
        if (entry->metadata_size > 0) {
            fwrite(entry->metadata, 1, entry->metadata_size, fp);
        }
    }
    
    fclose(fp);
    return 0;
}

qwen35_vector_db_t *qwen35_db_load(const char *filename) {
    if (!filename) return NULL;
    
    FILE *fp = fopen(filename, "rb");
    if (!fp) return NULL;
    
    uint32_t magic;
    if (fread(&magic, sizeof(uint32_t), 1, fp) != 1 || magic != 0x5157454E) {
        fclose(fp);
        return NULL;
    }
    
    uint32_t version;
    if (fread(&version, sizeof(uint32_t), 1, fp) != 1 || version != 1) {
        fclose(fp);
        return NULL;
    }
    
    size_t dimensions;
    qwen35_distance_t distance_type;
    size_t size;
    
    if (fread(&dimensions, sizeof(size_t), 1, fp) != 1 ||
        fread(&distance_type, sizeof(qwen35_distance_t), 1, fp) != 1 ||
        fread(&size, sizeof(size_t), 1, fp) != 1) {
        fclose(fp);
        return NULL;
    }
    
    qwen35_vector_db_t *db = qwen35_db_create(dimensions, distance_type);
    if (!db) {
        fclose(fp);
        return NULL;
    }
    
    while (db->size < size) {
        if (db->size >= db->capacity) {
            if (db_expand(db) != 0) {
                qwen35_db_destroy(db);
                fclose(fp);
                return NULL;
            }
        }
        
        qwen35_entry_t *entry = &db->entries[db->size];
        
        if (fread(&entry->id, sizeof(int64_t), 1, fp) != 1) {
            qwen35_db_destroy(db);
            fclose(fp);
            return NULL;
        }
        
        entry->vector = (float *)malloc(dimensions * sizeof(float));
        if (!entry->vector) {
            qwen35_db_destroy(db);
            fclose(fp);
            return NULL;
        }
        
        if (fread(entry->vector, sizeof(float), dimensions, fp) != dimensions) {
            free(entry->vector);
            qwen35_db_destroy(db);
            fclose(fp);
            return NULL;
        }
        
        if (fread(&entry->metadata_size, sizeof(size_t), 1, fp) != 1) {
            entry->metadata_size = 0;
        }
        
        if (entry->metadata_size > 0) {
            entry->metadata = malloc(entry->metadata_size);
            if (!entry->metadata) {
                free(entry->vector);
                qwen35_db_destroy(db);
                fclose(fp);
                return NULL;
            }
            
            if (fread(entry->metadata, 1, entry->metadata_size, fp) != entry->metadata_size) {
                free(entry->metadata);
                free(entry->vector);
                qwen35_db_destroy(db);
                fclose(fp);
                return NULL;
            }
        } else {
            entry->metadata = NULL;
        }
        
        entry->dim = dimensions;
        
        if (hashmap_insert(db->id_map, entry->id, db->size) != 0) {
            if (entry->metadata) free(entry->metadata);
            free(entry->vector);
            qwen35_db_destroy(db);
            fclose(fp);
            return NULL;
        }
        
        db->size++;
    }
    
    fclose(fp);
    return db;
}

const char *qwen35_get_version(void) {
    return QWEN35_VDB_VERSION;
}
