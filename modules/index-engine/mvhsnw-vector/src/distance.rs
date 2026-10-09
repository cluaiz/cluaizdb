/// Trait defining a distance metric for vector comparisons.
pub trait DistanceMetric: Send + Sync + 'static {
    /// Compute the distance between two vectors. Lower is more similar.
    fn calculate(a: &[f32], b: &[f32]) -> f32;
}

/// L2 (Euclidean) Distance.
/// Represents straight-line distance in Euclidean space.
pub struct L2Distance;

impl DistanceMetric for L2Distance {
    #[inline]
    fn calculate(a: &[f32], b: &[f32]) -> f32 {
        debug_assert_eq!(a.len(), b.len(), "Vectors must have matching dimensions");
        let mut sum = 0.0;
        for i in 0..a.len() {
            let diff = a[i] - b[i];
            sum += diff * diff;
        }
        sum.sqrt()
    }
}

/// Cosine Distance: `1.0 - Cosine Similarity`.
pub struct CosineDistance;

impl DistanceMetric for CosineDistance {
    #[inline]
    fn calculate(a: &[f32], b: &[f32]) -> f32 {
        debug_assert_eq!(a.len(), b.len(), "Vectors must have matching dimensions");

        let mut dot_product = 0.0;
        let mut norm_a = 0.0;
        let mut norm_b = 0.0;

        for i in 0..a.len() {
            dot_product += a[i] * b[i];
            norm_a += a[i] * a[i];
            norm_b += b[i] * b[i];
        }

        if norm_a == 0.0 || norm_b == 0.0 {
            return 1.0;
        }

        let similarity = dot_product / (norm_a.sqrt() * norm_b.sqrt());
        let similarity = similarity.clamp(-1.0, 1.0);
        1.0 - similarity
    }
}
