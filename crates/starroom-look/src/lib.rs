//! Portable `.srlook` schema, semantic blending, deterministic grain, and HDR-safe vignette.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use starroom_color::{BandAdjustment, ColorMixer, CurvePoint, ToneParameters, map_monotone_curve};
use starroom_detail::{DenoiseParameters, LinearImage, LocalDetailParameters, SharpenParameters};
use starroom_grading::{ColorWheel, GradingParameters};
use thiserror::Error;

pub const LOOK_SCHEMA: &str = "https://starroom.app/schemas/look/v1";

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PortableRelativeColor {
    pub temperature: f32,
    pub tint: f32,
    pub vibrance: f32,
    pub saturation: f32,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PortableCurves {
    pub master: Vec<CurvePoint>,
    pub red: Vec<CurvePoint>,
    pub green: Vec<CurvePoint>,
    pub blue: Vec<CurvePoint>,
}
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GrainSettings {
    pub amount: f32,
    pub size: f32,
    pub roughness: f32,
    pub seed: u64,
}
impl Default for GrainSettings {
    fn default() -> Self {
        Self {
            amount: 0.0,
            size: 0.5,
            roughness: 0.5,
            seed: 0,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct VignetteSettings {
    pub amount: f32,
    pub midpoint: f32,
    pub roundness: f32,
    pub feather: f32,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PortableLook {
    pub schema: String,
    pub version: u32,
    pub id: String,
    pub name: String,
    pub tone: ToneParameters,
    pub relative_color: PortableRelativeColor,
    pub curves: PortableCurves,
    pub color_mixer: ColorMixer,
    pub grading: GradingParameters,
    pub denoise: DenoiseParameters,
    pub local_detail: LocalDetailParameters,
    pub sharpen: SharpenParameters,
    pub grain: GrainSettings,
    pub vignette: VignetteSettings,
}
impl Default for PortableLook {
    fn default() -> Self {
        Self {
            schema: LOOK_SCHEMA.into(),
            version: 1,
            id: "neutral".into(),
            name: "Neutral".into(),
            tone: Default::default(),
            relative_color: Default::default(),
            curves: Default::default(),
            color_mixer: Default::default(),
            grading: Default::default(),
            denoise: Default::default(),
            local_detail: Default::default(),
            sharpen: Default::default(),
            grain: Default::default(),
            vignette: Default::default(),
        }
    }
}

#[derive(Debug, Error, PartialEq)]
pub enum LookError {
    #[error("unsupported look schema/version")]
    UnsupportedSchema,
    #[error("look JSON is invalid: {0}")]
    InvalidJson(String),
    #[error("look contains non-finite or out-of-range values")]
    InvalidValues,
    #[error("look effect image is malformed")]
    InvalidImage,
}
impl PortableLook {
    pub fn from_json(json: &str) -> Result<Self, LookError> {
        let look: Self =
            serde_json::from_str(json).map_err(|e| LookError::InvalidJson(e.to_string()))?;
        look.validate()?;
        Ok(look)
    }
    pub fn to_json(&self) -> Result<String, LookError> {
        self.validate()?;
        serde_json::to_string_pretty(self).map_err(|e| LookError::InvalidJson(e.to_string()))
    }
    pub fn validate(&self) -> Result<(), LookError> {
        if self.schema != LOOK_SCHEMA || self.version != 1 {
            return Err(LookError::UnsupportedSchema);
        }
        let values = [
            self.tone.exposure_ev,
            self.tone.contrast,
            self.relative_color.temperature,
            self.relative_color.tint,
            self.grain.amount,
            self.grain.size,
            self.grain.roughness,
            self.vignette.amount,
            self.vignette.midpoint,
            self.vignette.roundness,
            self.vignette.feather,
        ];
        if values.iter().any(|v| !v.is_finite())
            || self.grain.amount.abs() > 1.0
            || self.vignette.amount.abs() > 1.0
        {
            return Err(LookError::InvalidValues);
        }
        Ok(())
    }
}
fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}
fn hue_lerp(a: f32, b: f32, t: f32) -> f32 {
    (a + ((b - a + 180.0).rem_euclid(360.0) - 180.0) * t).rem_euclid(360.0)
}
fn wheel(a: ColorWheel, b: ColorWheel, t: f32) -> ColorWheel {
    ColorWheel {
        hue_degrees: hue_lerp(a.hue_degrees, b.hue_degrees, t),
        chroma: lerp(a.chroma, b.chroma, t),
        lightness: lerp(a.lightness, b.lightness, t),
    }
}
fn sample_curve(points: &[CurvePoint], x: f32) -> f32 {
    if points.is_empty() {
        x
    } else {
        map_monotone_curve(x, points)
    }
}
fn curve_blend(a: &[CurvePoint], b: &[CurvePoint], t: f32) -> Vec<CurvePoint> {
    (0..=32)
        .map(|i| {
            let x = i as f32 / 32.0;
            CurvePoint {
                x,
                y: lerp(sample_curve(a, x), sample_curve(b, x), t),
            }
        })
        .collect()
}
pub fn blend(
    a: &PortableLook,
    b: &PortableLook,
    amount: f32,
    name: impl Into<String>,
) -> PortableLook {
    let t = amount.clamp(0.0, 1.0);
    let l = |x, y| lerp(x, y, t);
    let tone = ToneParameters {
        exposure_ev: l(a.tone.exposure_ev, b.tone.exposure_ev),
        contrast: l(a.tone.contrast, b.tone.contrast),
        highlights: l(a.tone.highlights, b.tone.highlights),
        shadows: l(a.tone.shadows, b.tone.shadows),
        whites: l(a.tone.whites, b.tone.whites),
        blacks: l(a.tone.blacks, b.tone.blacks),
    };
    let mut mixer = ColorMixer::default();
    for i in 0..8 {
        mixer.bands[i] = BandAdjustment {
            hue_degrees: hue_lerp(
                a.color_mixer.bands[i].hue_degrees,
                b.color_mixer.bands[i].hue_degrees,
                t,
            ),
            chroma: l(a.color_mixer.bands[i].chroma, b.color_mixer.bands[i].chroma),
            lightness: l(
                a.color_mixer.bands[i].lightness,
                b.color_mixer.bands[i].lightness,
            ),
        };
    }
    mixer.hue_lock = if t < 0.5 {
        a.color_mixer.hue_lock
    } else {
        b.color_mixer.hue_lock
    };
    mixer.band_width_degrees = l(
        a.color_mixer.band_width_degrees,
        b.color_mixer.band_width_degrees,
    );
    let grading = GradingParameters {
        shadows: wheel(a.grading.shadows, b.grading.shadows, t),
        midtones: wheel(a.grading.midtones, b.grading.midtones, t),
        highlights: wheel(a.grading.highlights, b.grading.highlights, t),
        global: wheel(a.grading.global, b.grading.global, t),
        balance: l(a.grading.balance, b.grading.balance),
        blending: l(a.grading.blending, b.grading.blending),
        amount: l(a.grading.amount, b.grading.amount),
    };
    let name = name.into();
    let id = format!(
        "blend-{:x}",
        Sha256::digest(format!("{}:{}:{t:.6}", a.id, b.id).as_bytes())
    );
    PortableLook {
        schema: LOOK_SCHEMA.into(),
        version: 1,
        id,
        name,
        tone,
        relative_color: PortableRelativeColor {
            temperature: l(a.relative_color.temperature, b.relative_color.temperature),
            tint: l(a.relative_color.tint, b.relative_color.tint),
            vibrance: l(a.relative_color.vibrance, b.relative_color.vibrance),
            saturation: l(a.relative_color.saturation, b.relative_color.saturation),
        },
        curves: PortableCurves {
            master: curve_blend(&a.curves.master, &b.curves.master, t),
            red: curve_blend(&a.curves.red, &b.curves.red, t),
            green: curve_blend(&a.curves.green, &b.curves.green, t),
            blue: curve_blend(&a.curves.blue, &b.curves.blue, t),
        },
        color_mixer: mixer,
        grading,
        denoise: DenoiseParameters {
            luminance: l(a.denoise.luminance, b.denoise.luminance),
            chroma: l(a.denoise.chroma, b.denoise.chroma),
            radius: l(a.denoise.radius, b.denoise.radius),
            detail_protection: l(a.denoise.detail_protection, b.denoise.detail_protection),
            high_iso: l(a.denoise.high_iso, b.denoise.high_iso),
        },
        local_detail: LocalDetailParameters {
            texture: l(a.local_detail.texture, b.local_detail.texture),
            clarity: l(a.local_detail.clarity, b.local_detail.clarity),
            dehaze: l(a.local_detail.dehaze, b.local_detail.dehaze),
        },
        sharpen: SharpenParameters {
            amount: l(a.sharpen.amount, b.sharpen.amount),
            radius: l(a.sharpen.radius, b.sharpen.radius),
            detail: l(a.sharpen.detail, b.sharpen.detail),
            masking: l(a.sharpen.masking, b.sharpen.masking),
            halo_protection: l(a.sharpen.halo_protection, b.sharpen.halo_protection),
            threshold: l(a.sharpen.threshold, b.sharpen.threshold),
        },
        grain: GrainSettings {
            amount: l(a.grain.amount, b.grain.amount),
            size: l(a.grain.size, b.grain.size),
            roughness: l(a.grain.roughness, b.grain.roughness),
            seed: if t < 0.5 { a.grain.seed } else { b.grain.seed },
        },
        vignette: VignetteSettings {
            amount: l(a.vignette.amount, b.vignette.amount),
            midpoint: l(a.vignette.midpoint, b.vignette.midpoint),
            roundness: l(a.vignette.roundness, b.vignette.roundness),
            feather: l(a.vignette.feather, b.vignette.feather),
        },
    }
}

fn random_unit(seed: u64, x: u64, y: u64, c: u64) -> f32 {
    let mut z = seed
        ^ x.wrapping_mul(0x9E3779B185EBCA87)
        ^ y.wrapping_mul(0xC2B2AE3D27D4EB4F)
        ^ c.wrapping_mul(0x165667B19E3779F9);
    z ^= z >> 30;
    z = z.wrapping_mul(0xBF58476D1CE4E5B9);
    z ^= z >> 27;
    z = z.wrapping_mul(0x94D049BB133111EB);
    ((z ^ (z >> 31)) >> 40) as f32 / 16_777_215.0
}
pub fn apply_finishing_effects(
    image: &LinearImage,
    grain: GrainSettings,
    vignette: VignetteSettings,
    image_identity: &str,
) -> Result<LinearImage, LookError> {
    if image.data.len() != image.width * image.height * 3 {
        return Err(LookError::InvalidImage);
    }
    let identity = Sha256::digest(image_identity.as_bytes());
    let seed = grain.seed ^ u64::from_le_bytes(identity[..8].try_into().unwrap());
    let mut data = image.data.clone();
    let cx = (image.width.saturating_sub(1)) as f32 * 0.5;
    let cy = (image.height.saturating_sub(1)) as f32 * 0.5;
    let aspect = image.width as f32 / image.height.max(1) as f32;
    for y in 0..image.height {
        for x in 0..image.width {
            let i = (y * image.width + x) * 3;
            let dx = (x as f32 - cx) / cx.max(1.0);
            let dy = (y as f32 - cy) / cy.max(1.0);
            let roundness = (vignette.roundness.clamp(-1.0, 1.0) + 1.0) * 0.5;
            let aspect_correction = 1.0 + (aspect.max(1.0) - 1.0) * (1.0 - roundness);
            let radius = ((dx * aspect_correction).powi(2) + dy.powi(2)).sqrt();
            let edge = ((radius - vignette.midpoint.clamp(0.0, 1.0))
                / (vignette.feather.abs().max(0.02)))
            .clamp(0.0, 1.0);
            let smooth = edge * edge * (3.0 - 2.0 * edge);
            let vig_ev = -vignette.amount.clamp(-1.0, 1.0) * 2.0 * smooth;
            let scale = 2.0f32.powf(vig_ev);
            let luminance =
                (0.2627 * data[i] + 0.6780 * data[i + 1] + 0.0593 * data[i + 2]).max(0.0);
            let grain_gain = grain.amount.clamp(0.0, 1.0)
                * (0.003 + 0.025 * grain.size.clamp(0.0, 1.0))
                * (0.35 + 0.65 * luminance.sqrt());
            let mono = (random_unit(seed, x as u64, y as u64, 0) - 0.5) * 2.0;
            for c in 0..3 {
                let colored = (random_unit(seed, x as u64, y as u64, c as u64 + 1) - 0.5) * 2.0;
                let noise = lerp(mono, colored, grain.roughness.clamp(0.0, 1.0));
                data[i + c] = data[i + c] * scale + noise * grain_gain;
            }
        }
    }
    LinearImage::new(image.width, image.height, data).map_err(|_| LookError::InvalidImage)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn image() -> LinearImage {
        LinearImage::new(32, 24, vec![0.5; 32 * 24 * 3]).unwrap()
    }
    #[test]
    fn look_json_round_trip_and_schema_validation() {
        let a = PortableLook::default();
        assert_eq!(a, PortableLook::from_json(&a.to_json().unwrap()).unwrap());
        let mut b = a;
        b.version = 2;
        assert_eq!(b.validate(), Err(LookError::UnsupportedSchema));
    }
    #[test]
    fn amount_endpoints_and_circular_hue_are_semantic() {
        let a = PortableLook::default();
        let mut b = a.clone();
        b.id = "b".into();
        b.tone.exposure_ev = 2.0;
        b.grading.global.hue_degrees = 350.0;
        let mut c = a.clone();
        c.grading.global.hue_degrees = 10.0;
        assert_eq!(blend(&a, &b, 0.0, "x").tone, a.tone);
        assert_eq!(blend(&a, &b, 1.0, "x").tone, b.tone);
        assert!(
            blend(&c, &b, 0.5, "x").grading.global.hue_degrees.abs() < 1e-4
                || blend(&c, &b, 0.5, "x").grading.global.hue_degrees > 359.9
        );
    }
    #[test]
    fn curve_blend_is_sampled_and_monotonic_for_monotonic_inputs() {
        let mut a = PortableLook::default();
        let mut b = a.clone();
        a.curves.master = vec![CurvePoint { x: 0.0, y: 0.0 }, CurvePoint { x: 1.0, y: 1.0 }];
        b.curves.master = vec![
            CurvePoint { x: 0.0, y: 0.1 },
            CurvePoint { x: 0.5, y: 0.7 },
            CurvePoint { x: 1.0, y: 1.0 },
        ];
        let c = blend(&a, &b, 0.5, "c");
        assert!(c.curves.master.windows(2).all(|p| p[0].y <= p[1].y));
    }
    #[test]
    fn grain_and_vignette_are_deterministic_finite_and_hdr_safe() {
        let mut a = image();
        a.data[0] = 4.0;
        let g = GrainSettings {
            amount: 0.7,
            size: 0.5,
            roughness: 0.2,
            seed: 42,
        };
        let v = VignetteSettings {
            amount: 0.5,
            midpoint: 0.4,
            roundness: 0.0,
            feather: 0.5,
        };
        let x = apply_finishing_effects(&a, g, v, "id").unwrap();
        let y = apply_finishing_effects(&a, g, v, "id").unwrap();
        assert_eq!(x, y);
        assert!(x.data.iter().all(|v| v.is_finite()));
        assert!(x.data[0] > 1.0);
    }
}
