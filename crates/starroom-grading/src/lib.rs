//! Oklab color grading for Starroom.
//! Wheels are represented as hue/saturation-like controls but applied as opponent-axis offsets
//! in a perceptual space, with smooth luminance weighting across tonal ranges.

use serde::{Deserialize, Serialize};
use starroom_color::{LinearRgb, Oklab, oklab_to_rec2020, rec2020_to_oklab};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub struct ColorWheel {
    /// 0..360 degrees.
    pub hue_degrees: f32,
    /// 0..1 creative amount.
    pub saturation: f32,
    /// Relative lightness offset, -1..1.
    pub luminance: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct GradingParameters {
    pub shadows: ColorWheel,
    pub midtones: ColorWheel,
    pub highlights: ColorWheel,
    pub global: ColorWheel,
    /// -1 moves tonal crossover toward shadows, +1 toward highlights.
    pub balance: f32,
    /// 0 keeps zones tighter, 1 maximizes overlap.
    pub blending: f32,
    /// Master effect amount, 0..1.
    pub amount: f32,
}

impl Default for GradingParameters {
    fn default() -> Self {
        Self {
            shadows: ColorWheel::default(),
            midtones: ColorWheel::default(),
            highlights: ColorWheel::default(),
            global: ColorWheel::default(),
            balance: 0.0,
            blending: 0.5,
            amount: 1.0,
        }
    }
}

fn smoothstep(edge0: f32, edge1: f32, value: f32) -> f32 {
    let width = edge1 - edge0;
    if width.abs() < f32::EPSILON {
        return if value < edge0 { 0.0 } else { 1.0 };
    }
    let t = ((value - edge0) / width).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn wheel_vector(wheel: ColorWheel) -> (f32, f32, f32) {
    let angle = wheel.hue_degrees.rem_euclid(360.0).to_radians();
    let chroma = wheel.saturation.clamp(0.0, 1.0) * 0.12;
    (
        angle.cos() * chroma,
        angle.sin() * chroma,
        wheel.luminance.clamp(-1.0, 1.0) * 0.12,
    )
}

fn tonal_weights(lightness: f32, balance: f32, blending: f32) -> (f32, f32, f32) {
    let balance_shift = balance.clamp(-1.0, 1.0) * 0.12;
    let overlap = 0.08 + blending.clamp(0.0, 1.0) * 0.18;
    let shadow_end = 0.42 + balance_shift;
    let highlight_start = 0.58 + balance_shift;

    let shadow = 1.0 - smoothstep(shadow_end - overlap, shadow_end + overlap, lightness);
    let highlight = smoothstep(
        highlight_start - overlap,
        highlight_start + overlap,
        lightness,
    );
    let midtone = (1.0 - shadow.max(highlight)).clamp(0.0, 1.0);
    let sum = (shadow + midtone + highlight).max(f32::EPSILON);
    (shadow / sum, midtone / sum, highlight / sum)
}

fn apply_wheel(lab: &mut Oklab, wheel: ColorWheel, weight: f32, amount: f32) {
    if weight <= f32::EPSILON || amount <= f32::EPSILON {
        return;
    }
    let (a, b, l) = wheel_vector(wheel);
    lab.a += a * weight * amount;
    lab.b += b * weight * amount;
    lab.l += l * weight * amount;
}

pub fn apply_grading(rgb: LinearRgb, parameters: GradingParameters) -> LinearRgb {
    let amount = parameters.amount.clamp(0.0, 1.0);
    if amount <= f32::EPSILON {
        return rgb;
    }
    let mut lab = rec2020_to_oklab(rgb);
    let source_lightness = lab.l;
    let (shadow_weight, midtone_weight, highlight_weight) = tonal_weights(
        source_lightness,
        parameters.balance,
        parameters.blending,
    );

    apply_wheel(&mut lab, parameters.shadows, shadow_weight, amount);
    apply_wheel(&mut lab, parameters.midtones, midtone_weight, amount);
    apply_wheel(&mut lab, parameters.highlights, highlight_weight, amount);
    apply_wheel(&mut lab, parameters.global, 1.0, amount);
    lab.l = lab.l.max(0.0);
    oklab_to_rec2020(lab)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn distance(a: LinearRgb, b: LinearRgb) -> f32 {
        (a.r - b.r).abs() + (a.g - b.g).abs() + (a.b - b.b).abs()
    }

    #[test]
    fn zero_amount_is_identity() {
        let source = LinearRgb { r: 0.2, g: 0.15, b: 0.1 };
        let output = apply_grading(
            source,
            GradingParameters { amount: 0.0, ..Default::default() },
        );
        assert_eq!(source, output);
    }

    #[test]
    fn shadow_wheel_affects_dark_pixel_more_than_bright_pixel() {
        let parameters = GradingParameters {
            shadows: ColorWheel { hue_degrees: 220.0, saturation: 0.7, luminance: 0.0 },
            ..Default::default()
        };
        let dark = LinearRgb { r: 0.03, g: 0.025, b: 0.02 };
        let bright = LinearRgb { r: 0.8, g: 0.75, b: 0.7 };
        let dark_delta = distance(dark, apply_grading(dark, parameters));
        let bright_delta = distance(bright, apply_grading(bright, parameters));
        assert!(dark_delta > bright_delta);
    }

    #[test]
    fn global_wheel_affects_all_tones() {
        let parameters = GradingParameters {
            global: ColorWheel { hue_degrees: 35.0, saturation: 0.4, luminance: 0.0 },
            ..Default::default()
        };
        let source = LinearRgb { r: 0.45, g: 0.35, b: 0.25 };
        assert!(distance(source, apply_grading(source, parameters)) > 1.0e-4);
    }

    #[test]
    fn grading_stays_finite_at_extreme_controls() {
        let parameters = GradingParameters {
            shadows: ColorWheel { hue_degrees: 3600.0, saturation: 2.0, luminance: -3.0 },
            midtones: ColorWheel { hue_degrees: -720.0, saturation: 1.0, luminance: 1.0 },
            highlights: ColorWheel { hue_degrees: 120.0, saturation: 1.0, luminance: 1.0 },
            global: ColorWheel { hue_degrees: 300.0, saturation: 1.0, luminance: 1.0 },
            balance: 5.0,
            blending: 5.0,
            amount: 5.0,
        };
        let output = apply_grading(LinearRgb { r: 0.3, g: 0.2, b: 0.1 }, parameters);
        assert!(output.r.is_finite() && output.g.is_finite() && output.b.is_finite());
    }
}
