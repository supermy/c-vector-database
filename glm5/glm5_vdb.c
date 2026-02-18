#include "glm5_vdb.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <math.h>

#define INIT_CAP 4096
#define HASH_BUCKETS 8192

typedef struct HashNode {
    uint64_t key;
    VecEntry* val;
    struct HashNode* next;
} HashNode;

struct VecDB {
    VecEntry* entries;
    uint64_t cnt;
    uint64_t cap;
    uint32_t dim;
    HashNode** buckets;
    uint64_t bucket_cnt;
};

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

void vec_free(Vector* v) {
    if (v) { free(v->values); free(v); }
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
    
    db->cnt = 0;
    db->cap = INIT_CAP;
    db->dim = dim;
    db->bucket_cnt = HASH_BUCKETS;
    
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
    for (uint64_t i = 0; i < db->cnt; i++) {
        free_entry(&db->entries[i]);
    }
    free(db->entries);
    hash_free(db->buckets, db->bucket_cnt);
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
}
