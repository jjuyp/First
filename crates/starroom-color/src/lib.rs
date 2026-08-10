//! Starroom v0.2 CPU reference color engine.
//! Published color science and independent Starroom code are used here. The module is the
//! authoritative CPU reference for future wgpu shaders.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LinearRgb {
    pub r: f32,
    pub g: f32,
    pub b: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Oklab {
    pub l: f32,
    pub a: f32,
    pub b: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Oklch {
    pub l: f32,
    pub c: f32,
    pub h_deg: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ToneParameters {
    pub exposure_ev: f32,
    pub contrast: f32,
    pub highlights: f32,
    pub shadows: f32,
    pub whites: f32,
    pub blacks: f32,
}

impl Default for ToneParameters {
    fn default() -> Self {
        Self {
            exposure_ev: 0.0,
            contrast: 0.0,
            highlights: 0.0,
            shadows: 0.0,
            whites: 0.0,
            blacks: 0.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CurvePoint {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ColorBand {
    Red,
    Orange,
    Yellow,
    Green,
    Aqua,
    Blue,
    Purple,
    Magenta,
}

impl ColorBand {
    pub const ALL: [Self; 8] = [
        Self::Red,
        Self::Orange,
        Self::Yellow,
        Self::Green,
        Self::Aqua,
        Self::Blue,
        Self::Purple,
        Self::Magenta,
    ];

    fn center_degrees(self) -> f32 {
        match self {
            Self::Red => 25.0,
            Self::Orange => 55.0,
            Self::Yellow => 95.0,
            Self::Green => 145.0,
            Self::Aqua => 195.0,
            Self::Blue => 250.0,
            Self::Purple => 300.0,
            Self::Magenta => 335.0,
        }
    }

    fn index(self) -> usize {
        match self {
            Self::Red => 0,
            Self::Orange => 1,
            Self::Yellow => 2,
            Self::Green => 3,
            Self::Aqua => 4,
            Self::Blue => 5,
            Self::Purple => 6,
            Self::Magenta => 7,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub struct BandAdjustment {
    /// Relative hue rotation in degrees. UI target range: -30..30.
    pub hue_degrees: f32,
    /// Relative chroma adjustment. UI target range: -1..1.
    pub chroma: f32,
    /// Relative perceptual lightness adjustment. UI target range: -1..1.
    pub lightness: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub struct ColorMixer {
    pub bands: [BandAdjustment; 8],
}

impl ColorMixer {
    pub fn with_band(mut self, band: ColorBand, adjustment: BandAdjustment) -> Self {
        self.bands[band.index()] = adjustment;
        self
    }
}

fn clamp_unit_control(value: f32) -> f32 {
    value.clamp(-1.0, 1.0)
}

fn smoothstep(edge0: f32, edge1: f32, value: f32) -> f32 {
    if (edge1 - edge0).abs() < f32::EPSILON {
        return if value < edge0 { 0.0 } else { 1.0 };
    }
    let t = ((value - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Rec.2020/D65 relative luminance for Starroom's linear working RGB baseline.
pub fn luminance(rgb: LinearRgb) -> f32 {
    0.2627 * rgb.r + 0.6780 * rgb.g + 0.0593 * rgb.b
}

fn zone_weights(y: f32) -> (f32, f32, f32, f32) {
    let safe_y = y.max(0.0);
    // Preserve true black and fade shadow influence before the midtones. This avoids the
    // v0.1 failure where Shadows behaved like a broad white veil.
    let shadow =
        smoothstep(0.004, 0.012, safe_y) * (1.0 - smoothstep(0.06, 0.18, safe_y));
    let black = 1.0 - smoothstep(0.0, 0.11, safe_y);
    let highlight =
        smoothstep(0.34, 0.62, safe_y) * (1.0 - smoothstep(1.10, 1.55, safe_y));
    let white = smoothstep(0.72, 1.02, safe_y);
    (shadow, black, highlight, white)
}

fn tone_luminance(y: f32, parameters: ToneParameters) -> f32 {
    if !y.is_finite() {
        return 0.0;
    }

    let mut output = y.max(0.0) * 2.0_f32.powf(parameters.exposure_ev.clamp(-5.0, 5.0));
    let (shadow_weight, black_weight, highlight_weight, white_weight) = zone_weights(output);

    let shadows = clamp_unit_control(parameters.shadows);
    if shadows >= 0.0 {
        output += shadows
            * shadow_weight
            * (0.24 + 0.18 * output.sqrt())
            * (1.0 - output.min(1.0));
    } else {
        output *= 1.0 + shadows * shadow_weight * 0.72;
    }

    let highlights = clamp_unit_control(parameters.highlights);
    if highlights < 0.0 {
        let compression = 1.0 + (-highlights) * highlight_weight * 1.35;
        output = output / compression + output.min(0.22) * (1.0 - 1.0 / compression);
    } else {
        output += highlights * highlight_weight * (1.0 - output.min(1.0)) * 0.22;
    }

    let blacks = clamp_unit_control(parameters.blacks);
    if blacks >= 0.0 {
        output += blacks * black_weight * 0.055;
    } else {
        output *= 1.0 + blacks * black_weight * 0.82;
    }

    let whites = clamp_unit_control(parameters.whites);
    if whites >= 0.0 {
        output += whites * white_weight * (0.10 + 0.10 * output.min(1.0));
    } else {
        output *= 1.0 + whites * white_weight * 0.48;
    }

    let contrast = clamp_unit_control(parameters.contrast);
    if contrast.abs() > f32::EPSILON {
        let pivot = 0.18_f32;
        let safe = output.max(1.0e-6);
        let stops = (safe / pivot).log2();
        output = pivot * 2.0_f32.powf(stops * (1.0 + contrast * 0.62));
    }

    if output.is_finite() {
        output.max(0.0)
    } else {
        0.0
    }
}

/// Applies tone by remapping luminance and scaling RGB together. This keeps hue/chroma much
/// more stable than moving each RGB channel independently toward white or black.
pub fn apply_tone(rgb: LinearRgb, parameters: ToneParameters) -> LinearRgb {
    let source_luminance = luminance(rgb).max(0.0);
    let target_luminance = tone_luminance(source_luminance, parameters);
    if source_luminance <= 1.0e-7 {
        return LinearRgb {
            r: target_luminance,
            g: target_luminance,
            b: target_luminance,
        };
    }

    let scale = target_luminance / source_luminance;
    LinearRgb {
        r: rgb.r * scale,
        g: rgb.g * scale,
        b: rgb.b * scale,
    }
}

fn rec2020_to_xyz(rgb: LinearRgb) -> (f32, f32, f32) {
    (
        0.636_958_06 * rgb.r + 0.144_616_9 * rgb.g + 0.168_880_98 * rgb.b,
        0.262_700_2 * rgb.r + 0.677_998_07 * rgb.g + 0.059_301_72 * rgb.b,
        0.028_072_693 * rgb.g + 1.060_985_1 * rgb.b,
    )
}

fn xyz_to_rec2020(x: f32, y: f32, z: f32) -> LinearRgb {
    LinearRgb {
        r: 1.716_651_2 * x - 0.355_670_78 * y - 0.253_366_3 * z,
        g: -0.666_684_3 * x + 1.616_481_2 * y + 0.015_768_546 * z,
        b: 0.017_639_857 * x - 0.042_770_613 * y + 0.942_103_1 * z,
    }
}

pub fn rec2020_to_oklab(rgb: LinearRgb) -> Oklab {
    let (x, y, z) = rec2020_to_xyz(rgb);
    let l = (0.818_933 * x + 0.361_866_74 * y - 0.128_859_71 * z).cbrt();
    let m = (0.032_984_544 * x + 0.929_311_9 * y + 0.036_145_64 * z).cbrt();
    let s = (0.048_200_3 * x + 0.264_366_27 * y + 0.633_851_7 * z).cbrt();
    Oklab {
        l: 0.210_454_26 * l + 0.793_617_8 * m - 0.004_072_047 * s,
        a: 1.977_998_5 * l - 2.428_592_2 * m + 0.450_593_7 * s,
        b: 0.025_904_037 * l + 0.782_771_77 * m - 0.808_675_77 * s,
    }
}

pub fn oklab_to_rec2020(lab: Oklab) -> LinearRgb {
    let l_prime = lab.l + 0.396_337_78 * lab.a + 0.215_803_76 * lab.b;
    let m_prime = lab.l - 0.105_561_346 * lab.a - 0.063_854_17 * lab.b;
    let s_prime = lab.l - 0.089_484_18 * lab.a - 1.291_485_5 * lab.b;
    let l = l_prime * l_prime * l_prime;
    let m = m_prime * m_prime * m_prime;
    let s = s_prime * s_prime * s_prime;
    let x = 1.227_014 * l - 0.557_8 * m - 0.281_256_14 * s;
    let y = -0.040_580_18 * l + 1.112_256_9 * m - 0.071_676_68 * s;
    let z = -0.076_381_29 * l - 0.421_481_97 * m + 1.586_163_2 * s;
    xyz_to_rec2020(x, y, z)
}

pub fn oklab_to_oklch(lab: Oklab) -> Oklch {
    let chroma = (lab.a * lab.a + lab.b * lab.b).sqrt();
    let mut hue = lab.b.atan2(lab.a).to_degrees();
    if hue < 0.0 {
        hue += 360.0;
    }
    Oklch {
        l: lab.l,
        c: chroma,
        h_deg: hue,
    }
}

pub fn oklch_to_oklab(lch: Oklch) -> Oklab {
    let angle = lch.h_deg.to_radians();
    Oklab {
        l: lch.l,
        a: lch.c * angle.cos(),
        b: lch.c * angle.sin(),
    }
}

pub fn rotate_hue(rgb: LinearRgb, degrees: f32) -> LinearRgb {
    let mut lch = oklab_to_oklch(rec2020_to_oklab(rgb));
    lch.h_deg = (lch.h_deg + degrees).rem_euclid(360.0);
    oklab_to_rec2020(oklch_to_oklab(lch))
}

fn circular_distance_degrees(a: f32, b: f32) -> f32 {
    let difference = (a - b).rem_euclid(360.0).abs();
    difference.min(360.0 - difference)
}

fn color_band_weight(hue: f32, band: ColorBand) -> f32 {
    let distance = circular_distance_degrees(hue, band.center_degrees());
    1.0 - smoothstep(20.0, 55.0, distance)
}

/// Eight-band selective color editing in OKLCh. Hue adjustment keeps L and C fixed before
/// gamut mapping; chroma and lightness are explicit independent controls.
pub fn apply_color_mixer(rgb: LinearRgb, mixer: ColorMixer) -> LinearRgb {
    let mut lch = oklab_to_oklch(rec2020_to_oklab(rgb));
    if lch.c < 1.0e-7 {
        return rgb;
    }

    let original_hue = lch.h_deg;
    let mut weight_total = 0.0;
    let mut hue_delta = 0.0;
    let mut chroma_delta = 0.0;
    let mut lightness_delta = 0.0;

    for band in ColorBand::ALL {
        let weight = color_band_weight(original_hue, band);
        if weight <= 0.0 {
            continue;
        }
        let adjustment = mixer.bands[band.index()];
        weight_total += weight;
        hue_delta += adjustment.hue_degrees.clamp(-30.0, 30.0) * weight;
        chroma_delta += clamp_unit_control(adjustment.chroma) * weight;
        lightness_delta += clamp_unit_control(adjustment.lightness) * weight;
    }

    if weight_total > 1.0e-7 {
        let inverse = 1.0 / weight_total;
        lch.h_deg = (lch.h_deg + hue_delta * inverse).rem_euclid(360.0);
        lch.c *= 1.0 + chroma_delta * inverse * 0.75;
        lch.c = lch.c.max(0.0);
        lch.l += lightness_delta * inverse * 0.16;
    }

    oklab_to_rec2020(oklch_to_oklab(lch))
}

/// Smoothly reduces OKLCh chroma until RGB fits a normalized display/output gamut. Starroom's
/// internal creative pipeline remains unbounded; call this only at a bounded output boundary.
pub fn compress_to_unit_gamut(rgb: LinearRgb) -> LinearRgb {
    if [rgb.r, rgb.g, rgb.b]
        .into_iter()
        .all(|channel| (0.0..=1.0).contains(&channel))
    {
        return rgb;
    }

    let mut lch = oklab_to_oklch(rec2020_to_oklab(rgb));
    let original_chroma = lch.c;
    let mut low = 0.0;
    let mut high = original_chroma;
    let mut best = LinearRgb {
        r: lch.l,
        g: lch.l,
        b: lch.l,
    };

    for _ in 0..14 {
        lch.c = (low + high) * 0.5;
        let candidate = oklab_to_rec2020(oklch_to_oklab(lch));
        let in_gamut = [candidate.r, candidate.g, candidate.b]
            .into_iter()
            .all(|channel| (0.0..=1.0).contains(&channel));
        if in_gamut {
            best = candidate;
            low = lch.c;
        } else {
            high = lch.c;
        }
    }

    best
}

/// Monotone cubic Hermite curve mapping. For monotone control points this avoids spline
/// overshoot and the harsh piecewise-linear bends from the v0.1 browser prototype.
pub fn map_monotone_curve(value: f32, points: &[CurvePoint]) -> f32 {
    let mut points: Vec<CurvePoint> = points
        .iter()
        .copied()
        .filter(|point| point.x.is_finite() && point.y.is_finite())
        .collect();
    points.sort_by(|left, right| left.x.total_cmp(&right.x));
    points.dedup_by(|left, right| (left.x - right.x).abs() < 1.0e-6);

    if points.len() < 2 {
        return value;
    }
    if value <= points[0].x {
        return points[0].y;
    }
    if value >= points[points.len() - 1].x {
        return points[points.len() - 1].y;
    }

    let segment_count = points.len() - 1;
    let mut slopes = vec![0.0_f32; segment_count];
    for index in 0..segment_count {
        let dx = (points[index + 1].x - points[index].x).max(1.0e-6);
        slopes[index] = (points[index + 1].y - points[index].y) / dx;
    }

    let mut tangents = vec![0.0_f32; points.len()];
    tangents[0] = slopes[0];
    tangents[points.len() - 1] = slopes[segment_count - 1];
    for index in 1..points.len() - 1 {
        let left = slopes[index - 1];
        let right = slopes[index];
        tangents[index] = if left * right <= 0.0 {
            0.0
        } else {
            2.0 * left * right / (left + right)
        };
    }

    for index in 0..segment_count {
        let left = points[index];
        let right = points[index + 1];
        if value > right.x {
            continue;
        }

        let width = (right.x - left.x).max(1.0e-6);
        let t = ((value - left.x) / width).clamp(0.0, 1.0);
        let t2 = t * t;
        let t3 = t2 * t;
        let h00 = 2.0 * t3 - 3.0 * t2 + 1.0;
        let h10 = t3 - 2.0 * t2 + t;
        let h01 = -2.0 * t3 + 3.0 * t2;
        let h11 = t3 - t2;
        return h00 * left.y
            + h10 * width * tangents[index]
            + h01 * right.y
            + h11 * width * tangents[index + 1];
    }

    value
}

#[cfg(test)]
mod tests {
    use super::*;

    fn delta(a: f32, b: f32) -> f32 {
        (a - b).abs()
    }

    #[test]
    fn neutral_tone_is_identity() {
        let rgb = LinearRgb {
            r: 0.21,
            g: 0.13,
            b: 0.07,
        };
        let output = apply_tone(rgb, ToneParameters::default());
        assert!(delta(output.r, rgb.r) < 1e-6);
        assert!(delta(output.g, rgb.g) < 1e-6);
        assert!(delta(output.b, rgb.b) < 1e-6);
    }

    #[test]
    fn positive_shadows_lift_dark_region_without_washing_midtones() {
        let parameters = ToneParameters {
            shadows: 0.5,
            ..Default::default()
        };
        let dark = LinearRgb {
            r: 0.018,
            g: 0.014,
            b: 0.010,
        };
        let mid = LinearRgb {
            r: 0.23,
            g: 0.19,
            b: 0.16,
        };
        let dark_gain = luminance(apply_tone(dark, parameters)) - luminance(dark);
        let mid_gain = luminance(apply_tone(mid, parameters)) - luminance(mid);
        assert!(dark_gain > 0.0);
        assert!(dark_gain > mid_gain * 2.0);
    }

    #[test]
    fn shadow_lift_keeps_black_anchor() {
        let parameters = ToneParameters {
            shadows: 1.0,
            ..Default::default()
        };
        let black = LinearRgb {
            r: 0.0,
            g: 0.0,
            b: 0.0,
        };
        let output = apply_tone(black, parameters);
        assert!(output.r.abs() < 1e-6 && output.g.abs() < 1e-6 && output.b.abs() < 1e-6);
    }

    #[test]
    fn hue_rotation_preserves_oklch_lightness_and_chroma_before_gamut_mapping() {
        let rgb = LinearRgb {
            r: 0.30,
            g: 0.16,
            b: 0.08,
        };
        let before = oklab_to_oklch(rec2020_to_oklab(rgb));
        let after = oklab_to_oklch(rec2020_to_oklab(rotate_hue(rgb, 42.0)));
        assert!(delta(before.l, after.l) < 2e-4);
        assert!(delta(before.c, after.c) < 2e-4);
    }

    #[test]
    fn monotone_curve_is_smooth_and_bounded_for_monotone_points() {
        let points = [
            CurvePoint { x: 0.0, y: 0.0 },
            CurvePoint { x: 0.25, y: 0.18 },
            CurvePoint { x: 0.50, y: 0.58 },
            CurvePoint { x: 1.0, y: 1.0 },
        ];
        let mut previous = map_monotone_curve(0.0, &points);
        for sample in 1..=100 {
            let value = sample as f32 / 100.0;
            let output = map_monotone_curve(value, &points);
            assert!(output >= previous - 1.0e-5);
            assert!((0.0..=1.0).contains(&output));
            previous = output;
        }
    }

    #[test]
    fn color_mixer_changes_selected_hue_without_nan() {
        let mixer = ColorMixer::default().with_band(
            ColorBand::Red,
            BandAdjustment {
                hue_degrees: 15.0,
                chroma: 0.20,
                lightness: 0.05,
            },
        );
        let source = LinearRgb {
            r: 0.35,
            g: 0.08,
            b: 0.05,
        };
        let output = apply_color_mixer(source, mixer);
        assert!(output.r.is_finite() && output.g.is_finite() && output.b.is_finite());
        assert!(
            delta(source.r, output.r) + delta(source.g, output.g) + delta(source.b, output.b)
                > 1.0e-4
        );
    }
}
