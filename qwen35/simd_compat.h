/**
 * Cross-platform SIMD abstraction layer
 * Supports: x86_64 (SSE/AVX), ARM64 (NEON), and fallback scalar
 */

#ifndef QWEN35_SIMD_COMPAT_H
#define QWEN35_SIMD_COMPAT_H

#include <stdint.h>
#include <stddef.h>
#include <math.h>

// Platform detection
#if defined(__x86_64__) || defined(_M_X64) || defined(__i386) || defined(_M_IX86)
    #define QWEN35_ARCH_X86
    #if defined(__AVX512F__)
        #define QWEN35_SIMD_AVX512
    #elif defined(__AVX2__)
        #define QWEN35_SIMD_AVX2
    #elif defined(__AVX__)
        #define QWEN35_SIMD_AVX
    #elif defined(__SSE4_2__)
        #define QWEN35_SIMD_SSE42
    #elif defined(__SSE2__)
        #define QWEN35_SIMD_SSE2
    #endif
#elif defined(__aarch64__) || defined(_M_ARM64)
    #define QWEN35_ARCH_ARM64
    #define QWEN35_SIMD_NEON
#elif defined(__arm__) || defined(_M_ARM)
    #define QWEN35_ARCH_ARM
    #define QWEN35_SIMD_NEON
#else
    #define QWEN35_ARCH_UNKNOWN
#endif

// Compiler-specific macros
#if defined(__GNUC__) || defined(__clang__)
    #define QWEN35_LIKELY(x) __builtin_expect(!!(x), 1)
    #define QWEN35_UNLIKELY(x) __builtin_expect(!!(x), 0)
    #define QWEN35_ALIGNED(x) __attribute__((aligned(x)))
#else
    #define QWEN35_LIKELY(x) (x)
    #define QWEN35_UNLIKELY(x) (x)
    #define QWEN35_ALIGNED(x)
#endif

// Include appropriate SIMD headers
#ifdef QWEN35_ARCH_X86
    #include <xmmintrin.h>  // SSE
    #include <emmintrin.h>  // SSE2
    #include <pmmintrin.h>  // SSE3
    #ifdef QWEN35_SIMD_SSE42
        #include <smmintrin.h>  // SSE4.2
    #endif
    #ifdef QWEN35_SIMD_AVX
        #include <immintrin.h>  // AVX
    #endif
    #ifdef QWEN35_SIMD_AVX2
        #include <immintrin.h>  // AVX2
    #endif
    #ifdef QWEN35_SIMD_AVX512
        #include <immintrin.h>  // AVX512
    #endif
#elif defined(QWEN35_SIMD_NEON)
    #if defined(__APPLE__)
        #include <arm_neon.h>
    #elif defined(__linux__)
        #include <arm_neon.h>
    #endif
#endif

// SIMD width definitions
#if !defined(QWEN35_SIMD_WIDTH)
#ifdef QWEN35_SIMD_AVX512
    #define QWEN35_SIMD_WIDTH 16
    #define QWEN35_SIMD_ALIGNMENT 64
#elif defined(QWEN35_SIMD_AVX) || defined(QWEN35_SIMD_AVX2)
    #define QWEN35_SIMD_WIDTH 8
    #define QWEN35_SIMD_ALIGNMENT 32
#elif defined(QWEN35_SIMD_SSE) || defined(QWEN35_SIMD_SSE2) || defined(QWEN35_SIMD_SSE42)
    #define QWEN35_SIMD_WIDTH 4
    #define QWEN35_SIMD_ALIGNMENT 16
#elif defined(QWEN35_SIMD_NEON)
    #define QWEN35_SIMD_WIDTH 4
    #define QWEN35_SIMD_ALIGNMENT 16
#else
    #define QWEN35_SIMD_WIDTH 1
    #define QWEN35_SIMD_ALIGNMENT 1
#endif
#else
    // QWEN35_SIMD_WIDTH already defined, define alignment too
    #if QWEN35_SIMD_WIDTH >= 16
        #define QWEN35_SIMD_ALIGNMENT 64
    #elif QWEN35_SIMD_WIDTH >= 8
        #define QWEN35_SIMD_ALIGNMENT 32
    #elif QWEN35_SIMD_WIDTH >= 4
        #define QWEN35_SIMD_ALIGNMENT 16
    #else
        #define QWEN35_SIMD_ALIGNMENT 1
    #endif
#endif

