use serde::{Deserialize, Serialize};
use starroom_advisor::{AnalysisStats, Suggestion, advise};
use starroom_color::{ColorMixer, CurvePoint, ToneParameters};
use starroom_detail::{DenoiseParameters, LocalDetailParameters, SharpenParameters};
use starroom_grading::GradingParameters;
use starroom_imageio::{decode_source, decode_source_preview, encode_jpeg_rgb8};
use starroom_pipeline::{
    RelativeColorParameters, RenderSettings, ToneCurveSet, WhiteBalanceMode, WhiteBalanceSample,
    WhiteBalanceSettings, render_source_export_to_srgb8, render_source_preview_to_srgb8,
    sample_source_color_band,
};
use starroom_render::RenderGraph;
use std::path::{Path, PathBuf};
use tauri::ipc::Response;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct EngineCapabilities {
    version: &'static str,
    native_tone_reference: bool,
    oklab_oklch: bool,
    color_mixer: bool,
    color_grading: bool,
    render_graph: bool,
    layer_mask_schema: bool,
    local_advisor: bool,
    portrait_reference: bool,
    healing_reference: bool,
    gpu_renderer: bool,
    raw_pipeline: bool,
}

#[tauri::command]
fn engine_status() -> &'static str {
    "V0_2_CORE_QUALITY"
}

#[tauri::command]
fn engine_capabilities() -> EngineCapabilities {
    EngineCapabilities {
        version: "0.2.0",
        native_tone_reference: true,
        oklab_oklch: true,
        color_mixer: true,
        color_grading: true,
        render_graph: RenderGraph::default().validate().is_ok(),
        layer_mask_schema: true,
        local_advisor: true,
        portrait_reference: true,
        healing_reference: true,
        gpu_renderer: false,
        raw_pipeline: true,
    }
}

