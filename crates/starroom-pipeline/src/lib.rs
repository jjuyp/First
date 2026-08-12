//! Native rendered-image CPU pipeline for Starroom v0.2.
//! This is the executable reference graph for JPEG/PNG/TIFF editing. Future wgpu stages must
//! match this pipeline within documented tolerances before replacing the CPU reference.

use serde::{Deserialize, Serialize};
use starroom_color::{
    ColorBand, ColorMixer, CurvePoint, LinearRgb, ToneParameters, apply_color_mixer, apply_tone,
    compress_to_unit_gamut, map_monotone_curve, oklab_to_oklch, oklab_to_rec2020, oklch_to_oklab,
    rec2020_to_oklab, sample_color_band,
};
use starroom_color_management::{
    ColorManagementError, InputProfileSource, LittleCmsProvider, OutputProfileSource,
    RenderingIntent,
};
use starroom_detail::{DenoiseParameters, LinearImage, SharpenParameters, denoise, sharpen};
use starroom_grading::{GradingParameters, apply_grading};
use starroom_imageio::{DecodedRenderedImage, DecodedSourceImage};
use starroom_raw::{CameraProfileDescriptor, CameraProfileStatus, DecodedRawImage};
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

/// White-balance intent is persisted independently from the creative colour controls.
/// `SourceDefault` means LibRaw camera WB for RAW and relative controls for encoded sources.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum WhiteBalanceMode {
    #[default]
    SourceDefault,
    AsShot,
    Camera,
    Auto,
    NeutralPicker,
    Relative,
}

