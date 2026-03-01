#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#include <math.h>
#include "qwen35_vdb.h"

#define DIMENSIONS 128
#define NUM_VECTORS 1000
#define TOP_K 5

static void generate_random_vector(float *vector, size_t dim) {
    for (size_t i = 0; i < dim; i++) {
        vector[i] = (float)rand() / RAND_MAX;
    }
}

static void print_test_header(const char *test_name) {
    printf("\n");
    printf("========================================\n");
    printf("%s\n", test_name);
    printf("========================================\n");
}

static void test_basic_operations(void) {
    print_test_header("Test: Basic Operations");
    
    qwen35_vector_db_t *db = qwen35_db_create(DIMENSIONS, QWEN35_DIST_COSINE);
    if (!db) {
        printf("[X] Failed to create database\n");
        return;
    }
    printf("[OK] Database created: dimensions=%zu, distance_type=COSINE\n", DIMENSIONS);
    
    float vector1[DIMENSIONS] = {0};
    float vector2[DIMENSIONS] = {0};
    float vector3[DIMENSIONS] = {0};
    
    for (int i = 0; i < DIMENSIONS; i++) {
        vector1[i] = 1.0f;
        vector2[i] = 0.5f;
        vector3[i] = (i % 2 == 0) ? 1.0f : -1.0f;
    }
    
    qwen35_normalize_vector(vector2, DIMENSIONS);
    qwen35_normalize_vector(vector3, DIMENSIONS);
    
    if (qwen35_db_insert(db, 1, vector1, NULL, 0) == 0) {
        printf("[OK] Inserted vector with id=1\n");
    } else {
        printf("[X] Failed to insert vector with id=1\n");
    }
    
    if (qwen35_db_insert(db, 2, vector2, NULL, 0) == 0) {
        printf("[OK] Inserted vector with id=2\n");
    } else {
        printf("[X] Failed to insert vector with id=2\n");
    }
    
    if (qwen35_db_insert(db, 3, vector3, NULL, 0) == 0) {
        printf("[OK] Inserted vector with id=3\n");
    } else {
        printf("[X] Failed to insert vector with id=3\n");
    }
    
    printf("[OK] Database size: %zu\n", qwen35_db_size(db));
    
    int64_t result_ids[TOP_K];
    float result_distances[TOP_K];
    
    int count = qwen35_db_search(db, vector1, TOP_K, result_ids, result_distances);
    printf("[OK] Search returned %d results\n", count);
    
    for (int i = 0; i < count; i++) {
        printf("    Rank %d: id=%ld, distance=%.6f\n", 
               i + 1, (long)result_ids[i], result_distances[i]);
    }
    
    float retrieved_vector[DIMENSIONS];
    if (qwen35_db_get(db, 1, retrieved_vector, NULL, NULL) == 0) {
        printf("[OK] Retrieved vector with id=1\n");
    } else {
        printf("[X] Failed to retrieve vector with id=1\n");
    }
    
    if (qwen35_db_delete(db, 2) == 0) {
        printf("[OK] Deleted vector with id=2\n");
    } else {
        printf("[X] Failed to delete vector with id=2\n");
    }
    
    printf("[OK] Database size after deletion: %zu\n", qwen35_db_size(db));
    
    qwen35_db_destroy(db);
    printf("[OK] Database destroyed\n");
}

static void test_metadata(void) {
    print_test_header("Test: Metadata Storage");
    
    qwen35_vector_db_t *db = qwen35_db_create(DIMENSIONS, QWEN35_DIST_EUCLIDEAN);
    if (!db) {
        printf("[X] Failed to create database\n");
        return;
    }
    
    float vector[DIMENSIONS];
    generate_random_vector(vector, DIMENSIONS);
    
    const char *metadata = "This is test metadata for vector ID 100";
    
    if (qwen35_db_insert(db, 100, vector, (void *)metadata, strlen(metadata) + 1) == 0) {
        printf("[OK] Inserted vector with metadata\n");
    } else {
        printf("[X] Failed to insert vector with metadata\n");
    }
    
    float retrieved_vector[DIMENSIONS];
    char retrieved_metadata[256];
    size_t metadata_size = 0;
    
    if (qwen35_db_get(db, 100, retrieved_vector, retrieved_metadata, &metadata_size) == 0) {
        printf("[OK] Retrieved vector with metadata: %s\n", retrieved_metadata);
    } else {
        printf("[X] Failed to retrieve metadata\n");
    }
    
    qwen35_db_destroy(db);
    printf("[OK] Metadata test completed\n");
}

