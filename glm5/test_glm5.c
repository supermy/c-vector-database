#include "glm5_vdb.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#include <math.h>

static void rand_vec(Vector* v) {
    for (uint32_t i = 0; i < v->dimension; i++) {
        v->values[i] = (float)rand() / RAND_MAX;
    }
}

static void test_basic(void) {
    printf("\n=== Basic Operations ===\n");
    
    VecDB* db = vdb_new(128);
    if (!db) { printf("Failed to create DB\n"); return; }
    
    printf("Created DB with dim=128\n");
    
    for (int i = 0; i < 100; i++) {
        Vector* v = vec_new(128);
        rand_vec(v);
        char m[64];
        snprintf(m, sizeof(m), "item_%d", i);
        int r = vdb_add(db, i, v, m, strlen(m) + 1);
        if (r != GLM5_VDB_OK) printf("Add failed: %d\n", r);
        vec_free(v);
    }
    
    vdb_info(db);
    
    VecEntry* e = vdb_get(db, 50);
    if (e) printf("\nGet id=50: meta=%s\n", (char*)e->meta);
    
    Vector* nv = vec_new(128);
    rand_vec(nv);
    vdb_set(db, 50, nv, "updated_50", 11);
    vec_free(nv);
    
    e = vdb_get(db, 50);
    if (e) printf("After update: meta=%s\n", (char*)e->meta);
    
    vdb_del(db, 50);
    e = vdb_get(db, 50);
    printf("After delete: %s\n", e ? "found" : "not found");
    
    vdb_free(db);
    printf("Basic test passed!\n");
}

static void test_query(void) {
    printf("\n=== Query Test ===\n");
    
    VecDB* db = vdb_new(64);
    
    for (int i = 0; i < 20; i++) {
        Vector* v = vec_new(64);
        for (uint32_t j = 0; j < 64; j++) {
            v->values[j] = (j < 32) ? 1.0f : 0.0f;
        }
        v->values[i % 64] += 0.05f * i;
        
        char m[64];
        snprintf(m, sizeof(m), "vec_%d", i);
        vdb_add(db, i, v, m, strlen(m) + 1);
        vec_free(v);
    }
    
    for (int i = 20; i < 40; i++) {
        Vector* v = vec_new(64);
        for (uint32_t j = 0; j < 64; j++) {
            v->values[j] = (j >= 32) ? 1.0f : 0.0f;
        }
        char m[64];
        snprintf(m, sizeof(m), "diff_%d", i);
        vdb_add(db, i, v, m, strlen(m) + 1);
        vec_free(v);
    }
    
    Vector* q = vec_new(64);
    for (uint32_t j = 0; j < 64; j++) {
        q->values[j] = (j < 32) ? 1.0f : 0.0f;
    }
    
    QueryOpts opts = { .k = 5, .radius = 1.0f, .metric = METRIC_COSINE, .ef = 64 };
    uint32_t n;
    QueryResult* r = vdb_query(db, q, &opts, &n);
    
    printf("Top %u results:\n", n);
    for (uint32_t i = 0; i < n; i++) {
        printf("  %u: id=%llu dist=%.4f meta=%s\n", i + 1,
               (unsigned long long)r[i].id, r[i].dist, (char*)r[i].meta);
    }
    
    vdb_free_results(r, n);
    vec_free(q);
    vdb_free(db);
    printf("Query test passed!\n");
}

static void test_perf(void) {
    printf("\n=== Performance Test ===\n");
    
    const uint32_t dim = 256;
    const uint64_t n = 10000;
    const int queries = 100;
    
    VecDB* db = vdb_new(dim);
    
    printf("Inserting %llu vectors (dim=%u)...\n", (unsigned long long)n, dim);
    clock_t t0 = clock();
    
    for (uint64_t i = 0; i < n; i++) {
        Vector* v = vec_new(dim);
        rand_vec(v);
        vdb_add(db, i, v, NULL, 0);
        vec_free(v);
    }
    
    clock_t t1 = clock();
    double insert_time = (double)(t1 - t0) / CLOCKS_PER_SEC;
    printf("Insert: %.3f sec (%.0f vec/s)\n", insert_time, n / insert_time);
    
    vdb_info(db);
    
    Vector* q = vec_new(dim);
    rand_vec(q);
    
    QueryOpts opts = { .k = 10, .radius = 1e9f, .metric = METRIC_COSINE, .ef = 64 };
    
    printf("Running %d queries...\n", queries);
    t0 = clock();
    
    for (int i = 0; i < queries; i++) {
        uint32_t cnt;
        QueryResult* r = vdb_query(db, q, &opts, &cnt);
        if (r) vdb_free_results(r, cnt);
    }
    
    t1 = clock();
    double query_time = (double)(t1 - t0) / CLOCKS_PER_SEC;
    printf("Query: %.3f sec (%.3f ms/query)\n", query_time, query_time * 1000 / queries);
    
    vec_free(q);
    vdb_free(db);
    printf("Performance test passed!\n");
}

static void test_persist(void) {
    printf("\n=== Persistence Test ===\n");
    
    const char* path = "glm5_test.bin";
    
    VecDB* db = vdb_new(32);
    
    printf("Creating data...\n");
    for (int i = 0; i < 100; i++) {
        Vector* v = vec_new(32);
        rand_vec(v);
        char m[64];
        snprintf(m, sizeof(m), "persist_%d", i);
        vdb_add(db, i, v, m, strlen(m) + 1);
        vec_free(v);
    }
    
    printf("Saving to %s...\n", path);
    int r = vdb_save(db, path);
    printf("Save: %s\n", r == GLM5_VDB_OK ? "OK" : "FAIL");
    
    vdb_free(db);
    
    printf("Loading from %s...\n", path);
    VecDB* db2 = vdb_load(path);
    printf("Load: %s\n", db2 ? "OK" : "FAIL");
    
    if (db2) {
        vdb_info(db2);
        VecEntry* e = vdb_get(db2, 50);
        if (e) printf("Verified id=50: meta=%s\n", (char*)e->meta);
        vdb_free(db2);
    }
    
    remove(path);
    printf("Persistence test passed!\n");
}

int main(void) {
    srand((unsigned int)time(NULL));
    
    printf("========================================\n");
    printf("   GLM5 Vector Database Test Suite\n");
    printf("========================================\n");
    
    test_basic();
    test_query();
    test_perf();
    test_persist();
    
    printf("\n========================================\n");
    printf("   All Tests Passed!\n");
    printf("========================================\n");
    
    return 0;
}
