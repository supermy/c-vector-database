use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DistanceMetric {
    Cosine,
    Euclidean,
    DotProduct,
    Manhattan,
}

impl Default for DistanceMetric {
    fn default() -> Self {
        Self::Cosine
    }
}

impl fmt::Display for DistanceMetric {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Cosine => write!(f, "cosine"),
            Self::Euclidean => write!(f, "euclidean"),
            Self::DotProduct => write!(f, "dot_product"),
            Self::Manhattan => write!(f, "manhattan"),
        }
    }
}

#[inline]
pub fn cosine_distance(a: &[f32], b: &[f32]) -> f32 {
    let len = a.len();
    let mut dot = 0.0_f64;
    let mut norm_a = 0.0_f64;
    let mut norm_b = 0.0_f64;

    let chunks = len / 16;
    let remainder = len % 16;

    for i in 0..chunks {
        let offset = i * 16;
        dot += (a[offset] as f64 * b[offset] as f64)
            + (a[offset + 1] as f64 * b[offset + 1] as f64)
            + (a[offset + 2] as f64 * b[offset + 2] as f64)
            + (a[offset + 3] as f64 * b[offset + 3] as f64)
            + (a[offset + 4] as f64 * b[offset + 4] as f64)
            + (a[offset + 5] as f64 * b[offset + 5] as f64)
            + (a[offset + 6] as f64 * b[offset + 6] as f64)
            + (a[offset + 7] as f64 * b[offset + 7] as f64)
            + (a[offset + 8] as f64 * b[offset + 8] as f64)
            + (a[offset + 9] as f64 * b[offset + 9] as f64)
            + (a[offset + 10] as f64 * b[offset + 10] as f64)
            + (a[offset + 11] as f64 * b[offset + 11] as f64)
            + (a[offset + 12] as f64 * b[offset + 12] as f64)
            + (a[offset + 13] as f64 * b[offset + 13] as f64)
            + (a[offset + 14] as f64 * b[offset + 14] as f64)
            + (a[offset + 15] as f64 * b[offset + 15] as f64);

        norm_a += (a[offset] as f64 * a[offset] as f64)
            + (a[offset + 1] as f64 * a[offset + 1] as f64)
            + (a[offset + 2] as f64 * a[offset + 2] as f64)
            + (a[offset + 3] as f64 * a[offset + 3] as f64)
            + (a[offset + 4] as f64 * a[offset + 4] as f64)
            + (a[offset + 5] as f64 * a[offset + 5] as f64)
            + (a[offset + 6] as f64 * a[offset + 6] as f64)
            + (a[offset + 7] as f64 * a[offset + 7] as f64)
            + (a[offset + 8] as f64 * a[offset + 8] as f64)
            + (a[offset + 9] as f64 * a[offset + 9] as f64)
            + (a[offset + 10] as f64 * a[offset + 10] as f64)
            + (a[offset + 11] as f64 * a[offset + 11] as f64)
            + (a[offset + 12] as f64 * a[offset + 12] as f64)
            + (a[offset + 13] as f64 * a[offset + 13] as f64)
            + (a[offset + 14] as f64 * a[offset + 14] as f64)
            + (a[offset + 15] as f64 * a[offset + 15] as f64);

        norm_b += (b[offset] as f64 * b[offset] as f64)
            + (b[offset + 1] as f64 * b[offset + 1] as f64)
            + (b[offset + 2] as f64 * b[offset + 2] as f64)
            + (b[offset + 3] as f64 * b[offset + 3] as f64)
            + (b[offset + 4] as f64 * b[offset + 4] as f64)
            + (b[offset + 5] as f64 * b[offset + 5] as f64)
            + (b[offset + 6] as f64 * b[offset + 6] as f64)
            + (b[offset + 7] as f64 * b[offset + 7] as f64)
            + (b[offset + 8] as f64 * b[offset + 8] as f64)
            + (b[offset + 9] as f64 * b[offset + 9] as f64)
            + (b[offset + 10] as f64 * b[offset + 10] as f64)
            + (b[offset + 11] as f64 * b[offset + 11] as f64)
            + (b[offset + 12] as f64 * b[offset + 12] as f64)
            + (b[offset + 13] as f64 * b[offset + 13] as f64)
            + (b[offset + 14] as f64 * b[offset + 14] as f64)
            + (b[offset + 15] as f64 * b[offset + 15] as f64);
    }

    for i in 0..remainder {
        let idx = chunks * 16 + i;
        dot += a[idx] as f64 * b[idx] as f64;
        norm_a += a[idx] as f64 * a[idx] as f64;
        norm_b += b[idx] as f64 * b[idx] as f64;
    }

    let norm_a = norm_a.sqrt();
    let norm_b = norm_b.sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }

    (dot / (norm_a * norm_b)) as f32
}

