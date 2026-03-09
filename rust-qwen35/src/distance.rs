#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DistanceMetric {
    Cosine,
    Euclidean,
    DotProduct,
    Manhattan,
}

impl Default for DistanceMetric {
    fn default() -> Self {
        DistanceMetric::Cosine
    }
}

#[inline]
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot = dot_product(a, b);
    let norm_a = magnitude(a);
    let norm_b = magnitude(b);
    
    if norm_a < 1e-8 || norm_b < 1e-8 {
        return 0.0;
    }
    
    dot / (norm_a * norm_b)
}

#[inline]
pub fn cosine_distance(a: &[f32], b: &[f32]) -> f32 {
    1.0 - cosine_similarity(a, b)
}

#[inline]
pub fn euclidean_distance(a: &[f32], b: &[f32]) -> f32 {
    let mut sum = 0.0f32;
    let len = a.len();
    
    for i in 0..len {
        let diff = a[i] - b[i];
        sum += diff * diff;
    }
    
    sum.sqrt()
}

#[inline]
pub fn dot_product(a: &[f32], b: &[f32]) -> f32 {
    let mut sum = 0.0f32;
    let len = a.len();
    
    for i in 0..len {
        sum += a[i] * b[i];
    }
    
    sum
}

#[inline]
pub fn manhattan_distance(a: &[f32], b: &[f32]) -> f32 {
    let mut sum = 0.0f32;
    let len = a.len();
    
    for i in 0..len {
        sum += (a[i] - b[i]).abs();
    }
    
    sum
}

#[inline]
pub fn magnitude(v: &[f32]) -> f32 {
    dot_product(v, v).sqrt()
}

#[inline]
pub fn normalize(v: &mut [f32]) {
    let mag = magnitude(v);
    if mag > 1e-8 {
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
    fn test_cosine_similarity() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![0.0, 1.0, 0.0];
        assert!((cosine_similarity(&a, &b) - 0.0).abs() < 1e-6);
        
        let c = vec![1.0, 0.0, 0.0];
        assert!((cosine_similarity(&a, &c) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_euclidean_distance() {
        let a = vec![0.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert!((euclidean_distance(&a, &b) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_dot_product() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![4.0, 5.0, 6.0];
        assert!((dot_product(&a, &b) - 32.0).abs() < 1e-6);
    }
}
