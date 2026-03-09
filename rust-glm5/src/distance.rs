use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DistanceMetric {
    Cosine,
    Euclidean,
    DotProduct,
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
        }
    }
}

#[inline]
pub fn cosine_distance(a: &[f32], b: &[f32]) -> f32 {
    let len = a.len();
    let mut dot = 0.0_f64;
    let mut norm_a = 0.0_f64;
    let mut norm_b = 0.0_f64;
    
    let chunks = len / 8;
    let remainder = len % 8;
    
    for i in 0..chunks {
        let offset = i * 8;
        dot += (a[offset] as f64 * b[offset] as f64)
            + (a[offset + 1] as f64 * b[offset + 1] as f64)
            + (a[offset + 2] as f64 * b[offset + 2] as f64)
            + (a[offset + 3] as f64 * b[offset + 3] as f64)
            + (a[offset + 4] as f64 * b[offset + 4] as f64)
            + (a[offset + 5] as f64 * b[offset + 5] as f64)
            + (a[offset + 6] as f64 * b[offset + 6] as f64)
            + (a[offset + 7] as f64 * b[offset + 7] as f64);
        
        norm_a += (a[offset] as f64 * a[offset] as f64)
            + (a[offset + 1] as f64 * a[offset + 1] as f64)
            + (a[offset + 2] as f64 * a[offset + 2] as f64)
            + (a[offset + 3] as f64 * a[offset + 3] as f64)
            + (a[offset + 4] as f64 * a[offset + 4] as f64)
            + (a[offset + 5] as f64 * a[offset + 5] as f64)
            + (a[offset + 6] as f64 * a[offset + 6] as f64)
            + (a[offset + 7] as f64 * a[offset + 7] as f64);
        
        norm_b += (b[offset] as f64 * b[offset] as f64)
            + (b[offset + 1] as f64 * b[offset + 1] as f64)
            + (b[offset + 2] as f64 * b[offset + 2] as f64)
            + (b[offset + 3] as f64 * b[offset + 3] as f64)
            + (b[offset + 4] as f64 * b[offset + 4] as f64)
            + (b[offset + 5] as f64 * b[offset + 5] as f64)
            + (b[offset + 6] as f64 * b[offset + 6] as f64)
            + (b[offset + 7] as f64 * b[offset + 7] as f64);
    }
    
    for i in 0..remainder {
        let idx = chunks * 8 + i;
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
    
    let chunks = len / 8;
    let remainder = len % 8;
    
    for i in 0..chunks {
        let offset = i * 8;
        sum += {
            let d0 = (a[offset] - b[offset]) as f64;
            let d1 = (a[offset + 1] - b[offset + 1]) as f64;
            let d2 = (a[offset + 2] - b[offset + 2]) as f64;
            let d3 = (a[offset + 3] - b[offset + 3]) as f64;
            let d4 = (a[offset + 4] - b[offset + 4]) as f64;
            let d5 = (a[offset + 5] - b[offset + 5]) as f64;
            let d6 = (a[offset + 6] - b[offset + 6]) as f64;
            let d7 = (a[offset + 7] - b[offset + 7]) as f64;
            d0 * d0 + d1 * d1 + d2 * d2 + d3 * d3 + d4 * d4 + d5 * d5 + d6 * d6 + d7 * d7
        };
    }
    
    for i in 0..remainder {
        let idx = chunks * 8 + i;
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
    
    let chunks = len / 8;
    let remainder = len % 8;
    
    for i in 0..chunks {
        let offset = i * 8;
        sum += (a[offset] as f64 * b[offset] as f64)
            + (a[offset + 1] as f64 * b[offset + 1] as f64)
            + (a[offset + 2] as f64 * b[offset + 2] as f64)
            + (a[offset + 3] as f64 * b[offset + 3] as f64)
            + (a[offset + 4] as f64 * b[offset + 4] as f64)
            + (a[offset + 5] as f64 * b[offset + 5] as f64)
            + (a[offset + 6] as f64 * b[offset + 6] as f64)
            + (a[offset + 7] as f64 * b[offset + 7] as f64);
    }
    
    for i in 0..remainder {
        let idx = chunks * 8 + i;
        sum += a[idx] as f64 * b[idx] as f64;
    }
    
    sum as f32
}

#[inline]
pub fn magnitude(v: &[f32]) -> f32 {
    let len = v.len();
    let mut sum = 0.0_f64;
    
    let chunks = len / 8;
    let remainder = len % 8;
    
    for i in 0..chunks {
        let offset = i * 8;
        sum += (v[offset] as f64 * v[offset] as f64)
            + (v[offset + 1] as f64 * v[offset + 1] as f64)
            + (v[offset + 2] as f64 * v[offset + 2] as f64)
            + (v[offset + 3] as f64 * v[offset + 3] as f64)
            + (v[offset + 4] as f64 * v[offset + 4] as f64)
            + (v[offset + 5] as f64 * v[offset + 5] as f64)
            + (v[offset + 6] as f64 * v[offset + 6] as f64)
            + (v[offset + 7] as f64 * v[offset + 7] as f64);
    }
    
    for i in 0..remainder {
        let idx = chunks * 8 + i;
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
    fn test_large_vector() {
        let a: Vec<f32> = (0..128).map(|i| i as f32).collect();
        let b: Vec<f32> = (0..128).map(|i| (i + 1) as f32).collect();
        
        let dot = dot_product(&a, &b);
        assert!(dot > 0.0);
        
        let dist = euclidean_distance(&a, &b);
        assert!(dist > 0.0);
        
        let cos = cosine_distance(&a, &b);
        assert!(cos > 0.0 && cos <= 1.0);
    }
}