#[inline]
pub fn euclidean_distance(a: &[f32], b: &[f32]) -> f32 {
    let len = a.len();
    let mut sum = 0.0_f64;

    let chunks = len / 16;
    let remainder = len % 16;

    for i in 0..chunks {
        let offset = i * 16;
        sum += {
            let d0 = (a[offset] - b[offset]) as f64;
            let d1 = (a[offset + 1] - b[offset + 1]) as f64;
            let d2 = (a[offset + 2] - b[offset + 2]) as f64;
            let d3 = (a[offset + 3] - b[offset + 3]) as f64;
            let d4 = (a[offset + 4] - b[offset + 4]) as f64;
            let d5 = (a[offset + 5] - b[offset + 5]) as f64;
            let d6 = (a[offset + 6] - b[offset + 6]) as f64;
            let d7 = (a[offset + 7] - b[offset + 7]) as f64;
            let d8 = (a[offset + 8] - b[offset + 8]) as f64;
            let d9 = (a[offset + 9] - b[offset + 9]) as f64;
            let d10 = (a[offset + 10] - b[offset + 10]) as f64;
            let d11 = (a[offset + 11] - b[offset + 11]) as f64;
            let d12 = (a[offset + 12] - b[offset + 12]) as f64;
            let d13 = (a[offset + 13] - b[offset + 13]) as f64;
            let d14 = (a[offset + 14] - b[offset + 14]) as f64;
            let d15 = (a[offset + 15] - b[offset + 15]) as f64;
            d0 * d0 + d1 * d1 + d2 * d2 + d3 * d3 + d4 * d4 + d5 * d5 + d6 * d6 + d7 * d7 +
            d8 * d8 + d9 * d9 + d10 * d10 + d11 * d11 + d12 * d12 + d13 * d13 + d14 * d14 + d15 * d15
        };
    }

    for i in 0..remainder {
        let idx = chunks * 16 + i;
        let diff = (a[idx] - b[idx]) as f64;
        sum += diff * diff;
    }

    sum.sqrt() as f32
}

#[inline]
pub fn dot_product_distance(a: &[f32], b: &[f32]) -> f32 {
    dot_product(a, b)
}

#[inline]
pub fn dot_product(a: &[f32], b: &[f32]) -> f32 {
    let len = a.len();
    let mut sum = 0.0_f64;

    let chunks = len / 16;
    let remainder = len % 16;

    for i in 0..chunks {
        let offset = i * 16;
        sum += (a[offset] as f64 * b[offset] as f64)
            + (a[offset + 1] as f64 * b[offset + 1] as f64)
            + (a[offset + 2] as f64 * b[offset + 2] as f64)
            + (a[offset + 3] as f64 * b[offset + 3] as f64)
            + (a[offset + 4] as f64 * b[offset + 4] as f64)
            + (a[offset + 5] as f64 * b[offset + 5] as f64)
            + (a[offset + 6] as f64 * b[offset + 6] as f64)
            + (a[offset + 7] as f64 * b[offset + 7] as f64)
            + (a[offset + 8] as f64 * b[offset + 8] as f64)
            + (a[offset + 9] as f64 * b[offset + 9] as f64)
            + (a[offset + 10] as f64 * b[offset + 10] as f64)
            + (a[offset + 11] as f64 * b[offset + 11] as f64)
            + (a[offset + 12] as f64 * b[offset + 12] as f64)
            + (a[offset + 13] as f64 * b[offset + 13] as f64)
            + (a[offset + 14] as f64 * b[offset + 14] as f64)
            + (a[offset + 15] as f64 * b[offset + 15] as f64);
    }

    for i in 0..remainder {
        let idx = chunks * 16 + i;
        sum += a[idx] as f64 * b[idx] as f64;
    }

    sum as f32
}

