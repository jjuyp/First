use serde::{Deserialize, Serialize};
use starroom_advisor::{AnalysisStats, Suggestion, advise};
use starroom_color::{ColorMixer, CurvePoint, ToneParameters};
use starroom_detail::{DenoiseParameters, LocalDetailParameters, SharpenParameters};
use starroom_geometry::GeometryParameters;
use starroom_grading::GradingParameters;
use starroom_imageio::{
    DecodedSourceImage, decode_source, decode_source_preview, encode_jpeg_rgb8,
};
use starroom_optics::{LensProfileResolution, OpticsSettings};
use starroom_pipeline::{
    NativeAdjustmentLayer, PortraitMaskRaster, RelativeColorParameters, RenderSettings,
    ToneCurveSet, WhiteBalanceMode, WhiteBalanceSample, WhiteBalanceSettings,
    render_source_export_to_srgb8, render_source_preview_to_srgb8,
    render_source_preview_with_gpu_to_srgb8, resolve_source_lens_profile, sample_source_color_band,
};
use starroom_portrait::{
    DetectedFace, PortraitError, PortraitModelRegistry, PortraitOnnxProvider, PortraitParseResult,
    PortraitRegion,
};
use starroom_project::{MaskDefinition, MaskTree, PortraitMaskRegion};
use starroom_render::{
    RenderGraph,
    gpu::{GpuBackendKind, GpuRenderer, GpuStatus, probe_gpu_status},
    scheduler::{Completion, DEFAULT_TILE_EDGE, RenderScheduler, SchedulerStatus, Viewport},
};
use std::path::{Path, PathBuf};
use std::{collections::BTreeMap, sync::Mutex};
use tauri::State;
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
        gpu_renderer: true,
        raw_pipeline: true,
    }
}

/// UI-visible M12 backend state. This intentionally reports the fallback reason instead of
/// silently treating unavailable DX12/device resources as a browser-rendering failure.
#[tauri::command]
fn gpu_preview_status(prefer_gpu: Option<bool>) -> GpuStatus {
    probe_gpu_status(prefer_gpu.unwrap_or(true))
}

#[tauri::command]
fn advise_image(stats: AnalysisStats) -> Vec<Suggestion> {
    advise(stats)
}