static void test_persistence(void) {
    print_test_header("Test: Save and Load");
    
    qwen35_vector_db_t *db = qwen35_db_create(DIMENSIONS, QWEN35_DIST_DOT_PRODUCT);
    if (!db) {
        printf("[X] Failed to create database\n");
        return;
    }
    
    for (int i = 0; i < 10; i++) {
        float vector[DIMENSIONS];
        generate_random_vector(vector, DIMENSIONS);
        
        char meta[64];
        snprintf(meta, sizeof(meta), "Metadata for ID %d", i);
        
        if (qwen35_db_insert(db, i, vector, (void *)meta, strlen(meta) + 1) != 0) {
            printf("[X] Failed to insert vector %d\n", i);
        }
    }
    
    printf("[OK] Inserted 10 vectors\n");
    
    const char *filename = "test_qwen35_db.bin";
    if (qwen35_db_save(db, filename) == 0) {
        printf("[OK] Database saved to %s\n", filename);
    } else {
        printf("[X] Failed to save database\n");
    }
    
    qwen35_db_destroy(db);
    
    qwen35_vector_db_t *loaded_db = qwen35_db_load(filename);
    if (!loaded_db) {
        printf("[X] Failed to load database\n");
        return;
    }
    
    printf("[OK] Database loaded from %s\n", filename);
    printf("[OK] Loaded database size: %zu\n", qwen35_db_size(loaded_db));
    
    float query_vector[DIMENSIONS];
    generate_random_vector(query_vector, DIMENSIONS);
    
    int64_t result_ids[TOP_K];
    float result_distances[TOP_K];
    
    int count = qwen35_db_search(loaded_db, query_vector, TOP_K, result_ids, result_distances);
    printf("[OK] Search on loaded database returned %d results\n", count);
    
    qwen35_db_destroy(loaded_db);
    remove(filename);
    printf("[OK] Persistence test completed\n");
}

static void test_distance_metrics(void) {
    print_test_header("Test: Distance Metrics");
    
    float a[4] = {1.0f, 0.0f, 0.0f, 0.0f};
    float b[4] = {0.0f, 1.0f, 0.0f, 0.0f};
    float c[4] = {1.0f, 1.0f, 0.0f, 0.0f};
    
    float cosine_sim = qwen35_cosine_similarity(a, b, 4);
    printf("[OK] Cosine similarity(a, b): %.6f (expected: 0.0)\n", cosine_sim);
    
    cosine_sim = qwen35_cosine_similarity(a, a, 4);
    printf("[OK] Cosine similarity(a, a): %.6f (expected: 1.0)\n", cosine_sim);
    
    float euclidean_dist = qwen35_euclidean_distance(a, b, 4);
    printf("[OK] Euclidean distance(a, b): %.6f (expected: %.6f)\n", 
           euclidean_dist, sqrtf(2.0f));
    
    float dot_prod = qwen35_dot_product(a, c, 4);
    printf("[OK] Dot product(a, c): %.6f (expected: 1.0)\n", dot_prod);
    
    printf("[OK] Distance metrics test completed\n");
}