/// Normalized source-space rectangle used by the native Neutral Picker.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WhiteBalanceSample {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl WhiteBalanceSample {
    fn validated(self) -> bool {
        [self.x, self.y, self.width, self.height]
            .into_iter()
            .all(f32::is_finite)
            && self.x >= 0.0
            && self.y >= 0.0
            && self.width > 0.0
            && self.height > 0.0
            && self.x + self.width <= 1.0
            && self.y + self.height <= 1.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct WhiteBalanceSettings {
    pub mode: WhiteBalanceMode,
    pub sample: Option<WhiteBalanceSample>,
}

/// M6 native tone curves.  Each curve uses the tested monotone cubic Hermite mapper.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ToneCurveSet {
    #[serde(default)]
    pub master: Vec<CurvePoint>,
    #[serde(default)]
    pub red: Vec<CurvePoint>,
    #[serde(default)]
    pub green: Vec<CurvePoint>,
    #[serde(default)]
    pub blue: Vec<CurvePoint>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RenderSettings {
    pub color_management: ColorManagementSettings,
    pub tone: ToneParameters,
    pub relative_color: RelativeColorParameters,
    pub white_balance: WhiteBalanceSettings,
    pub curve: Vec<CurvePoint>,
    #[serde(default)]
    pub curves: ToneCurveSet,
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
            white_balance: WhiteBalanceSettings::default(),
            curve: Vec::new(),
            curves: ToneCurveSet::default(),
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
    #[error("white-balance mode {mode:?} is not valid for {input_kind} input")]
    WhiteBalanceSemantic {
        mode: WhiteBalanceMode,
        input_kind: &'static str,
    },
    #[error("neutral-picker sample is missing or invalid")]
    InvalidWhiteBalanceSample,
    #[error(transparent)]
    ColorManagement(#[from] ColorManagementError),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SourceKind {
    Raw,
    Encoded,
}

fn neutral_scale(sum: [f32; 3], count: usize) -> Option<[f32; 3]> {
    if count == 0 || !sum.into_iter().all(f32::is_finite) {
        return None;
    }
    let mean = sum.map(|channel| channel / count as f32);
    // Green is the stable reference used by common RAW pipelines; refuse black/non-finite
    // samples instead of inventing a white point.
    if mean.iter().any(|channel| *channel <= 1.0e-6) {
        return None;
    }
    Some([mean[1] / mean[0], 1.0, mean[1] / mean[2]])
}

fn apply_diagonal_white_balance(pixels: &mut [[f32; 3]], scale: [f32; 3]) {
    for pixel in pixels {
        pixel[0] *= scale[0];
        pixel[1] *= scale[1];
        pixel[2] *= scale[2];
    }
}

pub trait AutoWhiteBalanceProvider {
    fn estimate_scale(&self, pixels: &[[f32; 3]]) -> Option<[f32; 3]>;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct GrayWorldAutoWhiteBalance;

impl AutoWhiteBalanceProvider for GrayWorldAutoWhiteBalance {
    fn estimate_scale(&self, pixels: &[[f32; 3]]) -> Option<[f32; 3]> {
        // Deterministic grey-world provider: reject very dark and clipped samples so highlights
        // and empty black borders do not define the estimated neutral. It is an active provider,
        // not a fallback for Camera/As-Shot WB.
        let mut sum = [0.0; 3];
        let mut count = 0;
        for pixel in pixels {
            let y = pixel[0] * 0.2627 + pixel[1] * 0.6780 + pixel[2] * 0.0593;
            if y.is_finite()
                && (0.01..=0.85).contains(&y)
                && pixel.iter().all(|v| v.is_finite() && *v > 0.0)
            {
                for (target, source) in sum.iter_mut().zip(pixel) {
                    *target += *source;
                }
                count += 1;
            }
        }
        neutral_scale(sum, count)
    }
}

fn picker_white_balance_scale(
    pixels: &[[f32; 3]],
    width: u32,
    height: u32,
    sample: WhiteBalanceSample,
) -> Option<[f32; 3]> {
    if !sample.validated() {
        return None;
    }
    let left = (sample.x * width as f32).floor() as usize;
    let top = (sample.y * height as f32).floor() as usize;
    let right = ((sample.x + sample.width) * width as f32).ceil() as usize;
    let bottom = ((sample.y + sample.height) * height as f32).ceil() as usize;
    let mut sum = [0.0; 3];
    let mut count = 0;
    for y in top.min(height as usize)..bottom.min(height as usize) {
        for x in left.min(width as usize)..right.min(width as usize) {
            let pixel = pixels[y * width as usize + x];
            if pixel.iter().all(|v| v.is_finite() && *v > 1.0e-6) {
                for (target, source) in sum.iter_mut().zip(pixel) {
                    *target += source;
                }
                count += 1;
            }
        }
    }
    neutral_scale(sum, count)
}

fn apply_white_balance(
    pixels: &mut [[f32; 3]],
    width: u32,
    height: u32,
    source: SourceKind,
    settings: WhiteBalanceSettings,
) -> Result<(), PipelineError> {
    match (source, settings.mode) {
        // LibRaw has already applied the recorded Camera Neutral / As-Shot multipliers before
        // the RAW data reaches the linear Rec.2020 graph. The modes stay explicit so projects
        // preserve the photographer's intent and no encoded-image WB is silently substituted.
        (
            SourceKind::Raw,
            WhiteBalanceMode::SourceDefault | WhiteBalanceMode::AsShot | WhiteBalanceMode::Camera,
        ) => Ok(()),
        (SourceKind::Encoded, WhiteBalanceMode::SourceDefault | WhiteBalanceMode::Relative) => {
            Ok(())
        }
        (SourceKind::Encoded, WhiteBalanceMode::AsShot | WhiteBalanceMode::Camera) => {
            Err(PipelineError::WhiteBalanceSemantic {
                mode: settings.mode,
                input_kind: "encoded",
            })
        }
        (_, WhiteBalanceMode::Auto) => {
            let scale = GrayWorldAutoWhiteBalance
                .estimate_scale(pixels)
                .ok_or(PipelineError::InvalidWhiteBalanceSample)?;
            apply_diagonal_white_balance(pixels, scale);
            Ok(())
        }
        (_, WhiteBalanceMode::NeutralPicker) => {
            let sample = settings
                .sample
                .ok_or(PipelineError::InvalidWhiteBalanceSample)?;
            let scale = picker_white_balance_scale(pixels, width, height, sample)
                .ok_or(PipelineError::InvalidWhiteBalanceSample)?;
            apply_diagonal_white_balance(pixels, scale);
            Ok(())
        }
        (SourceKind::Raw, WhiteBalanceMode::Relative) => Err(PipelineError::WhiteBalanceSemantic {
            mode: settings.mode,
            input_kind: "RAW",
        }),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColorTransformReport {
    pub input: InputProfileSource,
    pub output: OutputProfileSource,
    pub working_space: &'static str,
    pub camera_profile_id: Option<String>,
    pub camera_profile_hash: Option<String>,
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

fn apply_one_curve(value: f32, curve: &[CurvePoint]) -> f32 {
    if curve.len() < 2 {
        value
    } else {
        map_monotone_curve(value, curve)
    }
}
fn apply_curve(rgb: LinearRgb, legacy: &[CurvePoint], curves: &ToneCurveSet) -> LinearRgb {
    let master = if curves.master.len() >= 2 {
        &curves.master
    } else {
        legacy
    };
    let rgb = LinearRgb {
        r: apply_one_curve(rgb.r, master),
        g: apply_one_curve(rgb.g, master),
        b: apply_one_curve(rgb.b, master),
    };
    LinearRgb {
        r: apply_one_curve(rgb.r, &curves.red),
        g: apply_one_curve(rgb.g, &curves.green),
        b: apply_one_curve(rgb.b, &curves.blue),
    }
}

fn apply_creative_graph(
    pixels: Vec<[f32; 3]>,
    settings: &RenderSettings,
) -> Result<Vec<f32>, PipelineError> {
    let mut data = Vec::with_capacity(pixels.len() * 3);
    for pixel in pixels {
        let mut rgb = LinearRgb {
            r: pixel[0],
            g: pixel[1],
            b: pixel[2],
        };
        rgb = apply_relative_color(rgb, settings.relative_color);
        rgb = apply_tone(rgb, settings.tone);
        rgb = apply_curve(rgb, &settings.curve, &settings.curves);
        rgb = apply_color_mixer(rgb, settings.color_mixer);
        rgb = apply_grading(rgb, settings.grading);
        if !rgb.r.is_finite() || !rgb.g.is_finite() || !rgb.b.is_finite() {
            return Err(PipelineError::InvalidDecodedBuffer);
        }
        data.extend_from_slice(&[rgb.r, rgb.g, rgb.b]);
    }
    Ok(data)
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

    apply_white_balance(
        &mut pixels,
        decoded.width,
        decoded.height,
        SourceKind::Encoded,
        settings.white_balance,
    )?;
    let data = apply_creative_graph(pixels, settings)?;

    let image = LinearImage::new(decoded.width as usize, decoded.height as usize, data)
        .map_err(|_| PipelineError::DetailBuffer)?;
    Ok((image, input_source))
}

fn to_working_raw(
    decoded: &DecodedRawImage,
    settings: &RenderSettings,
) -> Result<LinearImage, PipelineError> {
    let expected = decoded.width as usize * decoded.height as usize * 3;
    if decoded.rgb.len() != expected {
        return Err(PipelineError::InvalidDecodedBuffer);
    }
    let mut pixels: Vec<[f32; 3]> = decoded
        .rgb
        .chunks_exact(3)
        .map(|pixel| [pixel[0], pixel[1], pixel[2]])
        .collect();
    apply_white_balance(
        &mut pixels,
        decoded.width,
        decoded.height,
        SourceKind::Raw,
        settings.white_balance,
    )?;
    let data = apply_creative_graph(pixels, settings)?;
    LinearImage::new(decoded.width as usize, decoded.height as usize, data)
        .map_err(|_| PipelineError::DetailBuffer)
}

/// Samples the actual native working graph at normalized image coordinates for M7's targeted
/// Color Mixer tool. The browser transports only the selected enum, never image pixels or color
/// science. RAW and encoded inputs therefore use exactly the same decode/WB/creative stages as
/// preview and export.
pub fn sample_source_color_band(
    decoded: &DecodedSourceImage,
    settings: &RenderSettings,
    x: f32,
    y: f32,
) -> Result<Option<ColorBand>, PipelineError> {
    if !x.is_finite() || !y.is_finite() || !(0.0..=1.0).contains(&x) || !(0.0..=1.0).contains(&y) {
        return Err(PipelineError::InvalidDecodedBuffer);
    }
    let (image, width, height) = match decoded {
        DecodedSourceImage::Rendered(source) => {
            let (image, _) = to_working_image(source, settings)?;
            (image, source.width as usize, source.height as usize)
        }
        DecodedSourceImage::Raw(source) => (
            to_working_raw(source, settings)?,
            source.width as usize,
            source.height as usize,
        ),
    };
    let px = ((x * width as f32).floor() as usize).min(width.saturating_sub(1));
    let py = ((y * height as f32).floor() as usize).min(height.saturating_sub(1));
    let offset = (py * width + px) * 3;
    Ok(sample_color_band(LinearRgb {
        r: image.data[offset],
        g: image.data[offset + 1],
        b: image.data[offset + 2],
    }))
}

fn render_working_graph(
    working: LinearImage,
    width: u32,
    height: u32,
    input_source: InputProfileSource,
    camera_profile: Option<&CameraProfileDescriptor>,
    settings: &RenderSettings,
    output_icc: Option<&[u8]>,
) -> Result<RenderedRgb8, PipelineError> {
    let denoised = denoise(&working, settings.denoise);
    let detailed = sharpen(&denoised, settings.sharpen);
    let mut pixels = Vec::with_capacity(width as usize * height as usize);
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
        width,
        height,
        data: output,
        color: ColorTransformReport {
            input: input_source,
            output: output_source,
            working_space: "linear Rec.2020 D65",
            camera_profile_id: camera_profile.map(|profile| profile.id.clone()),
            camera_profile_hash: camera_profile.map(|profile| profile.hash.clone()),
        },
    })
}

fn render_shared_graph(
    decoded: &DecodedRenderedImage,
    settings: &RenderSettings,
    output_icc: Option<&[u8]>,
) -> Result<RenderedRgb8, PipelineError> {
    let (working, input_source) = to_working_image(decoded, settings)?;
    render_working_graph(
        working,
        decoded.width,
        decoded.height,
        input_source,
        None,
        settings,
        output_icc,
    )
}

fn render_shared_source_graph(
    decoded: &DecodedSourceImage,
    settings: &RenderSettings,
    output_icc: Option<&[u8]>,
) -> Result<RenderedRgb8, PipelineError> {
    match decoded {
        DecodedSourceImage::Rendered(image) => render_shared_graph(image, settings, output_icc),
        DecodedSourceImage::Raw(image) => {
            let input_source = match image.metadata.camera_profile.status {
                CameraProfileStatus::Resolved => InputProfileSource::RawCameraMatrix,
                CameraProfileStatus::Generic => InputProfileSource::RawGenericProfile,
            };
            render_working_graph(
                to_working_raw(image, settings)?,
                image.width,
                image.height,
                input_source,
                Some(&image.metadata.camera_profile),
                settings,
                output_icc,
            )
        }
    }
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

pub fn render_source_preview_to_srgb8(
    decoded: &DecodedSourceImage,
    settings: &RenderSettings,
) -> Result<RenderedRgb8, PipelineError> {
    render_shared_source_graph(decoded, settings, None)
}

pub fn render_source_export_to_srgb8(
    decoded: &DecodedSourceImage,
    settings: &RenderSettings,
) -> Result<RenderedRgb8, PipelineError> {
    render_shared_source_graph(decoded, settings, None)
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
    use starroom_grading::ColorWheel;
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
    fn m8_four_way_grading_preview_export_share_native_stage() {
        let decoded = fixture(&[
            [0.62, 0.35, 0.24, 1.0],
            [0.08, 0.1, 0.18, 1.0],
            [1.4, 0.1, 0.9, 1.0],
        ]);
        let settings = RenderSettings {
            grading: GradingParameters {
                shadows: ColorWheel {
                    hue_degrees: 225.0,
                    chroma: 0.35,
                    lightness: -0.08,
                },
                midtones: ColorWheel {
                    hue_degrees: 35.0,
                    chroma: 0.2,
                    lightness: 0.04,
                },
                highlights: ColorWheel {
                    hue_degrees: 55.0,
                    chroma: 0.12,
                    lightness: -0.02,
                },
                global: ColorWheel {
                    hue_degrees: 310.0,
                    chroma: 0.04,
                    lightness: 0.0,
                },
                balance: 0.1,
                blending: 0.7,
                amount: 0.85,
            },
            ..Default::default()
        };
        let preview = render_preview_to_srgb8(&decoded, &settings).expect("preview");
        let export = render_export_to_srgb8(&decoded, &settings).expect("export");
        assert_eq!(preview, export);
        assert_ne!(
            preview.data,
            render_to_srgb8(&decoded, &RenderSettings::default())
                .expect("baseline")
                .data
        );
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
    fn native_rgb_curves_are_channel_specific_and_preview_export_match() {
        let decoded = fixture(&[[0.5, 0.5, 0.5, 1.0]]);
        let settings = RenderSettings {
            curves: ToneCurveSet {
                red: vec![CurvePoint { x: 0.0, y: 0.0 }, CurvePoint { x: 1.0, y: 0.7 }],
                ..Default::default()
            },
            ..Default::default()
        };
        let preview = render_preview_to_srgb8(&decoded, &settings).expect("preview");
        let export = render_export_to_srgb8(&decoded, &settings).expect("export");
        assert_eq!(preview, export);
        assert!(preview.data[0] < preview.data[1]);
    }

    #[test]
    fn master_identity_curve_preserves_portrait_and_gradient_golden_vector() {
        let decoded = fixture(&[
            [0.62, 0.35, 0.24, 1.0],
            [0.05, 0.05, 0.05, 1.0],
            [0.25, 0.25, 0.25, 1.0],
            [0.75, 0.75, 0.75, 1.0],
        ]);
        let identity = vec![CurvePoint { x: 0.0, y: 0.0 }, CurvePoint { x: 1.0, y: 1.0 }];
        let baseline = render_to_srgb8(&decoded, &RenderSettings::default()).expect("baseline");
        let curved = render_to_srgb8(
            &decoded,
            &RenderSettings {
                curves: ToneCurveSet {
                    master: identity,
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .expect("identity");
        assert_eq!(baseline.data, curved.data);
    }

    #[test]
    fn s_curve_changes_gradient_ends_while_preserving_midpoint() {
        let curves = ToneCurveSet {
            master: vec![
                CurvePoint { x: 0.0, y: 0.0 },
                CurvePoint { x: 0.25, y: 0.16 },
                CurvePoint { x: 0.5, y: 0.5 },
                CurvePoint { x: 0.75, y: 0.86 },
                CurvePoint { x: 1.0, y: 1.0 },
            ],
            ..Default::default()
        };
        let dark = apply_curve(
            LinearRgb {
                r: 0.2,
                g: 0.2,
                b: 0.2,
            },
            &[],
            &curves,
        );
        let middle = apply_curve(
            LinearRgb {
                r: 0.5,
                g: 0.5,
                b: 0.5,
            },
            &[],
            &curves,
        );
        let bright = apply_curve(
            LinearRgb {
                r: 0.8,
                g: 0.8,
                b: 0.8,
            },
            &[],
            &curves,
        );
        assert!(dark.r < 0.2);
        assert!((middle.r - 0.5).abs() <= 1.0e-6);
        assert!(bright.r > 0.8);
    }

    #[test]
    fn extreme_curves_remain_finite_for_hdr_working_values() {
        let curves = ToneCurveSet {
            master: vec![
                CurvePoint { x: 0.0, y: 0.2 },
                CurvePoint { x: 0.5, y: 0.95 },
                CurvePoint { x: 1.0, y: 1.0 },
            ],
            red: vec![CurvePoint { x: 0.0, y: 1.0 }, CurvePoint { x: 1.0, y: 0.0 }],
            ..Default::default()
        };
        for rgb in [
            LinearRgb {
                r: -0.25,
                g: 0.0,
                b: 0.2,
            },
            LinearRgb {
                r: 1.5,
                g: 4.0,
                b: 12.0,
            },
        ] {
            let result = apply_curve(rgb, &[], &curves);
            assert!(
                [result.r, result.g, result.b]
                    .into_iter()
                    .all(f32::is_finite)
            );
        }
    }

    #[test]
    fn neutral_picker_removes_a_measured_encoded_colour_cast() {
        let decoded = fixture(&[[0.48, 0.36, 0.24, 1.0], [0.48, 0.36, 0.24, 1.0]]);
        let output = render_to_srgb8(
            &decoded,
            &RenderSettings {
                white_balance: WhiteBalanceSettings {
                    mode: WhiteBalanceMode::NeutralPicker,
                    sample: Some(WhiteBalanceSample {
                        x: 0.0,
                        y: 0.0,
                        width: 1.0,
                        height: 1.0,
                    }),
                },
                ..Default::default()
            },
        )
        .expect("picker render");
        assert!((i16::from(output.data[0]) - i16::from(output.data[1])).abs() <= 1);
        assert!((i16::from(output.data[1]) - i16::from(output.data[2])).abs() <= 1);
    }

    #[test]
    fn auto_white_balance_is_active_and_extreme_pixels_stay_finite() {
        let decoded = fixture(&[
            [0.9, 0.6, 0.3, 1.0],
            [0.72, 0.48, 0.24, 1.0],
            [4.0, 1.0, 0.1, 1.0],
            [0.001, 0.001, 0.001, 1.0],
        ]);
        let output = render_to_srgb8(
            &decoded,
            &RenderSettings {
                white_balance: WhiteBalanceSettings {
                    mode: WhiteBalanceMode::Auto,
                    sample: None,
                },
                ..Default::default()
            },
        )
        .expect("auto render");
        assert!(output.data.iter().any(|value| *value > 0));
    }

    #[test]
    fn skin_and_mixed_lighting_white_balance_regression_stays_warm_and_finite() {
        let decoded = fixture(&[
            [0.68, 0.42, 0.30, 1.0],
            [0.55, 0.37, 0.29, 1.0],
            [0.22, 0.31, 0.58, 1.0],
            [0.62, 0.48, 0.25, 1.0],
        ]);
        let output = render_to_srgb8(
            &decoded,
            &RenderSettings {
                white_balance: WhiteBalanceSettings {
                    mode: WhiteBalanceMode::Auto,
                    sample: None,
                },
                ..Default::default()
            },
        )
        .expect("mixed-light Auto WB");
        assert!(
            output.data[0] > output.data[2],
            "skin sample must retain warm ordering"
        );
        assert_eq!(output.data.len(), 12);
    }

    #[test]
    fn encoded_camera_white_balance_is_a_typed_error_not_a_silent_fallback() {
        let decoded = fixture(&[[0.4, 0.4, 0.4, 1.0]]);
        let result = render_to_srgb8(
            &decoded,
            &RenderSettings {
                white_balance: WhiteBalanceSettings {
                    mode: WhiteBalanceMode::Camera,
                    sample: None,
                },
                ..Default::default()
            },
        );
        assert!(matches!(
            result,
            Err(PipelineError::WhiteBalanceSemantic {
                mode: WhiteBalanceMode::Camera,
                input_kind: "encoded"
            })
        ));
    }

    #[test]
    fn invalid_picker_sample_is_rejected_before_rendering() {
        let decoded = fixture(&[[0.4, 0.4, 0.4, 1.0]]);
        let result = render_to_srgb8(
            &decoded,
            &RenderSettings {
                white_balance: WhiteBalanceSettings {
                    mode: WhiteBalanceMode::NeutralPicker,
                    sample: Some(WhiteBalanceSample {
                        x: 0.8,
                        y: 0.0,
                        width: 0.4,
                        height: 1.0,
                    }),
                },
                ..Default::default()
            },
        );
        assert!(matches!(
            result,
            Err(PipelineError::InvalidWhiteBalanceSample)
        ));
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
