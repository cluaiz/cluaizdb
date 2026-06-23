//! SIMD-Optimized Vector Distance Calculations
//! 
//! Exposes auto-vectorized cosine similarity and Euclidean distance functions
//! for 16-dimensional float arrays.

/// Calculates the dot product of two float vectors.
#[inline(always)]
pub fn dot_product(a: &[f32], b: &[f32]) -> f32 {
    let mut sum = 0.0f32;
    let len = a.len().min(b.len());
    for i in 0..len {
        sum += a[i] * b[i];
    }
    sum
}

/// Calculates the magnitude (L2 norm) of a float vector.
#[inline(always)]
pub fn magnitude(a: &[f32]) -> f32 {
    let mut sum = 0.0f32;
    for i in 0..a.len() {
        sum += a[i] * a[i];
    }
    sum.sqrt()
}

/// Calculates the Cosine Similarity between two vectors.
/// Returns a value between -1.0 and 1.0. If magnitude is zero, returns 0.0.
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot = dot_product(a, b);
    let mag_a = magnitude(a);
    let mag_b = magnitude(b);

    if mag_a == 0.0 || mag_b == 0.0 {
        return 0.0;
    }
    dot / (mag_a * mag_b)
}

/// Calculates the Euclidean Distance (L2 norm of the difference) between two vectors.
pub fn euclidean_distance(a: &[f32], b: &[f32]) -> f32 {
    let mut sum = 0.0f32;
    let len = a.len().min(b.len());
    for i in 0..len {
        let diff = a[i] - b[i];
        sum += diff * diff;
    }
    sum.sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosine_similarity() {
        let a = vec![1.0; 16];
        let b = vec![1.0; 16];
        let similarity = cosine_similarity(&a, &b);
        assert!((similarity - 1.0).abs() < 1e-6);

        let c = vec![-1.0; 16];
        let similarity_opposite = cosine_similarity(&a, &c);
        assert!((similarity_opposite - (-1.0)).abs() < 1e-6);
    }

    #[test]
    fn test_euclidean_distance() {
        let a = vec![1.0; 16];
        let b = vec![1.0; 16];
        assert_eq!(euclidean_distance(&a, &b), 0.0);

        let c = vec![0.0; 16];
        let distance = euclidean_distance(&a, &c);
        assert!((distance - 4.0).abs() < 1e-6); // sqrt(16 * 1) = 4
    }
}