static void benchmark_insert(void) {
    print_test_header("Benchmark: Insert Performance");
    
    qwen35_vector_db_t *db = qwen35_db_create(DIMENSIONS, QWEN35_DIST_COSINE);
    if (!db) {
        printf("[X] Failed to create database\n");
        return;
    }
    
    clock_t start = clock();
    
    for (int i = 0; i < NUM_VECTORS; i++) {
        float vector[DIMENSIONS];
        generate_random_vector(vector, DIMENSIONS);
        
        if (qwen35_db_insert(db, i, vector, NULL, 0) != 0) {
            printf("[X] Failed to insert vector %d\n", i);
        }
    }
    
    clock_t end = clock();
    double elapsed = (double)(end - start) / CLOCKS_PER_SEC;
    double throughput = NUM_VECTORS / elapsed;
    
    printf("[OK] Inserted %d vectors in %.3f seconds\n", NUM_VECTORS, elapsed);
    printf("[OK] Throughput: %.0f vectors/second\n", throughput);
    
    qwen35_db_destroy(db);
}

static void benchmark_search(void) {
    print_test_header("Benchmark: Search Performance");
    
    qwen35_vector_db_t *db = qwen35_db_create(DIMENSIONS, QWEN35_DIST_COSINE);
    if (!db) {
        printf("[X] Failed to create database\n");
        return;
    }
    
    printf("[OK] Populating database with %d vectors...\n", NUM_VECTORS);
    
    for (int i = 0; i < NUM_VECTORS; i++) {
        float vector[DIMENSIONS];
        generate_random_vector(vector, DIMENSIONS);
        qwen35_db_insert(db, i, vector, NULL, 0);
    }
    
    float query_vector[DIMENSIONS];
    generate_random_vector(query_vector, DIMENSIONS);
    
    const int num_searches = 100;
    clock_t start = clock();
    
    for (int i = 0; i < num_searches; i++) {
        int64_t result_ids[TOP_K];
        float result_distances[TOP_K];
        qwen35_db_search(db, query_vector, TOP_K, result_ids, result_distances);
    }
    
    clock_t end = clock();
    double elapsed = (double)(end - start) / CLOCKS_PER_SEC;
    double avg_time_ms = (elapsed / num_searches) * 1000.0;
    
    printf("[OK] Performed %d searches in %.3f seconds\n", num_searches, elapsed);
    printf("[OK] Average search time: %.3f ms\n", avg_time_ms);
    
    qwen35_db_destroy(db);
}

static void test_duplicate_insert(void) {
    print_test_header("Test: Duplicate Insert Prevention");
    
    qwen35_vector_db_t *db = qwen35_db_create(DIMENSIONS, QWEN35_DIST_COSINE);
    if (!db) {
        printf("[X] Failed to create database\n");
        return;
    }
    
    float vector[DIMENSIONS];
    generate_random_vector(vector, DIMENSIONS);
    
    if (qwen35_db_insert(db, 1, vector, NULL, 0) == 0) {
        printf("[OK] First insert succeeded\n");
    } else {
        printf("[X] First insert failed\n");
    }
    
    if (qwen35_db_insert(db, 1, vector, NULL, 0) == -1) {
        printf("[OK] Duplicate insert correctly rejected\n");
    } else {
        printf("[X] Duplicate insert should have been rejected\n");
    }
    
    qwen35_db_destroy(db);
}

int main(void) {
    printf("\n");
    printf("╔══════════════════════════════════════════════════════════╗\n");
    printf("║     Qwen35 Vector Database - Test Suite                  ║\n");
    printf("║     Version: %s                                      ║\n", qwen35_get_version());
    printf("╚══════════════════════════════════════════════════════════╝\n");
    printf("\n");
    
    srand((unsigned int)time(NULL));
    
    test_basic_operations();
    test_metadata();
    test_persistence();
    test_distance_metrics();
    test_duplicate_insert();
    benchmark_insert();
    benchmark_search();
    
    printf("\n");
    printf("╔══════════════════════════════════════════════════════════╗\n");
    printf("║              All Tests Completed                         ║\n");
    printf("╚══════════════════════════════════════════════════════════╝\n");
    printf("\n");
    
    return 0;
}
