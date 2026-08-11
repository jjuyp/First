//! Native rendered-image CPU pipeline for Starroom v0.2.
//! This is the executable reference graph for JPEG/PNG/TIFF editing. Future wgpu stages must
//! match this pipeline within documented tolerances before replacing the CPU reference.

use serde::{Deserialize, Serialize};
use starroom_color::{
    ColorMixer, CurvePoint, LinearRgb, ToneParameters, apply_color_mixer, apply_tone,
    compress_to_unit_gamut, map_monotone_curve, oklab_to_oklch, oklab_to_rec2020, oklch_to_oklab,
    rec2020_to_oklab,
};
use starroom_color_management::{
    ColorManagementError, InputProfileSource, LittleCmsProvider, OutputProfileSource,
    RenderingIntent,
};
use starroom_detail::{DenoiseParameters, LinearImage, SharpenParameters, denoise, sharpen};
use starroom_grading::{GradingParameters, apply_grading};
use starroom_imageio::DecodedRenderedImage;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ColorManagementSettings {
    pub intent: RenderingIntent,
    pub black_point_compensation: bool,
}

impl Default for ColorManagementSettings {
    fn default() -> Self {
        Self {
            intent: RenderingIntent::RelativeColorimetric,
            black_point_compensation: true,
        }
    }
}

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
    pub color_management: ColorManagementSettings,
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
            color_management: ColorManagementSettings::default(),
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

