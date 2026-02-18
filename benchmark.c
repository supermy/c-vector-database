#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#include <math.h>

// 模拟kimi25的向量操作
typedef struct {
    float* data;
    uint32_t dim;
} KVector;

KVector* kvector_create(uint32_t dim) {
    KVector* v = malloc(sizeof(KVector));
    v->data = calloc(dim, sizeof(float));
    v->dim = dim;
    return v;
}

void kvector_destroy(KVector* v) {
    if (v) { free(v->data); free(v); }
}

float kvector_dot(const KVector* a, const KVector* b) {
    float sum = 0;
    for (uint32_t i = 0; i < a->dim; i++) sum += a->data[i] * b->data[i];
    return sum;
}

float kvector_norm(const KVector* v) {
    float sum = 0;
    for (uint32_t i = 0; i < v->dim; i++) sum += v->data[i] * v->data[i];
    return sqrtf(sum);
}

float kcosine_similarity(const KVector* a, const KVector* b) {
    float na = kvector_norm(a), nb = kvector_norm(b);
    if (na == 0 || nb == 0) return -1;
    return kvector_dot(a, b) / (na * nb);
}

// 模拟minimax25的向量操作
typedef struct {
    float* data;
    uint32_t dim;
} MVector;

MVector* mvector_new(uint32_t dim) {
    MVector* v = malloc(sizeof(MVector));
    v->data = calloc(dim, sizeof(float));
    v->dim = dim;
    return v;
}

void mvector_free(MVector* v) {
    if (v) { free(v->data); free(v); }
}

float mvector_dot_product(const MVector* a, const MVector* b) {
    float sum = 0;
    for (uint32_t i = 0; i < a->dim; i++) sum += a->data[i] * b->data[i];
    return sum;
}

float mvector_magnitude(const MVector* v) {
    float sum = 0;
    for (uint32_t i = 0; i < v->dim; i++) sum += v->data[i] * v->data[i];
    return sqrtf(sum);
}

float mvector_cosine_similarity(const MVector* a, const MVector* b) {
    float ma = mvector_magnitude(a), mb = mvector_magnitude(b);
    if (ma == 0 || mb == 0) return -1;
    return mvector_dot_product(a, b) / (ma * mb);
}

// 生成随机向量
void random_floats(float* data, uint32_t dim) {
    for (uint32_t i = 0; i < dim; i++) {
        data[i] = (float)rand() / RAND_MAX;
    }
}

// 性能测试
int main() {
    srand(time(NULL));
    
    printf("========================================\n");
    printf("   向量数据库性能对比测试\n");
    printf("========================================\n\n");
    
    const uint32_t dim = 128;
    const uint64_t num_vectors = 10000;
    const int num_searches = 100;
    
    printf("测试配置:\n");
    printf("  向量维度: %u\n", dim);
    printf("  向量数量: %llu\n", (unsigned long long)num_vectors);
    printf("  搜索次数: %d\n\n", num_searches);
    
    // kimi25风格测试
    printf("【kimi25 版本】\n");
    
    KVector** kvectors = malloc(num_vectors * sizeof(KVector*));
    for (uint64_t i = 0; i < num_vectors; i++) {
        kvectors[i] = kvector_create(dim);
        random_floats(kvectors[i]->data, dim);
    }
    
    KVector* kquery = kvector_create(dim);
    random_floats(kquery->data, dim);
    
    clock_t start = clock();
    for (int s = 0; s < num_searches; s++) {
        float best_score = -1;
        uint64_t best_id = 0;
        for (uint64_t i = 0; i < num_vectors; i++) {
            float score = kcosine_similarity(kquery, kvectors[i]);
            if (score > best_score) {
                best_score = score;
                best_id = i;
            }
        }
    }
    clock_t end = clock();
    double ktime = (double)(end - start) / CLOCKS_PER_SEC;
    
    printf("  搜索耗时: %.3f 秒\n", ktime);
    printf("  单次搜索: %.3f ms\n", ktime * 1000 / num_searches);
    printf("  向量/秒: %.0f\n\n", (double)num_vectors * num_searches / ktime);
    
    // minimax25风格测试
    printf("【minimax25 版本】\n");
    
    MVector** mvectors = malloc(num_vectors * sizeof(MVector*));
    for (uint64_t i = 0; i < num_vectors; i++) {
        mvectors[i] = mvector_new(dim);
        random_floats(mvectors[i]->data, dim);
    }
    
    MVector* mquery = mvector_new(dim);
    random_floats(mquery->data, dim);
    
    start = clock();
    for (int s = 0; s < num_searches; s++) {
        float best_score = -1;
        uint64_t best_id = 0;
        for (uint64_t i = 0; i < num_vectors; i++) {
            float score = mvector_cosine_similarity(mquery, mvectors[i]);
            if (score > best_score) {
                best_score = score;
                best_id = i;
            }
        }
    }
    end = clock();
    double mtime = (double)(end - start) / CLOCKS_PER_SEC;
    
    printf("  搜索耗时: %.3f 秒\n", mtime);
    printf("  单次搜索: %.3f ms\n", mtime * 1000 / num_searches);
    printf("  向量/秒: %.0f\n\n", (double)num_vectors * num_searches / mtime);
    
    // 对比结果
    printf("【性能对比】\n");
    if (ktime < mtime) {
        printf("  kimi25 更快, 领先 %.1f%%\n", (mtime - ktime) / mtime * 100);
    } else {
        printf("  minimax25 更快, 领先 %.1f%%\n", (ktime - mtime) / ktime * 100);
    }
    
    // 清理
    for (uint64_t i = 0; i < num_vectors; i++) {
        kvector_destroy(kvectors[i]);
        mvector_free(mvectors[i]);
    }
    free(kvectors);
    free(mvectors);
    kvector_destroy(kquery);
    mvector_free(mquery);
    
    return 0;
}
