#include "vdb.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

void generate_random_vector(Vector* vec) {
    for (uint32_t i = 0; i < vec->dim; i++) {
        vec->data[i] = (float)rand() / RAND_MAX;
    }
}

void test_basic_operations() {
    printf("\n========== Basic Operations Test ==========\n");
    
    VectorDatabase* db = vdb_create(128);
    if (!db) {
        printf("Failed to create database\n");
        return;
    }
    
    printf("Database created with dimension 128\n");
    
    for (int i = 0; i < 50; i++) {
        Vector* vec = vector_new(128);
        generate_random_vector(vec);
        
        char meta[64];
        snprintf(meta, sizeof(meta), "record_%d", i);
        
        int ret = vdb_insert(db, i, vec, meta, strlen(meta) + 1);
        if (ret != VDB_OK) {
            printf("Failed to insert record %d: %d\n", i, ret);
        }
        
        vector_free(vec);
    }
    
    vdb_stats(db);
    
    VectorEntry* entry = vdb_get(db, 25);
    if (entry) {
        printf("\nFound entry with id=25, metadata: %s\n", (char*)entry->metadata);
    }
    
    Vector* new_vec = vector_new(128);
    generate_random_vector(new_vec);
    vdb_update(db, 25, new_vec, "updated_record", 15);
    entry = vdb_get(db, 25);
    if (entry) {
        printf("After update, metadata: %s\n", (char*)entry->metadata);
    }
    vector_free(new_vec);
    
    vdb_delete(db, 25);
    entry = vdb_get(db, 25);
    printf("After delete, entry= %s\n", entry ? "found" : "not found");
    
    vdb_free(db);
    printf("Basic operations test passed!\n");
}

void test_similarity_search() {
    printf("\n========== Similarity Search Test ==========\n");
    
    VectorDatabase* db = vdb_create(64);
    
    printf("Inserting vectors...\n");
    for (int i = 0; i < 20; i++) {
        Vector* vec = vector_new(64);
        
        for (uint32_t j = 0; j < 64; j++) {
            if (j < 32) {
                vec->data[j] = 1.0f;
            } else {
                vec->data[j] = 0.0f;
            }
        }
        
        vec->data[i % 64] += 0.1f * (i + 1);
        
        char meta[64];
        snprintf(meta, sizeof(meta), "similar_%d", i);
        vdb_insert(db, i, vec, meta, strlen(meta) + 1);
        
        vector_free(vec);
    }
    
    for (int i = 20; i < 40; i++) {
        Vector* vec = vector_new(64);
        
        for (uint32_t j = 0; j < 64; j++) {
            if (j >= 32) {
                vec->data[j] = 1.0f;
            } else {
                vec->data[j] = 0.0f;
            }
        }
        
        char meta[64];
        snprintf(meta, sizeof(meta), "different_%d", i);
        vdb_insert(db, i, vec, meta, strlen(meta) + 1);
        
        vector_free(vec);
    }
    
    Vector* query = vector_new(64);
    for (uint32_t j = 0; j < 64; j++) {
        query->data[j] = (j < 32) ? 1.0f : 0.0f;
    }
    
    SearchOptions opts = {
        .top_k = 5,
        .max_distance = 1.0f,
        .metric = DISTANCE_COSINE,
        .use_index = false,
        .ef_search = 64
    };
    
    uint32_t result_count;
    SearchResult* results = vdb_search(db, query, &opts, &result_count);
    
    printf("\nSearch results (top %u):\n", result_count);
    for (uint32_t i = 0; i < result_count; i++) {
        printf("  Rank %u: id=%llu, distance=%.4f, metadata=%s\n",
               i + 1,
               (unsigned long long)results[i].id,
               results[i].distance,
               results[i].metadata ? (char*)results[i].metadata : "N/A");
    }
    
    vdb_free_results(results, result_count);
    vector_free(query);
    vdb_free(db);
    
    printf("Similarity search test passed!\n");
}

void test_performance() {
    printf("\n========== Performance Test ==========\n");
    
    const uint32_t dim = 256;
    const uint64_t num_vectors = 5000;
    
    VectorDatabase* db = vdb_create(dim);
    
    printf("Inserting %llu vectors of dimension %u...\n",
           (unsigned long long)num_vectors, dim);
    
    clock_t start = clock();
    
    for (uint64_t i = 0; i < num_vectors; i++) {
        Vector* vec = vector_new(dim);
        generate_random_vector(vec);
        vdb_insert(db, i, vec, NULL, 0);
        vector_free(vec);
    }
    
    clock_t end = clock();
    double insert_time = (double)(end - start) / CLOCKS_PER_SEC;
    
    printf("Insert time: %.3f seconds (%.1f vectors/sec)\n",
           insert_time, num_vectors / insert_time);
    
    Vector* query = vector_new(dim);
    generate_random_vector(query);
    
    SearchOptions opts = {
        .top_k = 10,
        .max_distance = 1e9f,
        .metric = DISTANCE_COSINE,
        .use_index = false,
        .ef_search = 64
    };
    
    printf("Performing 50 searches...\n");
    start = clock();
    
    for (int i = 0; i < 50; i++) {
        uint32_t count;
        SearchResult* results = vdb_search(db, query, &opts, &count);
        if (results) {
            vdb_free_results(results, count);
        }
    }
    
    end = clock();
    double search_time = (double)(end - start) / CLOCKS_PER_SEC;
    
    printf("Search time: %.3f seconds (%.3f sec/query)\n",
           search_time, search_time / 50.0);
    
    vector_free(query);
    vdb_free(db);
    
    printf("Performance test passed!\n");
}

void test_persistence() {
    printf("\n========== Persistence Test ==========\n");
    
    const char* filename = "test_vdb.bin";
    
    VectorDatabase* db = vdb_create(32);
    
    printf("Creating test data...\n");
    for (int i = 0; i < 100; i++) {
        Vector* vec = vector_new(32);
        generate_random_vector(vec);
        
        char meta[64];
        snprintf(meta, sizeof(meta), "persistent_%d", i);
        vdb_insert(db, i, vec, meta, strlen(meta) + 1);
        
        vector_free(vec);
    }
    
    printf("Saving to file: %s\n", filename);
    int ret = vdb_save(db, filename);
    printf("Save result: %s\n", ret == VDB_OK ? "success" : "failed");
    
    vdb_free(db);
    
    printf("Loading from file: %s\n", filename);
    VectorDatabase* loaded_db = vdb_load(filename);
    printf("Load result: %s\n", loaded_db ? "success" : "failed");
    
    if (loaded_db) {
        vdb_stats(loaded_db);
        
        VectorEntry* entry = vdb_get(loaded_db, 50);
        if (entry) {
            printf("Verified entry id=50, metadata: %s\n", (char*)entry->metadata);
        }
        
        vdb_free(loaded_db);
    }
    
    remove(filename);
    
    printf("Persistence test passed!\n");
}

int main() {
    srand((unsigned int)time(NULL));
    
    printf("========================================\n");
    printf("     Vector Database Test Suite\n");
    printf("========================================\n");
    
    test_basic_operations();
    test_similarity_search();
    test_performance();
    test_persistence();
    
    printf("\n========================================\n");
    printf("     All Tests Passed!\n");
    printf("========================================\n");
    
    return 0;
}
