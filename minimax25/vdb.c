#include "vdb.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <math.h>
#include <time.h>

#define DEFAULT_CAPACITY 1024
#define DEFAULT_HASH_BUCKETS 8192
#define INDEX_NOT_BUILT -1

typedef struct HashMapEntry {
    uint64_t key;
    VectorEntry* value;
    struct HashMapEntry* next;
} HashMapEntry;

typedef struct {
    HashMapEntry** buckets;
    uint64_t size;
    uint64_t capacity;
} HashMap;

struct VectorDatabase {
    VectorEntry* entries;
    uint64_t count;
    uint64_t capacity;
    uint32_t dimension;
    bool use_index;
    HashMap* id_map;
    DistanceMetric metric;
};

static uint64_t hash_uint64(uint64_t x) {
    x ^= x >> 33;
    x *= 0xff51afd7ed558ccdULL;
    x ^= x >> 33;
    x *= 0xc4ceb9fe1a85ec53ULL;
    x ^= x >> 33;
    return x;
}

static HashMap* hashmap_create(uint64_t capacity) {
    HashMap* map = (HashMap*)malloc(sizeof(HashMap));
    if (!map) return NULL;
    
    map->capacity = capacity;
    map->size = 0;
    map->buckets = (HashMapEntry**)calloc(capacity, sizeof(HashMapEntry*));
    if (!map->buckets) {
        free(map);
        return NULL;
    }
    return map;
}

static void hashmap_destroy(HashMap* map) {
    if (!map) return;
    
    for (uint64_t i = 0; i < map->capacity; i++) {
        HashMapEntry* entry = map->buckets[i];
        while (entry) {
            HashMapEntry* next = entry->next;
            free(entry);
            entry = next;
        }
    }
    free(map->buckets);
    free(map);
}

static int hashmap_put(HashMap* map, uint64_t key, VectorEntry* value) {
    uint64_t idx = hash_uint64(key) % map->capacity;
    
    HashMapEntry* entry = map->buckets[idx];
    while (entry) {
        if (entry->key == key) {
            entry->value = value;
            return VDB_OK;
        }
        entry = entry->next;
    }
    
    HashMapEntry* new_entry = (HashMapEntry*)malloc(sizeof(HashMapEntry));
    if (!new_entry) return VDB_OUT_OF_MEMORY;
    
    new_entry->key = key;
    new_entry->value = value;
    new_entry->next = map->buckets[idx];
    map->buckets[idx] = new_entry;
    map->size++;
    
    return VDB_OK;
}

static VectorEntry* hashmap_get(HashMap* map, uint64_t key) {
    uint64_t idx = hash_uint64(key) % map->capacity;
    
    HashMapEntry* entry = map->buckets[idx];
    while (entry) {
        if (entry->key == key) {
            return entry->value;
        }
        entry = entry->next;
    }
    return NULL;
}

static int hashmap_remove(HashMap* map, uint64_t key) {
    uint64_t idx = hash_uint64(key) % map->capacity;
    
    HashMapEntry* entry = map->buckets[idx];
    HashMapEntry* prev = NULL;
    
    while (entry) {
        if (entry->key == key) {
            if (prev) {
                prev->next = entry->next;
            } else {
                map->buckets[idx] = entry->next;
            }
            free(entry);
            map->size--;
            return VDB_OK;
        }
        prev = entry;
        entry = entry->next;
    }
    
    return VDB_NOT_FOUND;
}