#[inline]
pub fn manhattan_distance(a: &[f32], b: &[f32]) -> f32 {
    let len = a.len();
    let mut sum = 0.0_f64;

    let chunks = len / 16;
    let remainder = len % 16;

    for i in 0..chunks {
        let offset = i * 16;
        sum += ((a[offset] - b[offset]).abs()
            + (a[offset + 1] - b[offset + 1]).abs()
            + (a[offset + 2] - b[offset + 2]).abs()
            + (a[offset + 3] - b[offset + 3]).abs()
            + (a[offset + 4] - b[offset + 4]).abs()
            + (a[offset + 5] - b[offset + 5]).abs()
            + (a[offset + 6] - b[offset + 6]).abs()
            + (a[offset + 7] - b[offset + 7]).abs()
            + (a[offset + 8] - b[offset + 8]).abs()
            + (a[offset + 9] - b[offset + 9]).abs()
            + (a[offset + 10] - b[offset + 10]).abs()
            + (a[offset + 11] - b[offset + 11]).abs()
            + (a[offset + 12] - b[offset + 12]).abs()
            + (a[offset + 13] - b[offset + 13]).abs()
            + (a[offset + 14] - b[offset + 14]).abs()
            + (a[offset + 15] - b[offset + 15]).abs()) as f64;
    }

    for i in 0..remainder {
        let idx = chunks * 16 + i;
        sum += (a[idx] - b[idx]).abs() as f64;
    }

    sum as f32
}

#[inline]
pub fn magnitude(v: &[f32]) -> f32 {
    let len = v.len();
    let mut sum = 0.0_f64;

    let chunks = len / 16;
    let remainder = len % 16;

    for i in 0..chunks {
        let offset = i * 16;
        sum += (v[offset] as f64 * v[offset] as f64)
            + (v[offset + 1] as f64 * v[offset + 1] as f64)
            + (v[offset + 2] as f64 * v[offset + 2] as f64)
            + (v[offset + 3] as f64 * v[offset + 3] as f64)
            + (v[offset + 4] as f64 * v[offset + 4] as f64)
            + (v[offset + 5] as f64 * v[offset + 5] as f64)
            + (v[offset + 6] as f64 * v[offset + 6] as f64)
            + (v[offset + 7] as f64 * v[offset + 7] as f64)
            + (v[offset + 8] as f64 * v[offset + 8] as f64)
            + (v[offset + 9] as f64 * v[offset + 9] as f64)
            + (v[offset + 10] as f64 * v[offset + 10] as f64)
            + (v[offset + 11] as f64 * v[offset + 11] as f64)
            + (v[offset + 12] as f64 * v[offset + 12] as f64)
            + (v[offset + 13] as f64 * v[offset + 13] as f64)
            + (v[offset + 14] as f64 * v[offset + 14] as f64)
            + (v[offset + 15] as f64 * v[offset + 15] as f64);
    }

    for i in 0..remainder {
        let idx = chunks * 16 + i;
        sum += v[idx] as f64 * v[idx] as f64;
    }

    sum.sqrt() as f32
}

#[inline]
pub fn normalize(v: &mut [f32]) {
    let mag = magnitude(v);
    if mag > 1e-10 {
        let inv_mag = 1.0 / mag;
        for x in v.iter_mut() {
            *x *= inv_mag;
        }
    }
}

pub fn get_distance_fn(metric: DistanceMetric) -> fn(&[f32], &[f32]) -> f32 {
    match metric {
        DistanceMetric::Cosine => cosine_distance,
        DistanceMetric::Euclidean => euclidean_distance,
        DistanceMetric::DotProduct => dot_product_distance,
        DistanceMetric::Manhattan => manhattan_distance,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosine_distance() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![0.0, 1.0, 0.0];
        let dist = cosine_distance(&a, &b);
        assert!((dist - 0.0).abs() < 1e-6);

        let c = vec![1.0, 1.0, 0.0];
        let dist2 = cosine_distance(&a, &c);
        assert!((dist2 - 0.70710678).abs() < 1e-6);
    }

    #[test]
    fn test_euclidean_distance() {
        let a = vec![0.0, 0.0, 0.0];
        let b = vec![1.0, 1.0, 1.0];
        let dist = euclidean_distance(&a, &b);
        assert!((dist - 1.7320508).abs() < 1e-6);
    }

    #[test]
    fn test_dot_product() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![4.0, 5.0, 6.0];
        let dot = dot_product(&a, &b);
        assert_eq!(dot, 32.0);
    }

    #[test]
    fn test_manhattan_distance() {
        let a = vec![0.0, 0.0, 0.0];
        let b = vec![1.0, 2.0, 3.0];
        let dist = manhattan_distance(&a, &b);
        assert_eq!(dist, 6.0);
    }

    #[test]
    fn test_large_vector() {
        let a: Vec<f32> = (0..256).map(|i| i as f32).collect();
        let b: Vec<f32> = (0..256).map(|i| (i + 1) as f32).collect();

        let dot = dot_product(&a, &b);
        assert!(dot > 0.0);

        let dist = euclidean_distance(&a, &b);
        assert!(dist > 0.0);

        let cos = cosine_distance(&a, &b);
        assert!(cos > 0.0 && cos <= 1.0);
    }
}
