//! Portrait-retouch contracts and CPU references for Starroom v0.2.
//! Face/skin detection is provider-based so MediaPipe can be integrated without coupling the core.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Landmark {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

pub trait FaceLandmarkProvider {
    type Error;
    fn detect(&self, width: u32, height: u32, rgba: &[u8]) -> Result<Vec<Vec<Landmark>>, Self::Error>;
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct FrequencySplitParams {
    pub radius: usize,
    pub smooth_strength: f32,
}

impl Default for FrequencySplitParams {
    fn default() -> Self { Self { radius: 3, smooth_strength: 0.35 } }
}

/// Reference 1D box blur used only to validate frequency-separation semantics.
/// Production rendering will use separable Gaussian/edge-aware GPU kernels.
pub fn split_frequency(signal: &[f32], params: FrequencySplitParams) -> (Vec<f32>, Vec<f32>) {
    if signal.is_empty() { return (Vec::new(), Vec::new()); }
    let radius = params.radius.max(1);
    let mut low = vec![0.0; signal.len()];
    for (i, out) in low.iter_mut().enumerate() {
        let start = i.saturating_sub(radius);
        let end = (i + radius + 1).min(signal.len());
        let sum: f32 = signal[start..end].iter().copied().sum();
        *out = sum / (end - start) as f32;
    }
    let high = signal.iter().zip(&low).map(|(src, base)| src - base).collect();
    (low, high)
}

pub fn recombine_with_smoothing(low: &[f32], high: &[f32], strength: f32) -> Vec<f32> {
    let keep = 1.0 - strength.clamp(0.0, 1.0);
    low.iter().zip(high).map(|(l, h)| l + h * keep).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frequency_split_recombines_exactly_at_zero_smoothing() {
        let source = [0.1, 0.2, 0.8, 0.3, 0.2];
        let (low, high) = split_frequency(&source, FrequencySplitParams::default());
        let rebuilt = recombine_with_smoothing(&low, &high, 0.0);
        for (a, b) in rebuilt.iter().zip(source) { assert!((a - b).abs() < 1e-6); }
    }

    #[test]
    fn smoothing_reduces_high_frequency_energy() {
        let source = [0.1, 0.2, 0.8, 0.3, 0.2];
        let (low, high) = split_frequency(&source, FrequencySplitParams::default());
        let rebuilt = recombine_with_smoothing(&low, &high, 0.6);
        assert!(rebuilt[2] < source[2]);
    }
}
