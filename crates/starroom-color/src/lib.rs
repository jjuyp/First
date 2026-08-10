//! Starroom v0.2 CPU reference color engine.
//! The implementation is clean-room and based on published color science, not copied GPL code.

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
        Self { exposure_ev: 0.0, contrast: 0.0, highlights: 0.0, shadows: 0.0, whites: 0.0, blacks: 0.0 }
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
    // Preserve a true black anchor: shadow lift fades to zero below ~1% linear.
    let shadow = smoothstep(0.008, 0.035, safe_y) * (1.0 - smoothstep(0.32, 0.58, safe_y));
    let black = 1.0 - smoothstep(0.0, 0.11, safe_y);
    let highlight = smoothstep(0.34, 0.62, safe_y) * (1.0 - smoothstep(1.10, 1.55, safe_y));
    let white = smoothstep(0.72, 1.02, safe_y);
    (shadow, black, highlight, white)
}

fn tone_luminance(y: f32, p: ToneParameters) -> f32 {
    if !y.is_finite() {
        return 0.0;
    }
    let mut out = (y.max(0.0)) * 2.0_f32.powf(p.exposure_ev.clamp(-5.0, 5.0));
    let (shadow_w, black_w, highlight_w, white_w) = zone_weights(out);

    let shadows = clamp_unit_control(p.shadows);
    if shadows >= 0.0 {
        out += shadows * shadow_w * (0.24 + 0.18 * out.sqrt()) * (1.0 - out.min(1.0));
    } else {
        out *= 1.0 + shadows * shadow_w * 0.72;
    }

    let highlights = clamp_unit_control(p.highlights);
    if highlights < 0.0 {
        // Compress high luminance without subtracting uniform gray from the image.
        let compression = 1.0 + (-highlights) * highlight_w * 1.35;
        out = out / compression + out.min(0.22) * (1.0 - 1.0 / compression);
    } else {
        out += highlights * highlight_w * (1.0 - out.min(1.0)) * 0.22;
    }

    let blacks = clamp_unit_control(p.blacks);
    if blacks >= 0.0 {
        out += blacks * black_w * 0.055;
    } else {
        out *= 1.0 + blacks * black_w * 0.82;
    }

    let whites = clamp_unit_control(p.whites);
    if whites >= 0.0 {
        out += whites * white_w * (0.10 + 0.10 * out.min(1.0));
    } else {
        out *= 1.0 + whites * white_w * 0.48;
    }

    let contrast = clamp_unit_control(p.contrast);
    if contrast.abs() > f32::EPSILON {
        let pivot = 0.18_f32;
        let safe = out.max(1.0e-6);
        let stops = (safe / pivot).log2();
        out = pivot * 2.0_f32.powf(stops * (1.0 + contrast * 0.62));
    }

    if out.is_finite() { out.max(0.0) } else { 0.0 }
}

/// Applies tone by remapping luminance and scaling RGB together. This preserves hue/chroma
/// far better than moving each RGB channel independently toward white/black.
pub fn apply_tone(rgb: LinearRgb, params: ToneParameters) -> LinearRgb {
    let y = luminance(rgb).max(0.0);
    let target = tone_luminance(y, params);
    if y <= 1.0e-7 {
        return LinearRgb { r: target, g: target, b: target };
    }
    let scale = target / y;
    LinearRgb { r: rgb.r * scale, g: rgb.g * scale, b: rgb.b * scale }
}

fn rec2020_to_xyz(rgb: LinearRgb) -> (f32, f32, f32) {
    (
        0.636_958_06 * rgb.r + 0.144_616_9 * rgb.g + 0.168_880_98 * rgb.b,
        0.262_700_2 * rgb.r + 0.677_998_07 * rgb.g + 0.059_301_72 * rgb.b,
        0.000_000_0 * rgb.r + 0.028_072_693 * rgb.g + 1.060_985_1 * rgb.b,
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
    let l_ = lab.l + 0.396_337_78 * lab.a + 0.215_803_76 * lab.b;
    let m_ = lab.l - 0.105_561_346 * lab.a - 0.063_854_17 * lab.b;
    let s_ = lab.l - 0.089_484_18 * lab.a - 1.291_485_5 * lab.b;
    let l = l_ * l_ * l_;
    let m = m_ * m_ * m_;
    let s = s_ * s_ * s_;
    let x = 1.227_014 * l - 0.557_8 * m - 0.281_256_14 * s;
    let y = -0.040_580_18 * l + 1.112_256_9 * m - 0.071_676_68 * s;
    let z = -0.076_381_29 * l - 0.421_481_97 * m + 1.586_163_2 * s;
    xyz_to_rec2020(x, y, z)
}

pub fn oklab_to_oklch(lab: Oklab) -> Oklch {
    let c = (lab.a * lab.a + lab.b * lab.b).sqrt();
    let mut h = lab.b.atan2(lab.a).to_degrees();
    if h < 0.0 { h += 360.0; }
    Oklch { l: lab.l, c, h_deg: h }
}

pub fn oklch_to_oklab(lch: Oklch) -> Oklab {
    let angle = lch.h_deg.to_radians();
    Oklab { l: lch.l, a: lch.c * angle.cos(), b: lch.c * angle.sin() }
}

pub fn rotate_hue(rgb: LinearRgb, degrees: f32) -> LinearRgb {
    let mut lch = oklab_to_oklch(rec2020_to_oklab(rgb));
    lch.h_deg = (lch.h_deg + degrees).rem_euclid(360.0);
    oklab_to_rec2020(oklch_to_oklab(lch))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn delta(a: f32, b: f32) -> f32 { (a - b).abs() }

    #[test]
    fn neutral_tone_is_identity() {
        let rgb = LinearRgb { r: 0.21, g: 0.13, b: 0.07 };
        let out = apply_tone(rgb, ToneParameters::default());
        assert!(delta(out.r, rgb.r) < 1e-6);
        assert!(delta(out.g, rgb.g) < 1e-6);
        assert!(delta(out.b, rgb.b) < 1e-6);
    }

    #[test]
    fn positive_shadows_lift_dark_region_without_washing_midtones() {
        let p = ToneParameters { shadows: 0.5, ..Default::default() };
        let dark = LinearRgb { r: 0.08, g: 0.06, b: 0.04 };
        let mid = LinearRgb { r: 0.50, g: 0.40, b: 0.30 };
        let dark_gain = luminance(apply_tone(dark, p)) - luminance(dark);
        let mid_gain = luminance(apply_tone(mid, p)) - luminance(mid);
        assert!(dark_gain > 0.0);
        assert!(dark_gain > mid_gain * 2.0);
    }

    #[test]
    fn shadow_lift_keeps_black_anchor() {
        let p = ToneParameters { shadows: 1.0, ..Default::default() };
        let black = LinearRgb { r: 0.0, g: 0.0, b: 0.0 };
        let out = apply_tone(black, p);
        assert!(out.r.abs() < 1e-6 && out.g.abs() < 1e-6 && out.b.abs() < 1e-6);
    }

    #[test]
    fn hue_rotation_preserves_oklch_lightness_and_chroma_before_gamut_mapping() {
        let rgb = LinearRgb { r: 0.30, g: 0.16, b: 0.08 };
        let before = oklab_to_oklch(rec2020_to_oklab(rgb));
        let after = oklab_to_oklch(rec2020_to_oklab(rotate_hue(rgb, 42.0)));
        assert!(delta(before.l, after.l) < 2e-4);
        assert!(delta(before.c, after.c) < 2e-4);
    }
}
