//! Native rendered-image CPU pipeline for Starroom v0.2.
//! This is the executable reference graph for JPEG/PNG/TIFF editing. Future wgpu stages must
//! match this pipeline within documented tolerances before replacing the CPU reference.

use serde::{Deserialize, Serialize};
use starroom_color::{
    ColorMixer, CurvePoint, LinearRgb, ToneParameters, apply_color_mixer, apply_tone,
    compress_to_unit_gamut, map_monotone_curve, oklab_to_oklch, oklab_to_rec2020,
    oklch_to_oklab, rec2020_to_oklab,
};
use starroom_color_management::{
    rec2020_linear_to_srgb_encoded, srgb_encoded_to_rec2020_linear,
};
use starroom_detail::{DenoiseParameters, LinearImage, SharpenParameters, denoise, sharpen};
use starroom_grading::{GradingParameters, apply_grading};
use starroom_imageio::DecodedRenderedImage;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub struct RelativeColorParameters {
    /// Encoded-image relative warm/cool correction in -1..1. Not a physical Kelvin value.
    pub temperature: f32,
    /// Encoded-image relative green/magenta correction in -1..1.
    pub tint: f32,
    pub vibrance: f32,
    pub saturation: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RenderSettings {
    pub tone: ToneParameters,
    pub relative_color: RelativeColorParameters,
    pub curve: Vec<CurvePoint>,
    pub color_mixer: ColorMixer,
    pub grading: GradingParameters,
    pub denoise: DenoiseParameters,
    pub sharpen: SharpenParameters,
}

impl Default for RenderSettings {
    fn default() -> Self {
        Self {
            tone: ToneParameters::default(),
            relative_color: RelativeColorParameters::default(),
            curve: Vec::new(),
            color_mixer: ColorMixer::default(),
            grading: GradingParameters::default(),
            denoise: DenoiseParameters::default(),
            sharpen: SharpenParameters {
                amount: 0.0,
                ..Default::default()
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PipelineError {
    InvalidDecodedBuffer,
    DetailBuffer,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RenderedRgb8 {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

fn apply_relative_color(rgb: LinearRgb, parameters: RelativeColorParameters) -> LinearRgb {
    let temperature = parameters.temperature.clamp(-1.0, 1.0);
    let tint = parameters.tint.clamp(-1.0, 1.0);
    let mut lab = rec2020_to_oklab(rgb);
    // This is deliberately labeled relative editing rather than Kelvin. Positive b is warmer;
    // positive a is more magenta in Oklab opponent coordinates.
    lab.b += temperature * 0.035;
    lab.a += tint * 0.025;

    let mut lch = oklab_to_oklch(lab);
    let saturation = parameters.saturation.clamp(-1.0, 1.0);
    let vibrance = parameters.vibrance.clamp(-1.0, 1.0);
    let normalized_chroma = (lch.c / 0.32).clamp(0.0, 1.0);
    let vibrance_weight = 1.0 - normalized_chroma;
    let scale = (1.0 + saturation * 0.85 + vibrance * vibrance_weight * 0.65).max(0.0);
    lch.c *= scale;
    oklab_to_rec2020(oklch_to_oklab(lch))
}

fn apply_curve(rgb: LinearRgb, curve: &[CurvePoint]) -> LinearRgb {
    if curve.len() < 2 {
        return rgb;
    }
    LinearRgb {
        r: map_monotone_curve(rgb.r, curve),
        g: map_monotone_curve(rgb.g, curve),
        b: map_monotone_curve(rgb.b, curve),
    }
}

fn to_working_image(
    decoded: &DecodedRenderedImage,
    settings: &RenderSettings,
) -> Result<LinearImage, PipelineError> {
    let expected = decoded.width as usize * decoded.height as usize * 4;
    if decoded.rgba.len() != expected {
        return Err(PipelineError::InvalidDecodedBuffer);
    }

    let mut data = Vec::with_capacity(decoded.width as usize * decoded.height as usize * 3);
    for rgba in decoded.rgba.chunks_exact(4) {
        let mut rgb = srgb_encoded_to_rec2020_linear([rgba[0], rgba[1], rgba[2]]);
        rgb = apply_relative_color(rgb, settings.relative_color);
        rgb = apply_tone(rgb, settings.tone);
        rgb = apply_curve(rgb, &settings.curve);
        rgb = apply_color_mixer(rgb, settings.color_mixer);
        rgb = apply_grading(rgb, settings.grading);
        data.extend_from_slice(&[rgb.r, rgb.g, rgb.b]);
    }

    LinearImage::new(decoded.width as usize, decoded.height as usize, data)
        .map_err(|_| PipelineError::DetailBuffer)
}

pub fn render_to_srgb8(
    decoded: &DecodedRenderedImage,
    settings: &RenderSettings,
) -> Result<RenderedRgb8, PipelineError> {
    let working = to_working_image(decoded, settings)?;
    let denoised = denoise(&working, settings.denoise);
    let detailed = sharpen(&denoised, settings.sharpen);
    let mut output = Vec::with_capacity(decoded.width as usize * decoded.height as usize * 3);

    for pixel in detailed.data.chunks_exact(3) {
        let working_rgb = compress_to_unit_gamut(LinearRgb {
            r: pixel[0],
            g: pixel[1],
            b: pixel[2],
        });
        let encoded = rec2020_linear_to_srgb_encoded(working_rgb);
        for channel in encoded {
            output.push((channel.clamp(0.0, 1.0) * 255.0).round() as u8);
        }
    }

    Ok(RenderedRgb8 {
        width: decoded.width,
        height: decoded.height,
        data: output,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use starroom_color::{BandAdjustment, ColorBand};
    use starroom_imageio::RenderedFormat;

    fn fixture(values: &[[f32; 4]]) -> DecodedRenderedImage {
        DecodedRenderedImage {
            width: values.len() as u32,
            height: 1,
            format: RenderedFormat::Png,
            rgba: values.iter().flat_map(|pixel| pixel.iter().copied()).collect(),
            embedded_icc: None,
            exif: None,
        }
    }

    #[test]
    fn neutral_pipeline_preserves_rendered_gray_nearly_exactly() {
        let decoded = fixture(&[[0.25, 0.25, 0.25, 1.0], [0.7, 0.7, 0.7, 1.0]]);
        let output = render_to_srgb8(&decoded, &RenderSettings::default()).expect("render");
        assert!((i16::from(output.data[0]) - 64).abs() <= 1);
        assert!((i16::from(output.data[3]) - 179).abs() <= 1);
        assert_eq!(output.data[0], output.data[1]);
        assert_eq!(output.data[1], output.data[2]);
    }

    #[test]
    fn shadow_control_targets_dark_pixel_more_than_mid_pixel() {
        let decoded = fixture(&[[0.12, 0.10, 0.08, 1.0], [0.5, 0.45, 0.4, 1.0]]);
        let baseline = render_to_srgb8(&decoded, &RenderSettings::default()).expect("baseline");
        let settings = RenderSettings {
            tone: ToneParameters {
                shadows: 0.5,
                ..Default::default()
            },
            ..Default::default()
        };
        let adjusted = render_to_srgb8(&decoded, &settings).expect("adjusted");
        let dark_gain = i16::from(adjusted.data[0]) - i16::from(baseline.data[0]);
        let mid_gain = i16::from(adjusted.data[3]) - i16::from(baseline.data[3]);
        assert!(dark_gain > 0);
        assert!(dark_gain > mid_gain * 2);
    }

    #[test]
    fn oklch_color_mixer_changes_selected_color() {
        let decoded = fixture(&[[0.8, 0.2, 0.12, 1.0]]);
        let baseline = render_to_srgb8(&decoded, &RenderSettings::default()).expect("baseline");
        let settings = RenderSettings {
            color_mixer: ColorMixer::default().with_band(
                ColorBand::Red,
                BandAdjustment {
                    hue_degrees: 20.0,
                    chroma: 0.2,
                    lightness: 0.0,
                },
            ),
            ..Default::default()
        };
        let adjusted = render_to_srgb8(&decoded, &settings).expect("adjusted");
        assert_ne!(baseline.data, adjusted.data);
    }

    #[test]
    fn relative_temperature_warms_neutral_pixel_without_kelvin_claim() {
        let decoded = fixture(&[[0.5, 0.5, 0.5, 1.0]]);
        let settings = RenderSettings {
            relative_color: RelativeColorParameters {
                temperature: 0.7,
                ..Default::default()
            },
            ..Default::default()
        };
        let output = render_to_srgb8(&decoded, &settings).expect("render");
        assert!(output.data[0] > output.data[2]);
    }
}