#[derive(Debug, Clone, Deserialize, Serialize)]
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
    #[serde(default)]
    optics: OpticsSettings,
    #[serde(default)]
    geometry: GeometryParameters,
    #[serde(default)]
    layers: Vec<NativeAdjustmentLayer>,
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
        if self.layers.len() > 64 {
            return Err("native layer stack accepts at most 64 layers".into());
        }
        let mut layer_ids = std::collections::BTreeSet::new();
        if self
            .layers
            .iter()
            .any(|layer| layer.id.trim().is_empty() || !layer_ids.insert(&layer.id))
        {
            return Err("native layer identifiers must be unique and non-empty".into());
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
        if self
            .optics
            .manual_identity
            .as_ref()
            .is_some_and(|identity| {
                ![identity.focal_length_mm, identity.aperture]
                    .into_iter()
                    .all(f32::is_finite)
                    || identity
                        .focus_distance_m
                        .is_some_and(|distance| !distance.is_finite() || distance <= 0.0)
            })
        {
            return Err("native manual lens metadata is invalid".into());
        }
        let geometry_values = [
            self.geometry.rotation_degrees,
            self.geometry.vertical_keystone,
            self.geometry.horizontal_keystone,
            self.geometry.scale,
            self.geometry.offset_x,
            self.geometry.offset_y,
            self.geometry.crop.left,
            self.geometry.crop.top,
            self.geometry.crop.right,
            self.geometry.crop.bottom,
            self.geometry.crop_aspect_width,
            self.geometry.crop_aspect_height,
        ];
        let four_point_finite = self.geometry.four_point.is_none_or(|points| {
            [
                points.top_left.x,
                points.top_left.y,
                points.top_right.x,
                points.top_right.y,
                points.bottom_right.x,
                points.bottom_right.y,
                points.bottom_left.x,
                points.bottom_left.y,
            ]
            .into_iter()
            .all(f32::is_finite)
        });
        if !geometry_values.into_iter().all(f32::is_finite)
            || !four_point_finite
            || !(-180.0..=180.0).contains(&self.geometry.rotation_degrees)
            || !(-1.5..=1.5).contains(&self.geometry.vertical_keystone)
            || !(-1.5..=1.5).contains(&self.geometry.horizontal_keystone)
            || !(0.05..=20.0).contains(&self.geometry.scale)
            || self.geometry.crop.left < 0.0
            || self.geometry.crop.top < 0.0
            || self.geometry.crop.right > 1.0
            || self.geometry.crop.bottom > 1.0
            || self.geometry.crop.right <= self.geometry.crop.left
            || self.geometry.crop.bottom <= self.geometry.crop.top
            || ((self.geometry.crop_aspect_width < 0.0 || self.geometry.crop_aspect_height < 0.0)
                && !(self.geometry.crop_aspect_width == -1.0
                    && self.geometry.crop_aspect_height == -1.0))
        {
            return Err("native geometry settings are outside supported ranges".into());
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
            optics: self.optics,
            geometry: self.geometry,
            layers: self.layers,
            ..Default::default()
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct NativePreviewRequest {
    source_path: PathBuf,
    max_edge: u32,
    #[serde(default = "default_prefer_gpu")]
    prefer_gpu: bool,
    settings: NativeEditSettings,
}

/// Process-wide M13 scheduler state. It holds only derived preview/cache bytes and request
/// identities; the immutable source image remains on disk and full export never reads this cache.
struct NativePreviewScheduler(Mutex<RenderScheduler>);

/// Process-local M16 model/session and soft-mask cache. It never crosses the Tauri boundary:
/// IPC transports face geometry and a compact cache reference, while Preview/Export resolve the
/// source-space R16Float-compatible mask in the Native shared graph.
struct NativePortraitRuntime(Mutex<PortraitRuntimeState>);

#[derive(Default)]
struct PortraitRuntimeState {
    provider: Option<PortraitOnnxProvider>,
    parsed: BTreeMap<String, PortraitParseResult>,
}

impl Default for NativePortraitRuntime {
    fn default() -> Self {
        Self(Mutex::new(PortraitRuntimeState::default()))
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct PortraitFailure {
    code: &'static str,
    message: String,
}

impl From<PortraitError> for PortraitFailure {
    fn from(value: PortraitError) -> Self {
        let code = match value {
            PortraitError::DetectorModelMissing { .. } => "detectorModelMissing",
            PortraitError::ParserModelMissing { .. } => "parserModelMissing",
            PortraitError::ModelHashMismatch { .. } => "modelHashMismatch",
            PortraitError::RuntimeUnavailable(_) => "runtimeUnavailable",
            PortraitError::DetectorInitializationFailed(_) => "detectorInitializationFailed",
            PortraitError::ParserInitializationFailed(_) => "parserInitializationFailed",
            PortraitError::DetectionFailed(_) => "detectionFailed",
            PortraitError::ParsingFailed(_) => "parsingFailed",
            PortraitError::InvalidDetectionOutput(_) => "invalidDetectionOutput",
            PortraitError::InvalidParsingOutput(_) => "invalidParsingOutput",
            PortraitError::NoFaceDetected => "noFaceDetected",
            PortraitError::InvalidTransform(_) => "invalidTransform",
            PortraitError::UnsupportedExecutionProvider(_) => "unsupportedExecutionProvider",
        };
        Self {
            code,
            message: value.to_string(),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PortraitFaceResponse {
    face: DetectedFace,
    cache_key: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PortraitDetectionResponse {
    status: &'static str,
    faces: Vec<PortraitFaceResponse>,
    detector_model_id: String,
    detector_model_version: String,
    detector_model_hash: String,
    parser_model_id: String,
    parser_model_version: String,
    parser_model_hash: String,
    execution_provider: starroom_portrait::ExecutionProvider,
    error: Option<PortraitFailure>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PortraitDetectRequest {
    source_path: PathBuf,
    #[serde(default = "default_face_crop_scale")]
    face_crop_scale: f32,
}

const fn default_face_crop_scale() -> f32 {
    1.4
}

impl Default for NativePreviewScheduler {
    fn default() -> Self {
        Self(Mutex::new(RenderScheduler::default()))
    }
}

const fn default_prefer_gpu() -> bool {
    true
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

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct NativeOpticsStatusRequest {
    source_path: PathBuf,
    settings: NativeEditSettings,
}

#[tauri::command]
fn native_optics_status(
    request: NativeOpticsStatusRequest,
) -> Result<LensProfileResolution, String> {
    let settings = request.settings.validated()?;
    let decoded = decode_source_preview(&request.source_path, 512)
        .map_err(|error| format!("native optics metadata decode failed: {error}"))?;
    resolve_source_lens_profile(&decoded, &settings.optics)
        .map_err(|error| format!("native Lensfun resolution failed: {error}"))
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

fn preview_source_identity(path: &Path) -> Result<String, String> {
    let metadata = std::fs::metadata(path)
        .map_err(|error| format!("native preview metadata failed: {error}"))?;
    let modified = metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    Ok(format!("{}:{}:{modified}", path.display(), metadata.len()))
}

fn source_dimensions(decoded: &DecodedSourceImage) -> (u32, u32) {
    match decoded {
        DecodedSourceImage::Rendered(image) => (image.width, image.height),
        DecodedSourceImage::Raw(image) => (image.width, image.height),
    }
}

fn local_portrait_models() -> PortraitModelRegistry {
    let root = std::env::var_os("STARROOM_LOCAL_MODELS")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("models").join("local"));
    PortraitModelRegistry::local_default(root)
}

fn source_rgba_for_portrait(path: &Path) -> Result<(u32, u32, Vec<u8>, String), PortraitError> {
    // M16 identity is source-image space, never the M13 preview-pyramid size.
    let decoded = decode_source(path)
        .map_err(|error| PortraitError::DetectionFailed(format!("source decode: {error}")))?;
    let rendered =
        render_source_export_to_srgb8(&decoded, &RenderSettings::default()).map_err(|error| {
            PortraitError::DetectionFailed(format!("source display transform: {error}"))
        })?;
    let mut rgba = Vec::with_capacity(rendered.width as usize * rendered.height as usize * 4);
    for rgb in rendered.data.chunks_exact(3) {
        rgba.extend_from_slice(&[rgb[0], rgb[1], rgb[2], 255]);
    }
    let identity = preview_source_identity(path).map_err(PortraitError::DetectionFailed)?;
    Ok((rendered.width, rendered.height, rgba, identity))
}

fn project_region(region: PortraitRegion) -> PortraitMaskRegion {
    match region {
        PortraitRegion::Face => PortraitMaskRegion::Face,
        PortraitRegion::Skin => PortraitMaskRegion::Skin,
        PortraitRegion::Eyes => PortraitMaskRegion::Eyes,
        PortraitRegion::LeftEye => PortraitMaskRegion::LeftEye,
        PortraitRegion::RightEye => PortraitMaskRegion::RightEye,
        PortraitRegion::Brows => PortraitMaskRegion::Brows,
        PortraitRegion::LeftBrow => PortraitMaskRegion::LeftBrow,
        PortraitRegion::RightBrow => PortraitMaskRegion::RightBrow,
        PortraitRegion::Lips => PortraitMaskRegion::Lips,
        PortraitRegion::Mouth => PortraitMaskRegion::Mouth,
        PortraitRegion::Hair => PortraitMaskRegion::Hair,
    }
}

fn collect_portrait_mask_references(
    tree: &MaskTree,
    values: &mut Vec<(String, String, PortraitMaskRegion)>,
) {
    match tree {
        MaskTree::Leaf(MaskDefinition::PortraitSemantic {
            face_id,
            region,
            cache_key,
            ..
        }) => values.push((cache_key.clone(), face_id.clone(), *region)),
        MaskTree::Leaf(_) => {}
        MaskTree::Composite(composite) => {
            for child in &composite.children {
                collect_portrait_mask_references(child, values);
            }
        }
    }
}

fn attach_portrait_masks(
    settings: &mut RenderSettings,
    runtime: &NativePortraitRuntime,
) -> Result<(), String> {
    let mut references = Vec::new();
    for layer in &settings.layers {
        collect_portrait_mask_references(&layer.mask, &mut references);
    }
    if references.is_empty() {
        return Ok(());
    }
    let state = runtime
        .0
        .lock()
        .map_err(|_| "portrait runtime lock was poisoned".to_owned())?;
    for (cache_key, face_id, region) in references {
        let parse = state
            .parsed
            .get(&cache_key)
            .ok_or_else(|| format!("portrait semantic cache is unavailable: {cache_key}"))?;
        let source_region = match region {
            PortraitMaskRegion::Face => PortraitRegion::Face,
            PortraitMaskRegion::Skin => PortraitRegion::Skin,
            PortraitMaskRegion::Eyes => PortraitRegion::Eyes,
            PortraitMaskRegion::LeftEye => PortraitRegion::LeftEye,
            PortraitMaskRegion::RightEye => PortraitRegion::RightEye,
            PortraitMaskRegion::Brows => PortraitRegion::Brows,
            PortraitMaskRegion::LeftBrow => PortraitRegion::LeftBrow,
            PortraitMaskRegion::RightBrow => PortraitRegion::RightBrow,
            PortraitMaskRegion::Lips => PortraitRegion::Lips,
            PortraitMaskRegion::Mouth => PortraitRegion::Mouth,
            PortraitMaskRegion::Hair => PortraitRegion::Hair,
        };
        let mask = parse
            .regions
            .get(&source_region)
            .ok_or_else(|| format!("portrait semantic region is unavailable: {cache_key}"))?;
        settings.portrait_masks.push(PortraitMaskRaster {
            cache_key,
            face_id,
            region,
            width: mask.width,
            height: mask.height,
            values: mask.values.clone(),
        });
    }
    Ok(())
}

#[tauri::command]
fn portrait_detect(
    runtime: State<'_, NativePortraitRuntime>,
    request: PortraitDetectRequest,
) -> PortraitDetectionResponse {
    let registry = local_portrait_models();
    let response_shell = |status, error: Option<PortraitError>| PortraitDetectionResponse {
        status,
        faces: Vec::new(),
        detector_model_id: registry.detector.id.clone(),
        detector_model_version: registry.detector.version.clone(),
        detector_model_hash: registry.detector.sha256.clone(),
        parser_model_id: registry.parser.id.clone(),
        parser_model_version: registry.parser.version.clone(),
        parser_model_hash: registry.parser.sha256.clone(),
        execution_provider: registry.execution_provider,
        error: error.map(Into::into),
    };
    if !request.face_crop_scale.is_finite() || !(1.0..=3.0).contains(&request.face_crop_scale) {
        return response_shell(
            "failed",
            Some(PortraitError::InvalidTransform(
                "face crop scale must be 1.0..3.0".into(),
            )),
        );
    }
    let (width, height, rgba, source_identity) =
        match source_rgba_for_portrait(&request.source_path) {
            Ok(value) => value,
            Err(error) => return response_shell("failed", Some(error)),
        };
    let mut state = match runtime.0.lock() {
        Ok(state) => state,
        Err(_) => {
            return response_shell(
                "failed",
                Some(PortraitError::RuntimeUnavailable(
                    "portrait runtime lock was poisoned".into(),
                )),
            );
        }
    };
    if state.provider.is_none() {
        match PortraitOnnxProvider::initialize(registry.clone()) {
            Ok(provider) => state.provider = Some(provider),
            Err(error) => return response_shell("unavailable", Some(error)),
        }
    }
    let provider = state.provider.as_mut().expect("initialized above");
    let faces = match provider.detect(
        width,
        height,
        &rgba,
        request.face_crop_scale,
        &source_identity,
    ) {
        Ok(value) => value,
        Err(PortraitError::NoFaceDetected) => {
            return response_shell("noFace", Some(PortraitError::NoFaceDetected));
        }
        Err(error) => return response_shell("failed", Some(error)),
    };
    let mut response_faces = Vec::with_capacity(faces.len());
    for face in faces {
        let parsed = match provider.parse(width, height, &rgba, &face, &source_identity) {
            Ok(value) => value,
            Err(error) => return response_shell("failed", Some(error)),
        };
        let cache_key = format!(
            "{}:{}",
            parsed.cache_key.face_id, parsed.cache_key.crop_transform_hash
        );
        state.parsed.insert(cache_key.clone(), parsed);
        response_faces.push(PortraitFaceResponse { face, cache_key });
    }
    PortraitDetectionResponse {
        status: "ready",
        faces: response_faces,
        detector_model_id: registry.detector.id,
        detector_model_version: registry.detector.version,
        detector_model_hash: registry.detector.sha256,
        parser_model_id: registry.parser.id,
        parser_model_version: registry.parser.version,
        parser_model_hash: registry.parser.sha256,
        execution_provider: provider.execution_provider,
        error: None,
    }
}

/// Explicit M13 diagnostics for progressive preview scheduling. The UI can expose cache and
/// stale-frame statistics without receiving pixels through JSON.
#[tauri::command]
fn native_preview_scheduler_status(
    scheduler: State<'_, NativePreviewScheduler>,
) -> Result<SchedulerStatus, String> {
    scheduler
        .0
        .lock()
        .map_err(|_| "native preview scheduler lock was poisoned".to_owned())
        .map(|scheduler| scheduler.status())
}

#[tauri::command]
fn native_preview(
    scheduler: State<'_, NativePreviewScheduler>,
    portrait_runtime: State<'_, NativePortraitRuntime>,
    request: NativePreviewRequest,
) -> Result<Response, String> {
    let graph_identity = serde_json::to_string(&request.settings)
        .map_err(|error| format!("native preview graph identity serialization failed: {error}"))?;
    let mut settings = request.settings.validated()?;
    attach_portrait_masks(&mut settings, &portrait_runtime)?;
    let requested_edge = request.max_edge.clamp(256, 4096);
    let level = starroom_render::scheduler::PreviewLevel::for_requested_edge(requested_edge);
    let decoded = decode_source_preview(&request.source_path, level.max_edge())
        .map_err(|error| format!("native preview decode failed: {error}"))?;
    let (source_width, source_height) = source_dimensions(&decoded);
    let source_identity = preview_source_identity(&request.source_path)?;
    let job = scheduler
        .0
        .lock()
        .map_err(|_| "native preview scheduler lock was poisoned".to_owned())?
        .schedule_preview(
            source_identity,
            graph_identity,
            source_width,
            source_height,
            requested_edge,
            Viewport::full(source_width, source_height),
            DEFAULT_TILE_EDGE,
            RenderGraph::default().maximum_halo(),
        );
    let frame_tile = job.full_frame_tile();
    if let Some(frame) = scheduler
        .0
        .lock()
        .map_err(|_| "native preview scheduler lock was poisoned".to_owned())?
        .cached_tile(&frame_tile.identity)
    {
        return Ok(Response::new(frame));
    }
    let (rendered, backend_flags) = if request.prefer_gpu {
        match GpuRenderer::try_new() {
            Ok(renderer) => {
                match render_source_preview_with_gpu_to_srgb8(&decoded, &settings, &renderer) {
                    Ok(rendered) => {
                        let flag = match renderer.status().backend {
                            GpuBackendKind::Dx12 | GpuBackendKind::Other => 0x0008,
                            GpuBackendKind::CpuFallback => 0x0010,
                        };
                        (rendered, flag)
                    }
                    Err(error) => {
                        let rendered = render_source_preview_to_srgb8(&decoded, &settings)
                        .map_err(|fallback| format!("native GPU preview failed ({error}); CPU reference fallback also failed: {fallback}"))?;
                        (rendered, 0x0010)
                    }
                }
            }
            Err(_) => {
                let rendered = render_source_preview_to_srgb8(&decoded, &settings)
                    .map_err(|error| format!("native CPU preview graph failed after GPU initialization fallback: {error}"))?;
                (rendered, 0x0010)
            }
        }
    } else {
        let rendered = render_source_preview_to_srgb8(&decoded, &settings)
            .map_err(|error| format!("native CPU preview graph failed: {error}"))?;
        (rendered, 0x0010)
    };
    let flags = profile_flag(rendered.color.input) | backend_flags;
    let profile_id = rendered.color.camera_profile_id.as_deref().unwrap_or("");
    let jpeg = encode_jpeg_rgb8(&rendered.data, rendered.width, rendered.height, 91, None)
        .map_err(|error| format!("native preview encode failed: {error}"))?;
    let frame = preview_frame(rendered.width, rendered.height, flags, profile_id, jpeg)?;
    let estimated_vram_bytes = rendered.width as usize * rendered.height as usize * 8;
    let completion = scheduler
        .0
        .lock()
        .map_err(|_| "native preview scheduler lock was poisoned".to_owned())?
        .complete_tile(&frame_tile, frame.clone(), estimated_vram_bytes);
    if completion == Completion::Stale {
        return Err("native preview was superseded by a newer render request".into());
    }
    Ok(Response::new(frame))
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
fn native_export_jpeg(
    portrait_runtime: State<'_, NativePortraitRuntime>,
    request: NativeExportRequest,
) -> Result<NativeExportResult, String> {
    if same_file(&request.source_path, &request.output_path) {
        return Err("export destination must not overwrite the source image".into());
    }
    let mut settings = request.settings.validated()?;
    attach_portrait_masks(&mut settings, &portrait_runtime)?;
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
        .manage(NativePreviewScheduler::default())
        .manage(NativePortraitRuntime::default())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            engine_status,
            engine_capabilities,
            gpu_preview_status,
            advise_image,
            native_preview,
            native_preview_scheduler_status,
            native_export_jpeg,
            portrait_detect,
            native_sample_color,
            native_optics_status
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
            optics: OpticsSettings::default(),
            geometry: GeometryParameters::default(),
            layers: Vec::new(),
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

    #[test]
    fn layer_contract_rejects_duplicate_ids_before_native_rendering() {
        let mut settings = settings();
        settings.layers = vec![
            NativeAdjustmentLayer {
                id: "same".into(),
                name: "First".into(),
                enabled: true,
                opacity: 1.0,
                blend_mode: Default::default(),
                mask: starroom_project::MaskDefinition::None.into(),
                adjustments: Default::default(),
            },
            NativeAdjustmentLayer {
                id: "same".into(),
                name: "Second".into(),
                enabled: true,
                opacity: 1.0,
                blend_mode: Default::default(),
                mask: starroom_project::MaskDefinition::None.into(),
                adjustments: Default::default(),
            },
        ];
        assert!(settings.validated().is_err());
    }
}
