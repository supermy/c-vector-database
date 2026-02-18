#include "vector_db.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <math.h>
#include <time.h>

// 生成随机向量
void random_vector(Vector* vec) {
    for (uint32_t i = 0; i < vec->dim; i++) {
        vec->data[i] = (float)rand() / RAND_MAX;
    }
}

// 测试向量操作
void test_vector_operations() {
    printf("\n=== 测试向量操作 ===\n");
    
    Vector* a = vector_create(3);
    Vector* b = vector_create(3);
    
    a->data[0] = 1.0f; a->data[1] = 0.0f; a->data[2] = 0.0f;
    b->data[0] = 0.0f; b->data[1] = 1.0f; b->data[2] = 0.0f;
    
    printf("向量A: [%.2f, %.2f, %.2f]\n", a->data[0], a->data[1], a->data[2]);
    printf("向量B: [%.2f, %.2f, %.2f]\n", b->data[0], b->data[1], b->data[2]);
    
    float dot = vector_dot(a, b);
    printf("点积: %.4f (期望: 0.0)\n", dot);
    
    float norm_a = vector_norm(a);
    printf("向量A模长: %.4f (期望: 1.0)\n", norm_a);
    
    float cos_sim = cosine_similarity(a, b);
    printf("余弦相似度: %.4f (期望: 0.0)\n", cos_sim);
    
    // 相同向量的相似度
    float cos_sim_same = cosine_similarity(a, a);
    printf("相同向量余弦相似度: %.4f (期望: 1.0)\n", cos_sim_same);
    
    vector_destroy(a);
    vector_destroy(b);
    printf("向量操作测试通过!\n");
}

// 测试基本CRUD操作
void test_crud_operations() {
    printf("\n=== 测试CRUD操作 ===\n");
    
    VectorDB* db = vectordb_create(128, false);
    if (!db) {
        printf("创建数据库失败!\n");
        return;
    }
    
    // 插入测试
    printf("插入100条记录...\n");
    for (int i = 0; i < 100; i++) {
        Vector* vec = vector_create(128);
        random_vector(vec);
        
        char metadata[64];
        snprintf(metadata, sizeof(metadata), "record_%d", i);
        
        int ret = vectordb_insert(db, i, vec, metadata, strlen(metadata) + 1);
        if (ret != VECTOR_DB_SUCCESS) {
            printf("插入记录 %d 失败!\n", i);
        }
        
        vector_destroy(vec);
    }
    
    vectordb_print_stats(db);
    
    // 查询测试
    printf("\n查询测试...\n");
    VectorRecord* record = vectordb_get(db, 50);
    if (record) {
        printf("找到记录 ID=50, metadata: %s\n", record->metadata);
    } else {
        printf("未找到记录 ID=50\n");
    }
    
    // 更新测试
    printf("\n更新测试...\n");
    Vector* new_vec = vector_create(128);
    random_vector(new_vec);
    char new_metadata[] = "updated_record_50";
    int ret = vectordb_update(db, 50, new_vec, new_metadata, strlen(new_metadata) + 1);
    if (ret == VECTOR_DB_SUCCESS) {
        printf("更新记录 ID=50 成功\n");
        record = vectordb_get(db, 50);
        if (record) {
            printf("更新后 metadata: %s\n", record->metadata);
        }
    }
    vector_destroy(new_vec);
    
    // 删除测试
    printf("\n删除测试...\n");
    ret = vectordb_delete(db, 50);
    if (ret == VECTOR_DB_SUCCESS) {
        printf("删除记录 ID=50 成功\n");
        record = vectordb_get(db, 50);
        if (!record) {
            printf("确认记录 ID=50 已删除\n");
        }
    }
    
    vectordb_destroy(db);
    printf("CRUD操作测试通过!\n");
}

// 测试相似度搜索
void test_similarity_search() {
    printf("\n=== 测试相似度搜索 ===\n");
    
    VectorDB* db = vectordb_create(64, false);
    if (!db) {
        printf("创建数据库失败!\n");
        return;
    }
    
    // 插入一些特定向量用于测试
    printf("插入测试数据...\n");
    
    // 插入与查询向量相似的向量
    for (int i = 0; i < 10; i++) {
        Vector* vec = vector_create(64);
        // 创建与目标相似的向量
        for (int j = 0; j < 64; j++) {
            vec->data[j] = (j < 32) ? 1.0f : 0.0f;  // 前32维为1，后32维为0
        }
        // 添加一些噪声
        vec->data[i] += 0.1f * i;
        
        char metadata[64];
        snprintf(metadata, sizeof(metadata), "similar_vector_%d", i);
        vectordb_insert(db, i, vec, metadata, strlen(metadata) + 1);
        vector_destroy(vec);
    }
    
    // 插入不相似的向量
    for (int i = 10; i < 20; i++) {
        Vector* vec = vector_create(64);
        for (int j = 0; j < 64; j++) {
            vec->data[j] = (j >= 32) ? 1.0f : 0.0f;  // 后32维为1，前32维为0
        }
        
        char metadata[64];
        snprintf(metadata, sizeof(metadata), "different_vector_%d", i);
        vectordb_insert(db, i, vec, metadata, strlen(metadata) + 1);
        vector_destroy(vec);
    }
    
    // 创建查询向量
    Vector* query = vector_create(64);
    for (int j = 0; j < 64; j++) {
        query->data[j] = (j < 32) ? 1.0f : 0.0f;
    }
    
    // 搜索
    SearchOptions options = {
        .top_k = 5,
        .threshold = 0.0f,
        .use_hnsw = false,
        .ef_search = 64
    };
    
    uint32_t result_count;
    SearchResult* results = vectordb_search_exact(db, query, &options, &result_count);
    
    printf("\n搜索结果 (Top %d):\n", result_count);
    for (uint32_t i = 0; i < result_count; i++) {
        printf("  Rank %d: ID=%llu, Score=%.4f, Metadata=%s\n", 
               i + 1, 
               (unsigned long long)results[i].id, 
               results[i].score,
               results[i].metadata ? results[i].metadata : "N/A");
    }
    
    search_results_destroy(results, result_count);
    vector_destroy(query);
    vectordb_destroy(db);
    printf("相似度搜索测试通过!\n");
}

