//! Lens correction primitives for Starroom.
//! Lensfun is the intended profile/database provider. The renderer consumes the normalized
//! correction parameters defined here so CPU and GPU paths do not depend on Lensfun internals.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LensIdentity {
    pub maker: String,
    pub model: String,
    pub focal_length_mm: f32,
    pub aperture: f32,
    pub focus_distance_m: Option<f32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct DistortionCoefficients {
    /// Brown-Conrady radial terms on normalized radius.
    pub k1: f32,
    pub k2: f32,
    pub k3: f32,
}

impl Default for DistortionCoefficients {
    fn default() -> Self {
        Self { k1: 0.0, k2: 0.0, k3: 0.0 }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ChromaticAberrationCoefficients {
    /// Relative radial scale for red and blue against green.
    pub red_scale: f32,
    pub blue_scale: f32,
}

impl Default for ChromaticAberrationCoefficients {
    fn default() -> Self {
        Self { red_scale: 1.0, blue_scale: 1.0 }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct VignetteCoefficients {
    /// Multiplicative falloff: gain = 1 / (1 + v1*r² + v2*r⁴ + v3*r⁶).
    pub v1: f32,
    pub v2: f32,
    pub v3: f32,
}

impl Default for VignetteCoefficients {
    fn default() -> Self {
        Self { v1: 0.0, v2: 0.0, v3: 0.0 }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub struct LensCorrection {
    pub distortion: DistortionCoefficients,
    pub chromatic_aberration: ChromaticAberrationCoefficients,
    pub vignette: VignetteCoefficients,
}

pub trait LensProfileProvider {
    type Error;

    fn resolve(&self, lens: &LensIdentity) -> Result<Option<LensCorrection>, Self::Error>;
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NormalizedPoint {
    /// Centered normalized coordinate where image center is 0,0 and a corner is near ±1,±1.
    pub x: f32,
    pub y: f32,
}

pub fn distort(point: NormalizedPoint, coefficients: DistortionCoefficients) -> NormalizedPoint {
    let radius2 = point.x * point.x + point.y * point.y;
    let radius4 = radius2 * radius2;
    let radius6 = radius4 * radius2;
    let scale = 1.0 + coefficients.k1 * radius2 + coefficients.k2 * radius4 + coefficients.k3 * radius6;
    NormalizedPoint { x: point.x * scale, y: point.y * scale }
}

/// Numerically inverts a radial distortion profile. This is deterministic and suitable for the
/// CPU reference; the GPU path can use the same fixed iteration count.
pub fn undistort(point: NormalizedPoint, coefficients: DistortionCoefficients) -> NormalizedPoint {
    let mut estimate = point;
    for _ in 0..8 {
        let projected = distort(estimate, coefficients);
        estimate.x += point.x - projected.x;
        estimate.y += point.y - projected.y;
    }
    estimate
}

pub fn channel_coordinate(
    green_coordinate: NormalizedPoint,
    coefficients: ChromaticAberrationCoefficients,
    channel: usize,
) -> NormalizedPoint {
    let scale = match channel {
        0 => coefficients.red_scale,
        2 => coefficients.blue_scale,
        _ => 1.0,
    };
    NormalizedPoint { x: green_coordinate.x * scale, y: green_coordinate.y * scale }
}

pub fn vignette_gain(point: NormalizedPoint, coefficients: VignetteCoefficients) -> f32 {
    let radius2 = point.x * point.x + point.y * point.y;
    let radius4 = radius2 * radius2;
    let radius6 = radius4 * radius2;
    let denominator = 1.0 + coefficients.v1 * radius2 + coefficients.v2 * radius4 + coefficients.v3 * radius6;
    if denominator.abs() < 1.0e-6 || !denominator.is_finite() {
        1.0
    } else {
        (1.0 / denominator).clamp(0.1, 8.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1.0e-4
    }

    #[test]
    fn neutral_profile_is_identity() {
        let point = NormalizedPoint { x: 0.7, y: -0.3 };
        let projected = distort(point, DistortionCoefficients::default());
        assert_eq!(point, projected);
        assert_eq!(vignette_gain(point, VignetteCoefficients::default()), 1.0);
    }

    #[test]
    fn distortion_round_trip_is_close() {
        let point = NormalizedPoint { x: 0.55, y: 0.32 };
        let coefficients = DistortionCoefficients { k1: -0.12, k2: 0.03, k3: 0.0 };
        let distorted = distort(point, coefficients);
        let restored = undistort(distorted, coefficients);
        assert!(close(point.x, restored.x));
        assert!(close(point.y, restored.y));
    }

    #[test]
    fn tca_scales_only_requested_channel() {
        let point = NormalizedPoint { x: 0.8, y: 0.1 };
        let coefficients = ChromaticAberrationCoefficients { red_scale: 0.998, blue_scale: 1.003 };
        let red = channel_coordinate(point, coefficients, 0);
        let green = channel_coordinate(point, coefficients, 1);
        let blue = channel_coordinate(point, coefficients, 2);
        assert!(red.x < green.x);
        assert!(blue.x > green.x);
    }

    #[test]
    fn positive_vignette_coefficients_brighten_corrected_edges() {
        let center = vignette_gain(NormalizedPoint { x: 0.0, y: 0.0 }, VignetteCoefficients { v1: -0.25, v2: 0.0, v3: 0.0 });
        let edge = vignette_gain(NormalizedPoint { x: 0.9, y: 0.0 }, VignetteCoefficients { v1: -0.25, v2: 0.0, v3: 0.0 });
        assert!(edge > center);
    }
}
