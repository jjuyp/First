//! Portrait-retouch contracts and CPU references for Starroom v0.2.
//! Face/skin detection is provider-based so MediaPipe can be integrated without coupling the core.

use serde::{Deserialize, Serialize};
use starroom_detail::{LinearImage, gaussian_blur};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Landmark {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

pub trait FaceLandmarkProvider {
    type Error;

    fn detect(
        &self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<Vec<Vec<Landmark>>, Self::Error>;
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct FaceBounds {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}

impl FaceBounds {
    pub fn from_landmarks(landmarks: &[Landmark]) -> Option<Self> {
        let first = landmarks.first()?;
        let mut left = first.x;
        let mut right = first.x;
        let mut top = first.y;
        let mut bottom = first.y;
        for point in &landmarks[1..] {
            if !point.x.is_finite() || !point.y.is_finite() {
                continue;
            }
            left = left.min(point.x);
            right = right.max(point.x);
            top = top.min(point.y);
            bottom = bottom.max(point.y);
        }
        Some(Self {
            left: left.clamp(0.0, 1.0),
            top: top.clamp(0.0, 1.0),
            right: right.clamp(0.0, 1.0),
            bottom: bottom.clamp(0.0, 1.0),
        })
    }

    pub fn expanded(self, fraction: f32) -> Self {
        let expansion_x = (self.right - self.left) * fraction.max(0.0);
        let expansion_y = (self.bottom - self.top) * fraction.max(0.0);
        Self {
            left: (self.left - expansion_x).clamp(0.0, 1.0),
            top: (self.top - expansion_y).clamp(0.0, 1.0),
            right: (self.right + expansion_x).clamp(0.0, 1.0),
            bottom: (self.bottom + expansion_y).clamp(0.0, 1.0),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct FrequencySplitParams {
    /// Full-resolution Gaussian radius in pixels.
    pub radius: f32,
    /// 0..1 high-frequency attenuation during recombination.
    pub smooth_strength: f32,
}

impl Default for FrequencySplitParams {
    fn default() -> Self {
        Self { radius: 4.0, smooth_strength: 0.35 }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct FrequencyLayers {
    pub low: LinearImage,
    pub high: LinearImage,
}

pub fn split_frequency_image(
    image: &LinearImage,
    parameters: FrequencySplitParams,
) -> FrequencyLayers {
    let low = gaussian_blur(image, parameters.radius.max(0.25));
    let mut high = image.clone();
    for ((destination, source), base) in high.data.iter_mut().zip(&image.data).zip(&low.data) {
        *destination = source - base;
    }
    FrequencyLayers { low, high }
}

/// Recombines frequency layers while attenuating only the high-frequency component.
/// A production skin tool can additionally modify the low layer inside a skin mask before this
/// step; this reference deliberately does not blur the entire face indiscriminately.
pub fn recombine_frequency(
    layers: &FrequencyLayers,
    smooth_strength: f32,
    mask: Option<&[f32]>,
) -> LinearImage {
    let keep = 1.0 - smooth_strength.clamp(0.0, 1.0);
    let pixel_count = layers.low.width.saturating_mul(layers.low.height);
    let mut output = layers.low.clone();
    for pixel in 0..pixel_count {
        let mask_weight = mask
            .and_then(|values| values.get(pixel))
            .copied()
            .unwrap_or(1.0)
            .clamp(0.0, 1.0);
        let local_keep = 1.0 - (1.0 - keep) * mask_weight;
        for channel in 0..3 {
            let index = pixel * 3 + channel;
            output.data[index] = layers.low.data[index] + layers.high.data[index] * local_keep;
        }
    }
    output
}

/// Legacy 1D reference retained for project/test compatibility. New portrait rendering uses the
/// 2D RGB functions above.
pub fn split_frequency(
    signal: &[f32],
    params: FrequencySplitParams,
) -> (Vec<f32>, Vec<f32>) {
    if signal.is_empty() {
        return (Vec::new(), Vec::new());
    }
    let radius = params.radius.round().max(1.0) as usize;
    let mut low = vec![0.0; signal.len()];
    for (index, output) in low.iter_mut().enumerate() {
        let start = index.saturating_sub(radius);
        let end = (index + radius + 1).min(signal.len());
        let sum: f32 = signal[start..end].iter().copied().sum();
        *output = sum / (end - start) as f32;
    }
    let high = signal
        .iter()
        .zip(&low)
        .map(|(source, base)| source - base)
        .collect();
    (low, high)
}

pub fn recombine_with_smoothing(low: &[f32], high: &[f32], strength: f32) -> Vec<f32> {
    let keep = 1.0 - strength.clamp(0.0, 1.0);
    low.iter()
        .zip(high)
        .map(|(low_value, high_value)| low_value + high_value * keep)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frequency_split_recombines_exactly_at_zero_smoothing() {
        let source = [0.1, 0.2, 0.8, 0.3, 0.2];
        let (low, high) = split_frequency(&source, FrequencySplitParams::default());
        let rebuilt = recombine_with_smoothing(&low, &high, 0.0);
        for (actual, expected) in rebuilt.iter().zip(source) {
            assert!((actual - expected).abs() < 1e-6);
        }
    }

    #[test]
    fn smoothing_reduces_high_frequency_energy() {
        let source = [0.1, 0.2, 0.8, 0.3, 0.2];
        let (low, high) = split_frequency(&source, FrequencySplitParams::default());
        let rebuilt = recombine_with_smoothing(&low, &high, 0.6);
        assert!(rebuilt[2] < source[2]);
    }

    #[test]
    fn two_dimensional_frequency_layers_recombine_at_zero_strength() {
        let image = LinearImage::new(
            3,
            1,
            vec![0.1, 0.1, 0.1, 0.8, 0.7, 0.6, 0.1, 0.1, 0.1],
        )
        .expect("fixture");
        let layers = split_frequency_image(&image, FrequencySplitParams::default());
        let rebuilt = recombine_frequency(&layers, 0.0, None);
        for (actual, expected) in rebuilt.data.iter().zip(&image.data) {
            assert!((actual - expected).abs() < 1e-5);
        }
    }

    #[test]
    fn mask_limits_smoothing_to_selected_pixels() {
        let image = LinearImage::new(
            3,
            1,
            vec![0.1, 0.1, 0.1, 0.8, 0.7, 0.6, 0.1, 0.1, 0.1],
        )
        .expect("fixture");
        let layers = split_frequency_image(&image, FrequencySplitParams::default());
        let rebuilt = recombine_frequency(&layers, 1.0, Some(&[0.0, 1.0, 0.0]));
        assert!((rebuilt.data[0] - image.data[0]).abs() < 1e-5);
        assert!((rebuilt.data[6] - image.data[6]).abs() < 1e-5);
        assert!((rebuilt.data[3] - image.data[3]).abs() > 1e-5);
    }

    #[test]
    fn face_bounds_are_normalized_and_expandable() {
        let bounds = FaceBounds::from_landmarks(&[
            Landmark { x: 0.3, y: 0.2, z: 0.0 },
            Landmark { x: 0.7, y: 0.8, z: 0.0 },
        ])
        .expect("bounds");
        let expanded = bounds.expanded(0.1);
        assert!(expanded.left < bounds.left);
        assert!(expanded.right > bounds.right);
        assert!(expanded.top < bounds.top);
        assert!(expanded.bottom > bounds.bottom);
    }
}