#[tauri::command]
fn advise_image(stats: AnalysisStats) -> Vec<Suggestion> {
    advise(stats)
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct NativeEditSettings {
    exposure: f32,
    contrast: f32,
    highlights: f32,
    shadows: f32,
    whites: f32,
    blacks: f32,
    temperature: f32,
    tint: f32,
    vibrance: f32,
    saturation: f32,
    sharpness: f32,
    noise_reduction: f32,
    #[serde(default)]
    white_balance_mode: WhiteBalanceMode,
    #[serde(default)]
    white_balance_sample: Option<WhiteBalanceSample>,
    curve: Vec<CurvePoint>,
    #[serde(default)]
    curves: ToneCurveSet,
    #[serde(default)]
    color_mixer: ColorMixer,
    #[serde(default)]
    grading: GradingParameters,
    #[serde(default)]
    sharpen_settings: SharpenParameters,
    #[serde(default)]
    denoise_settings: DenoiseParameters,
    #[serde(default)]
    local_detail: LocalDetailParameters,
}

impl NativeEditSettings {
    fn validated(self) -> Result<RenderSettings, String> {
        let finite = [
            self.exposure,
            self.contrast,
            self.highlights,
            self.shadows,
            self.whites,
            self.blacks,
            self.temperature,
            self.tint,
            self.vibrance,
            self.saturation,
            self.sharpness,
            self.noise_reduction,
            self.color_mixer.band_width_degrees,
            self.grading.balance,
            self.grading.blending,
            self.grading.amount,
        ]
        .into_iter()
        .all(f32::is_finite)
            && self
                .curve
                .iter()
                .all(|point| point.x.is_finite() && point.y.is_finite());
        if !finite {
            return Err("native edit settings contain NaN or Inf".into());
        }
        if self.curve.len() > 32 {
            return Err("native tone curve accepts at most 32 points".into());
        }
        if !(30.0..=80.0).contains(&self.color_mixer.band_width_degrees)
            || self.color_mixer.bands.iter().any(|band| {
                ![band.hue_degrees, band.chroma, band.lightness]
                    .into_iter()
                    .all(f32::is_finite)
                    || !(-30.0..=30.0).contains(&band.hue_degrees)
                    || !(-1.0..=1.0).contains(&band.chroma)
                    || !(-1.0..=1.0).contains(&band.lightness)
            })
        {
            return Err("native color mixer settings are outside supported ranges".into());
        }
        let grading_wheels = [
            self.grading.shadows,
            self.grading.midtones,
            self.grading.highlights,
            self.grading.global,
        ];
        if grading_wheels.iter().any(|wheel| {
            ![wheel.hue_degrees, wheel.chroma, wheel.lightness]
                .into_iter()
                .all(f32::is_finite)
                || !(-360.0..=360.0).contains(&wheel.hue_degrees)
                || !(-1.0..=1.0).contains(&wheel.chroma)
                || !(-1.0..=1.0).contains(&wheel.lightness)
        }) || !(-1.0..=1.0).contains(&self.grading.balance)
            || !(0.0..=1.0).contains(&self.grading.blending)
            || !(0.0..=1.0).contains(&self.grading.amount)
        {
            return Err("native color grading settings are outside supported ranges".into());
        }
        let detail_values = [
            self.sharpen_settings.amount,
            self.sharpen_settings.radius,
            self.sharpen_settings.detail,
            self.sharpen_settings.masking,
            self.sharpen_settings.halo_protection,
            self.sharpen_settings.threshold,
            self.denoise_settings.luminance,
            self.denoise_settings.chroma,
            self.denoise_settings.radius,
            self.denoise_settings.detail_protection,
            self.denoise_settings.high_iso,
            self.local_detail.texture,
            self.local_detail.clarity,
            self.local_detail.dehaze,
        ];
        if !detail_values.into_iter().all(f32::is_finite)
            || !(0.0..=2.0).contains(&self.sharpen_settings.amount)
            || !(0.3..=4.0).contains(&self.sharpen_settings.radius)
            || !(0.0..=1.0).contains(&self.sharpen_settings.detail)
            || !(0.0..=1.0).contains(&self.sharpen_settings.masking)
            || !(0.0..=1.0).contains(&self.sharpen_settings.halo_protection)
            || !(0.0..=1.0).contains(&self.denoise_settings.luminance)
            || !(0.0..=1.0).contains(&self.denoise_settings.chroma)
            || !(0.6..=4.0).contains(&self.denoise_settings.radius)
            || !(0.0..=1.0).contains(&self.denoise_settings.detail_protection)
            || !(0.0..=1.0).contains(&self.denoise_settings.high_iso)
            || [
                self.local_detail.texture,
                self.local_detail.clarity,
                self.local_detail.dehaze,
            ]
            .into_iter()
            .any(|value| !(-1.0..=1.0).contains(&value))
        {
            return Err("native detail settings are outside supported ranges".into());
        }

        let unit = |value: f32| (value / 100.0).clamp(-1.0, 1.0);
        let mut curve = self.curve;
        curve.sort_by(|a, b| a.x.total_cmp(&b.x));
        if curve
            .iter()
            .any(|point| !(0.0..=1.0).contains(&point.x) || !(0.0..=1.0).contains(&point.y))
        {
            return Err("native tone curve points must stay inside 0..1".into());
        }
        Ok(RenderSettings {
            tone: ToneParameters {
                exposure_ev: self.exposure.clamp(-5.0, 5.0),
                contrast: unit(self.contrast),
                highlights: unit(self.highlights),
                shadows: unit(self.shadows),
                whites: unit(self.whites),
                blacks: unit(self.blacks),
            },
            relative_color: RelativeColorParameters {
                temperature: unit(self.temperature),
                tint: unit(self.tint),
                vibrance: unit(self.vibrance),
                saturation: unit(self.saturation),
            },
            white_balance: WhiteBalanceSettings {
                mode: self.white_balance_mode,
                sample: self.white_balance_sample,
            },
            curve,
            curves: self.curves,
            color_mixer: self.color_mixer,
            grading: self.grading,
            denoise: self.denoise_settings,
            local_detail: self.local_detail,
            sharpen: self.sharpen_settings,
            ..Default::default()
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct NativePreviewRequest {
    source_path: PathBuf,
    max_edge: u32,
    settings: NativeEditSettings,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct NativeExportRequest {
    source_path: PathBuf,
    output_path: PathBuf,
    quality: u8,
    settings: NativeEditSettings,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct NativeColorSampleRequest {
    source_path: PathBuf,
    x: f32,
    y: f32,
    settings: NativeEditSettings,
}

#[tauri::command]
fn native_sample_color(
    request: NativeColorSampleRequest,
) -> Result<Option<starroom_color::ColorBand>, String> {
    let settings = request.settings.validated()?;
    let decoded = decode_source_preview(&request.source_path, 1800)
        .map_err(|error| format!("native color sample decode failed: {error}"))?;
    sample_source_color_band(&decoded, &settings, request.x, request.y)
        .map_err(|error| format!("native color sample failed: {error}"))
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct NativeExportResult {
    output_path: PathBuf,
    width: u32,
    height: u32,
    input_profile: String,
    camera_profile_hash: Option<String>,
    working_space: &'static str,
}

fn profile_flag(source: starroom_color_management::InputProfileSource) -> u16 {
    match source {
        starroom_color_management::InputProfileSource::EmbeddedIcc => 1,
        starroom_color_management::InputProfileSource::AssumedSrgb => 0,
        starroom_color_management::InputProfileSource::RawCameraMatrix => 2,
        starroom_color_management::InputProfileSource::RawGenericProfile => 4,
    }
}

fn preview_frame(
    width: u32,
    height: u32,
    flags: u16,
    profile_id: &str,
    jpeg: Vec<u8>,
) -> Result<Vec<u8>, String> {
    let profile_len = u16::try_from(profile_id.len()).map_err(|_| "profile ID is too long")?;
    let payload_len = u32::try_from(jpeg.len()).map_err(|_| "native preview is too large")?;
    let mut frame = Vec::with_capacity(24 + profile_id.len() + jpeg.len());
    frame.extend_from_slice(b"SRP2");
    frame.extend_from_slice(&2_u16.to_le_bytes());
    frame.extend_from_slice(&flags.to_le_bytes());
    frame.extend_from_slice(&width.to_le_bytes());
    frame.extend_from_slice(&height.to_le_bytes());
    frame.extend_from_slice(&profile_len.to_le_bytes());
    frame.extend_from_slice(&0_u16.to_le_bytes());
    frame.extend_from_slice(&payload_len.to_le_bytes());
    frame.extend_from_slice(profile_id.as_bytes());
    frame.extend_from_slice(&jpeg);
    Ok(frame)
}

#[tauri::command]
fn native_preview(request: NativePreviewRequest) -> Result<Response, String> {
    let settings = request.settings.validated()?;
    let decoded = decode_source_preview(&request.source_path, request.max_edge.clamp(256, 4096))
        .map_err(|error| format!("native preview decode failed: {error}"))?;
    let rendered = render_source_preview_to_srgb8(&decoded, &settings)
        .map_err(|error| format!("native preview graph failed: {error}"))?;
    let flags = profile_flag(rendered.color.input);
    let profile_id = rendered.color.camera_profile_id.as_deref().unwrap_or("");
    let jpeg = encode_jpeg_rgb8(&rendered.data, rendered.width, rendered.height, 91, None)
        .map_err(|error| format!("native preview encode failed: {error}"))?;
    Ok(Response::new(preview_frame(
        rendered.width,
        rendered.height,
        flags,
        profile_id,
        jpeg,
    )?))
}

fn same_file(left: &Path, right: &Path) -> bool {
    if left == right {
        return true;
    }
    match (left.canonicalize(), right.canonicalize()) {
        (Ok(left), Ok(right)) => left == right,
        _ => false,
    }
}

#[tauri::command]
fn native_export_jpeg(request: NativeExportRequest) -> Result<NativeExportResult, String> {
    if same_file(&request.source_path, &request.output_path) {
        return Err("export destination must not overwrite the source image".into());
    }
    let settings = request.settings.validated()?;
    let decoded = decode_source(&request.source_path)
        .map_err(|error| format!("native export decode failed: {error}"))?;
    let rendered = render_source_export_to_srgb8(&decoded, &settings)
        .map_err(|error| format!("native export graph failed: {error}"))?;
    let input_profile = rendered
        .color
        .camera_profile_id
        .clone()
        .unwrap_or_else(|| match rendered.color.input {
            starroom_color_management::InputProfileSource::EmbeddedIcc => "embedded ICC".into(),
            starroom_color_management::InputProfileSource::AssumedSrgb => "assumed sRGB".into(),
            starroom_color_management::InputProfileSource::RawCameraMatrix => {
                "resolved RAW camera profile".into()
            }
            starroom_color_management::InputProfileSource::RawGenericProfile => {
                "Generic RAW Profile".into()
            }
        });
    let jpeg = encode_jpeg_rgb8(
        &rendered.data,
        rendered.width,
        rendered.height,
        request.quality.clamp(1, 100),
        None,
    )
    .map_err(|error| format!("native export encode failed: {error}"))?;
    std::fs::write(&request.output_path, jpeg)
        .map_err(|error| format!("native export write failed: {error}"))?;
    Ok(NativeExportResult {
        output_path: request.output_path,
        width: rendered.width,
        height: rendered.height,
        input_profile,
        camera_profile_hash: rendered.color.camera_profile_hash,
        working_space: rendered.color.working_space,
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            engine_status,
            engine_capabilities,
            advise_image,
            native_preview,
            native_export_jpeg,
            native_sample_color
        ])
        .run(tauri::generate_context!())
        .expect("error while running Starroom");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings() -> NativeEditSettings {
        NativeEditSettings {
            exposure: 0.5,
            contrast: 10.0,
            highlights: -20.0,
            shadows: 25.0,
            whites: 0.0,
            blacks: 0.0,
            temperature: 30.0,
            tint: -10.0,
            vibrance: 0.0,
            saturation: 0.0,
            sharpness: 0.0,
            noise_reduction: 0.0,
            white_balance_mode: WhiteBalanceMode::SourceDefault,
            white_balance_sample: None,
            curve: vec![CurvePoint { x: 0.0, y: 0.0 }, CurvePoint { x: 1.0, y: 1.0 }],
            curves: ToneCurveSet::default(),
            color_mixer: ColorMixer::default(),
            grading: GradingParameters::default(),
            sharpen_settings: SharpenParameters {
                amount: 0.0,
                ..Default::default()
            },
            denoise_settings: DenoiseParameters::default(),
            local_detail: LocalDetailParameters::default(),
        }
    }

    #[test]
    fn ui_contract_maps_exposure_wb_tone_and_curve_into_shared_settings() {
        let settings = settings().validated().expect("valid settings");
        assert_eq!(settings.tone.exposure_ev, 0.5);
        assert_eq!(settings.tone.contrast, 0.1);
        assert_eq!(settings.tone.highlights, -0.2);
        assert_eq!(settings.tone.shadows, 0.25);
        assert_eq!(settings.relative_color.temperature, 0.3);
        assert_eq!(settings.relative_color.tint, -0.1);
        assert_eq!(settings.curve.len(), 2);
        assert_eq!(settings.color_mixer, ColorMixer::default());
    }

    #[test]
    fn binary_preview_contract_has_fixed_header_and_payload_length() {
        let profile = "dng-forward-matrix:test:camera";
        let frame = preview_frame(640, 480, 2, profile, vec![0xff, 0xd8, 0xff]).expect("frame");
        assert_eq!(&frame[0..4], b"SRP2");
        assert_eq!(u32::from_le_bytes(frame[8..12].try_into().unwrap()), 640);
        assert_eq!(u32::from_le_bytes(frame[12..16].try_into().unwrap()), 480);
        assert_eq!(
            u16::from_le_bytes(frame[16..18].try_into().unwrap()) as usize,
            profile.len()
        );
        assert_eq!(u32::from_le_bytes(frame[20..24].try_into().unwrap()), 3);
        assert_eq!(&frame[24..24 + profile.len()], profile.as_bytes());
        assert_eq!(&frame[24 + profile.len()..], &[0xff, 0xd8, 0xff]);
    }

    #[test]
    fn non_finite_settings_are_rejected_before_the_graph() {
        let mut settings = settings();
        settings.exposure = f32::NAN;
        assert!(settings.validated().is_err());
    }
}