Vector* vector_new(uint32_t dim) {
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

void vector_free(Vector* vec) {
    if (vec) {
        free(vec->data);
        free(vec);
    }
}

int vector_set(Vector* vec, const float* data) {
    if (!vec || !data) return VDB_ERROR;
    memcpy(vec->data, data, vec->dim * sizeof(float));
    return VDB_OK;
}

int vector_copy(const Vector* src, Vector* dst) {
    if (!src || !dst || src->dim != dst->dim) return VDB_ERROR;
    memcpy(dst->data, src->data, src->dim * sizeof(float));
    return VDB_OK;
}

float vector_magnitude(const Vector* vec) {
    if (!vec) return 0.0f;
    float sum = 0.0f;
    for (uint32_t i = 0; i < vec->dim; i++) {
        sum += vec->data[i] * vec->data[i];
    }
    return sqrtf(sum);
}

float vector_dot_product(const Vector* a, const Vector* b) {
    if (!a || !b || a->dim != b->dim) return 0.0f;
    
    float sum = 0.0f;
    for (uint32_t i = 0; i < a->dim; i++) {
        sum += a->data[i] * b->data[i];
    }
    return sum;
}

float vector_cosine_similarity(const Vector* a, const Vector* b) {
    if (!a || !b || a->dim != b->dim) return -1.0f;
    
    float mag_a = vector_magnitude(a);
    float mag_b = vector_magnitude(b);
    
    if (mag_a == 0.0f || mag_b == 0.0f) return -1.0f;
    
    return vector_dot_product(a, b) / (mag_a * mag_b);
}

float vector_euclidean_distance(const Vector* a, const Vector* b) {
    if (!a || !b || a->dim != b->dim) return -1.0f;
    
    float sum = 0.0f;
    for (uint32_t i = 0; i < a->dim; i++) {
        float diff = a->data[i] - b->data[i];
        sum += diff * diff;
    }
    return sqrtf(sum);
}

float vector_distance(const Vector* a, const Vector* b, DistanceMetric metric) {
    switch (metric) {
        case DISTANCE_COSINE:
            return 1.0f - vector_cosine_similarity(a, b);
        case DISTANCE_EUCLIDEAN:
            return vector_euclidean_distance(a, b);
        case DISTANCE_DOT_PRODUCT:
            return -vector_dot_product(a, b);
        default:
            return vector_euclidean_distance(a, b);
    }
}

static int vdb_resize(VectorDatabase* db) {
    if (db->count < db->capacity) return VDB_OK;
    
    uint64_t new_capacity = db->capacity * 2;
    VectorEntry* new_entries = (VectorEntry*)realloc(db->entries, 
                                                      new_capacity * sizeof(VectorEntry));
    if (!new_entries) return VDB_OUT_OF_MEMORY;
    
    db->entries = new_entries;
    db->capacity = new_capacity;
    return VDB_OK;
}

VectorDatabase* vdb_create(uint32_t dimension) {
    if (dimension == 0) return NULL;
    
    VectorDatabase* db = (VectorDatabase*)malloc(sizeof(VectorDatabase));
    if (!db) return NULL;
    
    db->entries = (VectorEntry*)calloc(DEFAULT_CAPACITY, sizeof(VectorEntry));
    if (!db->entries) {
        free(db);
        return NULL;
    }
    
    db->id_map = hashmap_create(DEFAULT_HASH_BUCKETS);
    if (!db->id_map) {
        free(db->entries);
        free(db);
        return NULL;
    }
    
    db->count = 0;
    db->capacity = DEFAULT_CAPACITY;
    db->dimension = dimension;
    db->use_index = false;
    db->metric = DISTANCE_COSINE;
    
    return db;
}

void vdb_free(VectorDatabase* db) {
    if (!db) return;
    
    for (uint64_t i = 0; i < db->count; i++) {
        free(db->entries[i].vector.data);
        free(db->entries[i].metadata);
    }
    
    free(db->entries);
    hashmap_destroy(db->id_map);
    free(db);
}

static void normalize_vector(Vector* vec) {
    if (!vec || !vec->data) return;
    
    float mag = 0.0f;
    for (uint32_t i = 0; i < vec->dim; i++) {
        mag += vec->data[i] * vec->data[i];
    }
    
    if (mag > 0.0f) {
        mag = sqrtf(mag);
        for (uint32_t i = 0; i < vec->dim; i++) {
            vec->data[i] /= mag;
        }
    }
}

int vdb_insert(VectorDatabase* db, uint64_t id, const Vector* vec,
              const void* metadata, uint32_t metadata_size) {
    if (!db || !vec) return VDB_ERROR;
    if (vec->dim != db->dimension) return VDB_INVALID_DIM;
    
    if (hashmap_get(db->id_map, id)) {
        return VDB_DUPLICATE_ID;
    }
    
    if (vdb_resize(db) != VDB_OK) {
        return VDB_OUT_OF_MEMORY;
    }
    
    VectorEntry* entry = &db->entries[db->count];
    entry->id = id;
    entry->vector.dim = vec->dim;
    entry->vector.data = (float*)malloc(vec->dim * sizeof(float));
    
    if (!entry->vector.data) {
        return VDB_OUT_OF_MEMORY;
    }
    
    memcpy(entry->vector.data, vec->data, vec->dim * sizeof(float));
    
    if (db->metric == DISTANCE_COSINE) {
        normalize_vector(&entry->vector);
    }
    
    if (metadata && metadata_size > 0) {
        entry->metadata = malloc(metadata_size);
        if (!entry->metadata) {
            free(entry->vector.data);
            return VDB_OUT_OF_MEMORY;
        }
        memcpy(entry->metadata, metadata, metadata_size);
        entry->metadata_size = metadata_size;
    } else {
        entry->metadata = NULL;
        entry->metadata_size = 0;
    }
    
    hashmap_put(db->id_map, id, entry);
    db->count++;
    
    return VDB_OK;
}

int vdb_delete(VectorDatabase* db, uint64_t id) {
    if (!db) return VDB_ERROR;
    
    VectorEntry* entry = hashmap_get(db->id_map, id);
    if (!entry) return VDB_NOT_FOUND;
    
    uint64_t idx = entry - db->entries;
    
    free(entry->vector.data);
    free(entry->metadata);
    
    if (idx < db->count - 1) {
        memmove(&db->entries[idx], &db->entries[idx + 1],
                (db->count - idx - 1) * sizeof(VectorEntry));
    }
    
    db->count--;
    hashmap_remove(db->id_map, id);
    
    return VDB_OK;
}

int vdb_update(VectorDatabase* db, uint64_t id, const Vector* vec,
              const void* metadata, uint32_t metadata_size) {
    if (!db) return VDB_ERROR;
    
    VectorEntry* entry = hashmap_get(db->id_map, id);
    if (!entry) return VDB_NOT_FOUND;
    
    if (vec && vec->dim != db->dimension) {
        return VDB_INVALID_DIM;
    }
    
    if (vec) {
        free(entry->vector.data);
        entry->vector.data = (float*)malloc(vec->dim * sizeof(float));
        if (!entry->vector.data) return VDB_OUT_OF_MEMORY;
        memcpy(entry->vector.data, vec->data, vec->dim * sizeof(float));
    }
    
    if (metadata && metadata_size > 0) {
        free(entry->metadata);
        entry->metadata = malloc(metadata_size);
        if (!entry->metadata) return VDB_OUT_OF_MEMORY;
        memcpy(entry->metadata, metadata, metadata_size);
        entry->metadata_size = metadata_size;
    }
    
    return VDB_OK;
}

VectorEntry* vdb_get(VectorDatabase* db, uint64_t id) {
    if (!db) return NULL;
    return hashmap_get(db->id_map, id);
}

static int compare_results(const void* a, const void* b) {
    const SearchResult* ra = (const SearchResult*)a;
    const SearchResult* rb = (const SearchResult*)b;
    
    if (ra->distance < rb->distance) return -1;
    if (ra->distance > rb->distance) return 1;
    return 0;
}

static float fast_cosine_similarity(const Vector* a, const Vector* b) {
    if (!a || !b || a->dim != b->dim) return 0.0f;
    
    float dot = 0.0f;
    for (uint32_t i = 0; i < a->dim; i++) {
        dot += a->data[i] * b->data[i];
    }
    return dot;
}

SearchResult* vdb_search(VectorDatabase* db, const Vector* query,
                        const SearchOptions* options, uint32_t* result_count) {
    if (!db || !query || !result_count) return NULL;
    if (query->dim != db->dimension) return NULL;
    
    uint32_t top_k = options ? options->top_k : 10;
    float max_dist = options ? options->max_distance : 1e9f;
    DistanceMetric metric = options ? options->metric : DISTANCE_COSINE;
    
    if (db->count == 0) {
        *result_count = 0;
        return NULL;
    }
    
    SearchResult* results = (SearchResult*)malloc(db->count * sizeof(SearchResult));
    if (!results) return NULL;
    
    Vector normalized_query;
    normalized_query.dim = query->dim;
    normalized_query.data = NULL;
    const Vector* search_query = query;
    
    if (metric == DISTANCE_COSINE) {
        normalized_query.data = (float*)malloc(query->dim * sizeof(float));
        if (normalized_query.data) {
            memcpy(normalized_query.data, query->data, query->dim * sizeof(float));
            normalize_vector(&normalized_query);
            search_query = &normalized_query;
        }
    }
    
    uint32_t count = 0;
    for (uint64_t i = 0; i < db->count; i++) {
        float dist;
        
        if (metric == DISTANCE_COSINE && normalized_query.data) {
            dist = 1.0f - fast_cosine_similarity(search_query, &db->entries[i].vector);
        } else {
            dist = vector_distance(query, &db->entries[i].vector, metric);
        }
        
        if (dist <= max_dist) {
            results[count].id = db->entries[i].id;
            results[count].distance = dist;
            
            if (db->entries[i].metadata && db->entries[i].metadata_size > 0) {
                results[count].metadata = malloc(db->entries[i].metadata_size);
                if (results[count].metadata) {
                    memcpy(results[count].metadata, db->entries[i].metadata,
                           db->entries[i].metadata_size);
                    results[count].metadata_size = db->entries[i].metadata_size;
                }
            } else {
                results[count].metadata = NULL;
                results[count].metadata_size = 0;
            }
            
            count++;
        }
    }
    
    if (normalized_query.data) {
        free(normalized_query.data);
    }
    
    qsort(results, count, sizeof(SearchResult), compare_results);
    
    if (count > top_k) {
        for (uint32_t i = top_k; i < count; i++) {
            free(results[i].metadata);
        }
        count = top_k;
    }
    
    *result_count = count;
    return results;
}

void vdb_free_results(SearchResult* results, uint32_t count) {
    if (!results) return;
    
    for (uint32_t i = 0; i < count; i++) {
        free(results[i].metadata);
    }
    free(results);
}

void vdb_set_index(VectorDatabase* db, bool enable) {
    if (db) {
        db->use_index = enable;
    }
}

int vdb_build_index(VectorDatabase* db) {
    if (!db) return VDB_ERROR;
    db->use_index = true;
    return VDB_OK;
}

void vdb_stats(const VectorDatabase* db) {
    if (!db) return;
    
    printf("Vector Database Statistics:\n");
    printf("  Dimension: %u\n", db->dimension);
    printf("  Total entries: %llu\n", (unsigned long long)db->count);
    printf("  Capacity: %llu\n", (unsigned long long)db->capacity);
    printf("  Using index: %s\n", db->use_index ? "yes" : "no");
    
    const char* metric_name = "unknown";
    switch (db->metric) {
        case DISTANCE_COSINE: metric_name = "cosine"; break;
        case DISTANCE_EUCLIDEAN: metric_name = "euclidean"; break;
        case DISTANCE_DOT_PRODUCT: metric_name = "dot_product"; break;
    }
    printf("  Distance metric: %s\n", metric_name);
}

int vdb_save(const VectorDatabase* db, const char* filename) {
    if (!db || !filename) return VDB_ERROR;
    
    FILE* fp = fopen(filename, "wb");
    if (!fp) return VDB_ERROR;
    
    fwrite(&db->dimension, sizeof(uint32_t), 1, fp);
    fwrite(&db->count, sizeof(uint64_t), 1, fp);
    fwrite(&db->use_index, sizeof(bool), 1, fp);
    
    for (uint64_t i = 0; i < db->count; i++) {
        fwrite(&db->entries[i].id, sizeof(uint64_t), 1, fp);
        fwrite(&db->entries[i].metadata_size, sizeof(uint32_t), 1, fp);
        fwrite(db->entries[i].vector.data, sizeof(float), db->dimension, fp);
        
        if (db->entries[i].metadata_size > 0) {
            fwrite(db->entries[i].metadata, 1, db->entries[i].metadata_size, fp);
        }
    }
    
    fclose(fp);
    return VDB_OK;
}

VectorDatabase* vdb_load(const char* filename) {
    if (!filename) return NULL;
    
    FILE* fp = fopen(filename, "rb");
    if (!fp) return NULL;
    
    uint32_t dimension;
    uint64_t count;
    bool use_index;
    
    if (fread(&dimension, sizeof(uint32_t), 1, fp) != 1) {
        fclose(fp);
        return NULL;
    }
    
    fread(&count, sizeof(uint64_t), 1, fp);
    fread(&use_index, sizeof(bool), 1, fp);
    
    VectorDatabase* db = vdb_create(dimension);
    if (!db) {
        fclose(fp);
        return NULL;
    }
    
    db->use_index = use_index;
    
    for (uint64_t i = 0; i < count; i++) {
        uint64_t id;
        uint32_t metadata_size;
        
        fread(&id, sizeof(uint64_t), 1, fp);
        fread(&metadata_size, sizeof(uint32_t), 1, fp);
        
        Vector* vec = vector_new(dimension);
        if (!vec) {
            vdb_free(db);
            fclose(fp);
            return NULL;
        }
        
        fread(vec->data, sizeof(float), dimension, fp);
        
        void* metadata = NULL;
        if (metadata_size > 0) {
            metadata = malloc(metadata_size);
            fread(metadata, 1, metadata_size, fp);
        }
        
        vdb_insert(db, id, vec, metadata, metadata_size);
        
        vector_free(vec);
        free(metadata);
    }
    
    fclose(fp);
    return db;
}
