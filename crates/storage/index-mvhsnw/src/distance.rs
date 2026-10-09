/// Trait defining a distance metric for vector comparisons.
pub trait DistanceMetric {
    /// Compute the distance between two vectors. Lower is more similar.
    fn calculate(a: &[f32], b: &[f32]) -> f32;
}

/// L2 (Euclidean) Distance.
/// Represents the straight-line distance between two points in vector space.
pub struct L2Distance;

impl DistanceMetric for L2Distance {
    #[inline]
    fn calculate(a: &[f32], b: &[f32]) -> f32 {
        debug_assert_eq!(a.len(), b.len(), "Vectors must have the same length");
        
        let mut sum = 0.0;
        // In a real production SIMD build, this would use AVX-512 intrinsics.
        // Rust's auto-vectorizer is usually good enough for this loop.
        for i in 0..a.len() {
            let diff = a[i] - b[i];
            sum += diff * diff;
        }
        sum.sqrt()
    }
}

/// Cosine Distance.
/// Measures the angle between two vectors. 
/// Distance = 1.0 - Cosine Similarity.
pub struct CosineDistance;

impl DistanceMetric for CosineDistance {
    #[inline]
    fn calculate(a: &[f32], b: &[f32]) -> f32 {
        debug_assert_eq!(a.len(), b.len(), "Vectors must have the same length");

        let mut dot_product = 0.0;
        let mut norm_a = 0.0;
        let mut norm_b = 0.0;

        for i in 0..a.len() {
            dot_product += a[i] * b[i];
            norm_a += a[i] * a[i];
            norm_b += b[i] * b[i];
        }

        if norm_a == 0.0 || norm_b == 0.0 {
            return 1.0; // Max distance if one vector is zero
        }

        let similarity = dot_product / (norm_a.sqrt() * norm_b.sqrt());
        
        // Clamp to [-1.0, 1.0] to avoid precision errors causing NaN in acos
        let similarity = similarity.clamp(-1.0, 1.0);
        
        // Convert similarity to distance (0.0 is identical, 2.0 is opposite)
        1.0 - similarity
    }
}