// Function declarations
#ifdef __cplusplus
extern "C" {
#endif

// Dot product with automatic SIMD selection
static inline float qwen35_dot_product_simd(const float *a, const float *b, size_t dim) {
    float sum = 0.0f;
    size_t i = 0;
    
#if defined(QWEN35_SIMD_AVX512)
    // AVX-512: Process 16 floats at once
    __m512 sum_vec = _mm512_setzero_ps();
    for (; i + 15 < dim; i += 16) {
        __m512 va = _mm512_loadu_ps(&a[i]);
        __m512 vb = _mm512_loadu_ps(&b[i]);
        sum_vec = _mm512_fmadd_ps(va, vb, sum_vec);
    }
    sum += _mm512_reduce_add_ps(sum_vec);
    
#elif defined(QWEN35_SIMD_AVX2)
    // AVX2: Process 8 floats at once
    __m256 sum_vec = _mm256_setzero_ps();
    for (; i + 7 < dim; i += 8) {
        __m256 va = _mm256_loadu_ps(&a[i]);
        __m256 vb = _mm256_loadu_ps(&b[i]);
        #ifdef __FMA__
        sum_vec = _mm256_fmadd_ps(va, vb, sum_vec);
        #else
        sum_vec = _mm256_add_ps(sum_vec, _mm256_mul_ps(va, vb));
        #endif
    }
    __m128 sum_lo = _mm_add_ps(_mm256_castps256_ps128(sum_vec), _mm256_extractf128_ps(sum_vec, 1));
    sum += _mm_cvtss_f32(sum_lo);
    
#elif defined(QWEN35_SIMD_AVX)
    // AVX: Process 8 floats at once
    __m256 sum_vec = _mm256_setzero_ps();
    for (; i + 7 < dim; i += 8) {
        __m256 va = _mm256_loadu_ps(&a[i]);
        __m256 vb = _mm256_loadu_ps(&b[i]);
        sum_vec = _mm256_add_ps(sum_vec, _mm256_mul_ps(va, vb));
    }
    __m128 sum_lo = _mm_add_ps(_mm256_castps256_ps128(sum_vec), _mm256_extractf128_ps(sum_vec, 1));
    sum += _mm_cvtss_f32(sum_lo);
    
#elif defined(QWEN35_SIMD_SSE42) || defined(QWEN35_SIMD_SSE2)
    // SSE2/SSE4.2: Process 4 floats at once
    __m128 sum_vec = _mm_setzero_ps();
    for (; i + 3 < dim; i += 4) {
        __m128 va = _mm_loadu_ps(&a[i]);
        __m128 vb = _mm_loadu_ps(&b[i]);
        sum_vec = _mm_add_ps(sum_vec, _mm_mul_ps(va, vb));
    }
    __m128 tmp = sum_vec;
    tmp = _mm_hadd_ps(tmp, tmp);
    tmp = _mm_hadd_ps(tmp, tmp);
    sum += _mm_cvtss_f32(tmp);
    
#elif defined(QWEN35_SIMD_NEON)
    // ARM NEON: Process 4 floats at once
    float32x4_t sum_vec = vdupq_n_f32(0.0f);
    for (; i + 3 < dim; i += 4) {
        float32x4_t va = vld1q_f32(&a[i]);
        float32x4_t vb = vld1q_f32(&b[i]);
        sum_vec = vmlaq_f32(sum_vec, va, vb);
    }
    sum += vaddvq_f32(sum_vec);
    
#else
    // Fallback: Scalar implementation
    (void)i; // Suppress unused variable warning
    
#endif
    
    // Handle remaining elements
    for (; i < dim; i++) {
        sum += a[i] * b[i];
    }
    
    return sum;
}

// Euclidean distance squared with automatic SIMD selection
static inline float qwen35_euclidean_squared_simd(const float *a, const float *b, size_t dim) {
    float sum = 0.0f;
    size_t i = 0;
    
#if defined(QWEN35_SIMD_AVX512)
    // AVX-512: Process 16 floats at once
    __m512 sum_vec = _mm512_setzero_ps();
    for (; i + 15 < dim; i += 16) {
        __m512 va = _mm512_loadu_ps(&a[i]);
        __m512 vb = _mm512_loadu_ps(&b[i]);
        __m512 diff = _mm512_sub_ps(va, vb);
        sum_vec = _mm512_fmadd_ps(diff, diff, sum_vec);
    }
    sum += _mm512_reduce_add_ps(sum_vec);
    
#elif defined(QWEN35_SIMD_AVX2)
    // AVX2: Process 8 floats at once
    __m256 sum_vec = _mm256_setzero_ps();
    for (; i + 7 < dim; i += 8) {
        __m256 va = _mm256_loadu_ps(&a[i]);
        __m256 vb = _mm256_loadu_ps(&b[i]);
        __m256 diff = _mm256_sub_ps(va, vb);
        #ifdef __FMA__
        sum_vec = _mm256_fmadd_ps(diff, diff, sum_vec);
        #else
        sum_vec = _mm256_add_ps(sum_vec, _mm256_mul_ps(diff, diff));
        #endif
    }
    __m128 sum_lo = _mm_add_ps(_mm256_castps256_ps128(sum_vec), _mm256_extractf128_ps(sum_vec, 1));
    sum += _mm_cvtss_f32(sum_lo);
    
#elif defined(QWEN35_SIMD_AVX)
    // AVX: Process 8 floats at once
    __m256 sum_vec = _mm256_setzero_ps();
    for (; i + 7 < dim; i += 8) {
        __m256 va = _mm256_loadu_ps(&a[i]);
        __m256 vb = _mm256_loadu_ps(&b[i]);
        __m256 diff = _mm256_sub_ps(va, vb);
        sum_vec = _mm256_add_ps(sum_vec, _mm256_mul_ps(diff, diff));
    }
    __m128 sum_lo = _mm_add_ps(_mm256_castps256_ps128(sum_vec), _mm256_extractf128_ps(sum_vec, 1));
    sum += _mm_cvtss_f32(sum_lo);
    
#elif defined(QWEN35_SIMD_SSE42) || defined(QWEN35_SIMD_SSE2)
    // SSE2/SSE4.2: Process 4 floats at once
    __m128 sum_vec = _mm_setzero_ps();
    for (; i + 3 < dim; i += 4) {
        __m128 va = _mm_loadu_ps(&a[i]);
        __m128 vb = _mm_loadu_ps(&b[i]);
        __m128 diff = _mm_sub_ps(va, vb);
        sum_vec = _mm_add_ps(sum_vec, _mm_mul_ps(diff, diff));
    }
    __m128 tmp = sum_vec;
    tmp = _mm_hadd_ps(tmp, tmp);
    tmp = _mm_hadd_ps(tmp, tmp);
    sum += _mm_cvtss_f32(tmp);
    
#elif defined(QWEN35_SIMD_NEON)
    // ARM NEON: Process 4 floats at once
    float32x4_t sum_vec = vdupq_n_f32(0.0f);
    for (; i + 3 < dim; i += 4) {
        float32x4_t va = vld1q_f32(&a[i]);
        float32x4_t vb = vld1q_f32(&b[i]);
        float32x4_t diff = vsubq_f32(va, vb);
        sum_vec = vfmaq_f32(sum_vec, diff, diff);
    }
    sum += vaddvq_f32(sum_vec);
    
#else
    // Fallback: Scalar implementation
    (void)i;
    
#endif
    
    // Handle remaining elements
    for (; i < dim; i++) {
        float diff = a[i] - b[i];
        sum += diff * diff;
    }
    
    return sum;
}

// Vector magnitude (L2 norm) with SIMD
static inline float qwen35_magnitude_simd(const float *a, size_t dim) {
    return sqrtf(qwen35_euclidean_squared_simd(a, a, dim));
}

// Vector normalization with SIMD
static inline void qwen35_normalize_simd(float *a, size_t dim) {
    float mag = qwen35_magnitude_simd(a, dim);
    if (mag > 1e-10f) {
        float inv_mag = 1.0f / mag;
        
#if defined(QWEN35_SIMD_AVX512)
        __m512 vinv = _mm512_set1_ps(inv_mag);
        for (size_t i = 0; i < dim; i += 16) {
            __m512 va = _mm512_loadu_ps(&a[i]);
            _mm512_storeu_ps(&a[i], _mm512_mul_ps(va, vinv));
        }
#elif defined(QWEN35_SIMD_AVX)
        __m256 vinv = _mm256_set1_ps(inv_mag);
        for (size_t i = 0; i < dim; i += 8) {
            __m256 va = _mm256_loadu_ps(&a[i]);
            _mm256_storeu_ps(&a[i], _mm256_mul_ps(va, vinv));
        }
#elif defined(QWEN35_SIMD_SSE2)
        __m128 vinv = _mm_set1_ps(inv_mag);
        for (size_t i = 0; i < dim; i += 4) {
            __m128 va = _mm_loadu_ps(&a[i]);
            _mm_storeu_ps(&a[i], _mm_mul_ps(va, vinv));
        }
#elif defined(QWEN35_SIMD_NEON)
        float32x4_t vinv = vdupq_n_f32(inv_mag);
        for (size_t i = 0; i < dim; i += 4) {
            float32x4_t va = vld1q_f32(&a[i]);
            vst1q_f32(&a[i], vmulq_f32(va, vinv));
        }
#else
        for (size_t i = 0; i < dim; i++) {
            a[i] *= inv_mag;
        }
#endif
    }
}

// Get SIMD width at runtime
static inline int qwen35_get_simd_width(void) {
    return QWEN35_SIMD_WIDTH;
}

// Get SIMD alignment requirement
static inline int qwen35_get_simd_alignment(void) {
    return QWEN35_SIMD_ALIGNMENT;
}

#ifdef __cplusplus
}
#endif

#endif // QWEN35_SIMD_COMPAT_H
