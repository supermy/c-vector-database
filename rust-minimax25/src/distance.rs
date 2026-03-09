use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DistanceMetric {
    #[default]
    Cosine,
    Euclidean,
    DotProduct,
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
pub fn dot_product(a: &[f32], b: &[f32]) -> f32 {
    let mut sum = 0.0f32;
    for i in 0..a.len() {
        sum += a[i] * b[i];
    }
    sum
}

#[inline]
pub fn euclidean_distance(a: &[f32], b: &[f32]) -> f32 {
    let sum: f32 = a.iter()
        .zip(b.iter())
        .map(|(x, y)| {
            let diff = x - y;
            diff * diff
        })
        .sum();
    sum.sqrt()
}

#[inline]
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot = dot_product(a, b);
    let norm_a = magnitude(a);
    let norm_b = magnitude(b);
    
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    
    dot / (norm_a * norm_b)
}

#[inline]
pub fn cosine_distance(a: &[f32], b: &[f32]) -> f32 {
    1.0 - cosine_similarity(a, b)
}

#[inline]
pub fn dot_product_distance(a: &[f32], b: &[f32]) -> f32 {
    -dot_product(a, b)
}

#[inline]
pub fn magnitude(v: &[f32]) -> f32 {
    dot_product(v, v).sqrt()
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

#[inline]
pub fn compute_distance(a: &[f32], b: &[f32], metric: DistanceMetric) -> f32 {
    match metric {
        DistanceMetric::Cosine => cosine_distance(a, b),
        DistanceMetric::Euclidean => euclidean_distance(a, b),
        DistanceMetric::DotProduct => dot_product_distance(a, b),
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
        assert!((dist - 1.0).abs() < 1e-6);
        
        let c = vec![1.0, 1.0, 0.0];
        let dist2 = cosine_distance(&a, &c);
        assert!((dist2 - 0.292893).abs() < 1e-5);
    }
    
    #[test]
    fn test_euclidean_distance() {
        let a = vec![0.0, 0.0, 0.0];
        let b = vec![1.0, 1.0, 1.0];
        let dist = euclidean_distance(&a, &b);
        assert!((dist - 1.7320508).abs() < 1e-5);
    }
    
    #[test]
    fn test_dot_product() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![4.0, 5.0, 6.0];
        let dot = dot_product(&a, &b);
        assert_eq!(dot, 32.0);
    }
}