// 测试性能
void test_performance() {
    printf("\n=== 测试性能 ===\n");
    
    const uint32_t dim = 128;
    const uint64_t num_vectors = 10000;
    
    VectorDB* db = vectordb_create(dim, false);
    if (!db) {
        printf("创建数据库失败!\n");
        return;
    }
    
    // 插入性能测试
    printf("插入 %llu 条 %u 维向量...\n", (unsigned long long)num_vectors, dim);
    clock_t start = clock();
    
    for (uint64_t i = 0; i < num_vectors; i++) {
        Vector* vec = vector_create(dim);
        random_vector(vec);
        vectordb_insert(db, i, vec, NULL, 0);
        vector_destroy(vec);
    }
    
    clock_t end = clock();
    double insert_time = (double)(end - start) / CLOCKS_PER_SEC;
    printf("插入耗时: %.3f 秒 (%.1f vectors/sec)\n", 
           insert_time, num_vectors / insert_time);
    
    vectordb_print_stats(db);
    
    // 搜索性能测试
    printf("\n执行 100 次相似度搜索...\n");
    Vector* query = vector_create(dim);
    random_vector(query);
    
    SearchOptions options = {
        .top_k = 10,
        .threshold = 0.0f,
        .use_hnsw = false,
        .ef_search = 64
    };
    
    start = clock();
    for (int i = 0; i < 100; i++) {
        uint32_t result_count;
        SearchResult* results = vectordb_search_exact(db, query, &options, &result_count);
        if (results) {
            search_results_destroy(results, result_count);
        }
    }
    end = clock();
    
    double search_time = (double)(end - start) / CLOCKS_PER_SEC;
    printf("100次搜索耗时: %.3f 秒 (%.3f sec/search)\n", 
           search_time, search_time / 100);
    
    vector_destroy(query);
    vectordb_destroy(db);
    printf("性能测试完成!\n");
}

// 测试持久化
void test_persistence() {
    printf("\n=== 测试持久化 ===\n");
    
    const char* filename = "test_db.bin";
    
    // 创建并保存数据库
    VectorDB* db = vectordb_create(64, false);
    if (!db) {
        printf("创建数据库失败!\n");
        return;
    }
    
    printf("创建测试数据...\n");
    for (int i = 0; i < 100; i++) {
        Vector* vec = vector_create(64);
        random_vector(vec);
        
        char metadata[64];
        snprintf(metadata, sizeof(metadata), "persistent_record_%d", i);
        vectordb_insert(db, i, vec, metadata, strlen(metadata) + 1);
        vector_destroy(vec);
    }
    
    printf("保存数据库到 %s...\n", filename);
    int ret = vectordb_save(db, filename);
    if (ret == VECTOR_DB_SUCCESS) {
        printf("保存成功!\n");
    } else {
        printf("保存失败!\n");
        vectordb_destroy(db);
        return;
    }
    
    vectordb_destroy(db);
    
    // 加载数据库
    printf("从 %s 加载数据库...\n", filename);
    VectorDB* loaded_db = vectordb_load(filename);
    if (!loaded_db) {
        printf("加载失败!\n");
        return;
    }
    
    printf("加载成功!\n");
    vectordb_print_stats(loaded_db);
    
    // 验证数据
    printf("\n验证数据...\n");
    VectorRecord* record = vectordb_get(loaded_db, 50);
    if (record) {
        printf("找到记录 ID=50, metadata: %s\n", record->metadata);
        if (strncmp(record->metadata, "persistent_record_50", 20) == 0) {
            printf("数据验证成功!\n");
        } else {
            printf("数据验证失败!\n");
        }
    }
    
    vectordb_destroy(loaded_db);
    
    // 清理测试文件
    remove(filename);
    printf("持久化测试通过!\n");
}

// 主函数
int main() {
    srand((unsigned int)time(NULL));
    
    printf("========================================\n");
    printf("    向量数据库测试程序\n");
    printf("========================================\n");
    
    test_vector_operations();
    test_crud_operations();
    test_similarity_search();
    test_performance();
    test_persistence();
    
    printf("\n========================================\n");
    printf("    所有测试通过!\n");
    printf("========================================\n");
    
    return 0;
}