#[derive(Debug, Error)]
pub enum PipelineError {
    #[error("decoded RGBA buffer length does not match dimensions")]
    InvalidDecodedBuffer,
    #[error("detail image buffer is invalid")]
    DetailBuffer,
    #[error(transparent)]
    ColorManagement(#[from] ColorManagementError),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ColorTransformReport {
    pub input: InputProfileSource,
    pub output: OutputProfileSource,
    pub working_space: &'static str,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RenderedRgb8 {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
    pub color: ColorTransformReport,
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
) -> Result<(LinearImage, InputProfileSource), PipelineError> {
    let expected = decoded.width as usize * decoded.height as usize * 4;
    if decoded.rgba.len() != expected {
        return Err(PipelineError::InvalidDecodedBuffer);
    }

    let mut pixels: Vec<[f32; 3]> = decoded
        .rgba
        .chunks_exact(4)
        .map(|rgba| [rgba[0], rgba[1], rgba[2]])
        .collect();
    let input_source = LittleCmsProvider.input_to_working(
        &mut pixels,
        decoded.embedded_icc.as_deref(),
        settings.color_management.intent,
        settings.color_management.black_point_compensation,
    )?;

    let mut data = Vec::with_capacity(pixels.len() * 3);
    for pixel in pixels {
        let mut rgb = LinearRgb {
            r: pixel[0],
            g: pixel[1],
            b: pixel[2],
        };
        rgb = apply_relative_color(rgb, settings.relative_color);
        rgb = apply_tone(rgb, settings.tone);
        rgb = apply_curve(rgb, &settings.curve);
        rgb = apply_color_mixer(rgb, settings.color_mixer);
        rgb = apply_grading(rgb, settings.grading);
        data.extend_from_slice(&[rgb.r, rgb.g, rgb.b]);
    }

    let image = LinearImage::new(decoded.width as usize, decoded.height as usize, data)
        .map_err(|_| PipelineError::DetailBuffer)?;
    Ok((image, input_source))
}

fn render_shared_graph(
    decoded: &DecodedRenderedImage,
    settings: &RenderSettings,
    output_icc: Option<&[u8]>,
) -> Result<RenderedRgb8, PipelineError> {
    let (working, input_source) = to_working_image(decoded, settings)?;
    let denoised = denoise(&working, settings.denoise);
    let detailed = sharpen(&denoised, settings.sharpen);
    let mut pixels = Vec::with_capacity(decoded.width as usize * decoded.height as usize);
    for pixel in detailed.data.chunks_exact(3) {
        let working_rgb = compress_to_unit_gamut(LinearRgb {
            r: pixel[0],
            g: pixel[1],
            b: pixel[2],
        });
        pixels.push([working_rgb.r, working_rgb.g, working_rgb.b]);
    }
    let output_source = LittleCmsProvider.working_to_output(
        &mut pixels,
        output_icc,
        settings.color_management.intent,
        settings.color_management.black_point_compensation,
    )?;
    let mut output = Vec::with_capacity(pixels.len() * 3);
    for encoded in pixels {
        for channel in encoded {
            output.push((channel.clamp(0.0, 1.0) * 255.0).round() as u8);
        }
    }

    Ok(RenderedRgb8 {
        width: decoded.width,
        height: decoded.height,
        data: output,
        color: ColorTransformReport {
            input: input_source,
            output: output_source,
            working_space: "linear Rec.2020 D65",
        },
    })
}

/// Preview and export deliberately enter the same graph; only the requested output profile differs.
pub fn render_preview_to_srgb8(
    decoded: &DecodedRenderedImage,
    settings: &RenderSettings,
) -> Result<RenderedRgb8, PipelineError> {
    render_shared_graph(decoded, settings, None)
}

pub fn render_preview_to_display_icc8(
    decoded: &DecodedRenderedImage,
    settings: &RenderSettings,
    display_icc: &[u8],
) -> Result<RenderedRgb8, PipelineError> {
    render_shared_graph(decoded, settings, Some(display_icc))
}

pub fn render_export_to_srgb8(
    decoded: &DecodedRenderedImage,
    settings: &RenderSettings,
) -> Result<RenderedRgb8, PipelineError> {
    render_shared_graph(decoded, settings, None)
}

pub fn render_export_to_icc8(
    decoded: &DecodedRenderedImage,
    settings: &RenderSettings,
    output_icc: &[u8],
) -> Result<RenderedRgb8, PipelineError> {
    render_shared_graph(decoded, settings, Some(output_icc))
}

/// Compatibility entry point. New callers should name preview or export explicitly.
pub fn render_to_srgb8(
    decoded: &DecodedRenderedImage,
    settings: &RenderSettings,
) -> Result<RenderedRgb8, PipelineError> {
    render_preview_to_srgb8(decoded, settings)
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
            rgba: values
                .iter()
                .flat_map(|pixel| pixel.iter().copied())
                .collect(),
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

    #[test]
    fn preview_and_export_share_identical_srgb_graph() {
        let decoded = fixture(&[[0.15, 0.3, 0.8, 1.0], [0.8, 0.45, 0.1, 1.0]]);
        let settings = RenderSettings {
            tone: ToneParameters {
                exposure_ev: 0.4,
                ..Default::default()
            },
            ..Default::default()
        };
        let preview = render_preview_to_srgb8(&decoded, &settings).expect("preview");
        let export = render_export_to_srgb8(&decoded, &settings).expect("export");
        assert_eq!(preview, export);
        assert_eq!(preview.color.input, InputProfileSource::AssumedSrgb);
        assert_eq!(preview.color.output, OutputProfileSource::Srgb);
    }

    #[test]
    fn embedded_icc_is_used_by_shared_graph() {
        let mut decoded = fixture(&[[0.3, 0.5, 0.7, 1.0]]);
        decoded.embedded_icc = Some(
            LittleCmsProvider
                .srgb_profile_bytes()
                .expect("serialize sRGB profile"),
        );
        let output = render_preview_to_srgb8(&decoded, &RenderSettings::default())
            .expect("profiled preview");
        assert_eq!(output.color.input, InputProfileSource::EmbeddedIcc);
    }

    #[test]
    fn invalid_embedded_icc_fails_the_shared_graph() {
        let mut decoded = fixture(&[[0.3, 0.5, 0.7, 1.0]]);
        decoded.embedded_icc = Some(b"broken profile".to_vec());
        let result = render_preview_to_srgb8(&decoded, &RenderSettings::default());
        assert!(matches!(
            result,
            Err(PipelineError::ColorManagement(
                ColorManagementError::InvalidProfile { .. }
            ))
        ));
    }

    #[test]
    fn supplied_output_profile_is_applied_and_reported() {
        let decoded = fixture(&[[0.2, 0.4, 0.6, 1.0]]);
        let output_profile = LittleCmsProvider
            .srgb_profile_bytes()
            .expect("serialize sRGB profile");
        let output = render_export_to_icc8(&decoded, &RenderSettings::default(), &output_profile)
            .expect("profiled export");
        assert_eq!(output.color.output, OutputProfileSource::SuppliedIcc);
    }

    #[test]
    fn display_profile_uses_the_same_preview_graph() {
        let decoded = fixture(&[[0.2, 0.4, 0.6, 1.0]]);
        let display_profile = LittleCmsProvider
            .srgb_profile_bytes()
            .expect("serialize display profile");
        let display =
            render_preview_to_display_icc8(&decoded, &RenderSettings::default(), &display_profile)
                .expect("display preview");
        let fallback = render_preview_to_srgb8(&decoded, &RenderSettings::default())
            .expect("fallback preview");
        assert_eq!(display.data, fallback.data);
        assert_eq!(display.color.output, OutputProfileSource::SuppliedIcc);
    }

    #[test]
    fn invalid_output_icc_fails_instead_of_falling_back() {
        let decoded = fixture(&[[0.2, 0.4, 0.6, 1.0]]);
        let result = render_export_to_icc8(
            &decoded,
            &RenderSettings::default(),
            b"broken output profile",
        );
        assert!(matches!(
            result,
            Err(PipelineError::ColorManagement(
                ColorManagementError::InvalidProfile { .. }
            ))
        ));
    }
}
