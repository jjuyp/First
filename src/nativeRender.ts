import { convertFileSrc, invoke, isTauri } from '@tauri-apps/api/core'
import { open, save } from '@tauri-apps/plugin-dialog'
import type { Adjustments } from './editorState'
import type { RadialMask, ToneCurvePoint } from './imagePipeline'

export type RenderBackend = 'native' | 'browserFallback'
export type NativeWhiteBalanceMode = 'sourceDefault' | 'asShot' | 'camera' | 'auto' | 'neutralPicker' | 'relative'
export interface NativeWhiteBalanceSample { x: number; y: number; width: number; height: number }
export interface NativeToneCurves { master: ToneCurvePoint[]; red: ToneCurvePoint[]; green: ToneCurvePoint[]; blue: ToneCurvePoint[] }
export type NativeColorBand = 'red' | 'orange' | 'yellow' | 'green' | 'cyan' | 'blue' | 'purple' | 'magenta'
export interface NativeBandAdjustment { hueDegrees: number; chroma: number; lightness: number }
export interface NativeColorMixer { bands: NativeBandAdjustment[]; hueLock: boolean; bandWidthDegrees: number }
export interface NativeColorWheel { hueDegrees: number; chroma: number; lightness: number }
export interface NativeGrading { shadows: NativeColorWheel; midtones: NativeColorWheel; highlights: NativeColorWheel; global: NativeColorWheel; balance: number; blending: number; amount: number }

export interface NativeEditSettings {
  exposure: number
  contrast: number
  highlights: number
  shadows: number
  whites: number
  blacks: number
  temperature: number
  tint: number
  vibrance: number
  saturation: number
  sharpness: number
  noiseReduction: number
  whiteBalanceMode: NativeWhiteBalanceMode
  whiteBalanceSample: NativeWhiteBalanceSample | null
  curve: Array<{ x: number; y: number }>
  curves: { master: Array<{ x: number; y: number }>; red: Array<{ x: number; y: number }>; green: Array<{ x: number; y: number }>; blue: Array<{ x: number; y: number }> }
  colorMixer: NativeColorMixer
  grading: NativeGrading
  sharpenSettings: { amount: number; radius: number; detail: number; masking: number; haloProtection: number; threshold: number }
  denoiseSettings: { luminance: number; chroma: number; radius: number; detailProtection: number; highIso: number }
  localDetail: { texture: number; clarity: number; dehaze: number }
}

export interface NativePreviewResult {
  width: number
  height: number
  inputProfile: 'embedded ICC' | 'assumed sRGB' | 'resolved RAW camera profile' | 'Generic RAW Profile'
  cameraProfileId: string | null
  jpeg: Uint8Array
}

export interface NativeExportResult {
  outputPath: string
  width: number
  height: number
  inputProfile: string
  workingSpace: string
  cameraProfileHash: string | null
}

const HEADER_BYTES = 24

export const nativeRuntimeAvailable = () => isTauri()

export function assertNativeSupported(adjustments: Adjustments, mask: RadialMask) {
  const unsupported: string[] = []
  if (adjustments.maskExposure !== 0 || mask.x !== .5 || mask.y !== .5 || mask.width !== .42
    || mask.height !== .42 || mask.rotation !== 0) unsupported.push('Masks')
  if (adjustments.vignette !== 0 || adjustments.lensBrightness !== 0) unsupported.push('Optics')
  if (adjustments.rotation !== 0 || adjustments.flipHorizontal !== 0 || adjustments.flipVertical !== 0) unsupported.push('Geometry')
  if (unsupported.length) {
    throw new Error(`Native M1C does not support ${unsupported.join(', ')} yet; Browser fallback was not used.`)
  }
}

export function toNativeSettings(adjustments: Adjustments, curve: ToneCurvePoint[],
  whiteBalanceMode: NativeWhiteBalanceMode = 'sourceDefault', whiteBalanceSample: NativeWhiteBalanceSample | null = null,
  toneCurves: NativeToneCurves = { master: curve, red: [], green: [], blue: [] }): NativeEditSettings {
  const bands: NativeColorBand[] = ['red', 'orange', 'yellow', 'green', 'cyan', 'blue', 'purple', 'magenta']
  const title = (band: string) => `${band[0].toUpperCase()}${band.slice(1)}`
  const wheel = (zone: 'Global' | 'Shadows' | 'Midtones' | 'Highlights'): NativeColorWheel => ({
    hueDegrees: adjustments[`grade${zone}Hue`],
    chroma: adjustments[`grade${zone}Chroma`] / 100,
    lightness: adjustments[`grade${zone}Lightness`] / 100,
  })
  return {
    exposure: adjustments.exposure,
    contrast: adjustments.contrast,
    highlights: adjustments.highlights,
    shadows: adjustments.shadows,
    whites: adjustments.whites,
    blacks: adjustments.blacks,
    temperature: adjustments.temperature,
    tint: adjustments.tint,
    vibrance: adjustments.vibrance,
    saturation: adjustments.saturation,
    sharpness: adjustments.sharpness,
    noiseReduction: adjustments.noiseReduction,
    whiteBalanceMode,
    whiteBalanceSample,
    curve: [...curve].sort((left, right) => left.x - right.x).map(({ x, y }) => ({ x, y })),
    curves: Object.fromEntries(Object.entries(toneCurves).map(([channel, points]) => [channel, [...points].sort((left, right) => left.x - right.x).map(({ x, y }) => ({ x, y }))])) as NativeEditSettings['curves'],
    colorMixer: {
      bands: bands.map((band) => ({
        hueDegrees: adjustments[`mixer${title(band)}Hue` as keyof Adjustments],
        chroma: adjustments[`mixer${title(band)}Chroma` as keyof Adjustments] / 100,
        lightness: adjustments[`mixer${title(band)}Lightness` as keyof Adjustments] / 100,
      })),
      hueLock: adjustments.mixerHueLock !== 0,
      bandWidthDegrees: 52,
    },
    grading: {
      global: wheel('Global'), shadows: wheel('Shadows'), midtones: wheel('Midtones'), highlights: wheel('Highlights'),
      balance: adjustments.gradeBalance / 100, blending: adjustments.gradeBlending / 100, amount: adjustments.gradeAmount / 100,
    },
    sharpenSettings: {
      amount: Math.max(0, adjustments.sharpness / 50), radius: adjustments.sharpenRadius,
      detail: adjustments.sharpenDetail / 100, masking: adjustments.sharpenMasking / 100,
      haloProtection: adjustments.sharpenHaloProtection / 100, threshold: .002,
    },
    denoiseSettings: {
      luminance: Math.max(adjustments.noiseReduction, adjustments.denoiseLuminance) / 100,
      chroma: Math.max(adjustments.noiseReduction, adjustments.denoiseChroma) / 100,
      radius: adjustments.denoiseRadius, detailProtection: adjustments.denoiseDetailProtection / 100,
      highIso: adjustments.denoiseHighIso / 100,
    },
    localDetail: { texture: adjustments.texture / 100, clarity: adjustments.clarity / 100, dehaze: adjustments.dehaze / 100 },
  }
}

export function parseNativePreviewFrame(value: ArrayBuffer | Uint8Array): NativePreviewResult {
  const bytes = value instanceof Uint8Array ? value : new Uint8Array(value)
  if (bytes.byteLength < HEADER_BYTES || String.fromCharCode(...bytes.subarray(0, 4)) !== 'SRP2') {
    throw new Error('Native preview returned an invalid binary frame.')
  }
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength)
  const version = view.getUint16(4, true)
  if (version !== 2) throw new Error(`Unsupported native preview contract version ${version}.`)
  const flags = view.getUint16(6, true)
  const width = view.getUint32(8, true)
  const height = view.getUint32(12, true)
  const profileLength = view.getUint16(16, true)
  const payloadLength = view.getUint32(20, true)
  if (!width || !height || HEADER_BYTES + profileLength + payloadLength !== bytes.byteLength) {
    throw new Error('Native preview returned inconsistent dimensions or payload length.')
  }
  const profileStart = HEADER_BYTES
  const payloadStart = profileStart + profileLength
  const cameraProfileId = profileLength
    ? new TextDecoder('utf-8', { fatal: true }).decode(bytes.subarray(profileStart, payloadStart))
    : null
  return {
    width,
    height,
    inputProfile: flags & 4 ? 'Generic RAW Profile'
      : flags & 2 ? 'resolved RAW camera profile'
        : flags & 1 ? 'embedded ICC' : 'assumed sRGB',
    cameraProfileId,
    jpeg: bytes.slice(payloadStart),
  }
}

export async function chooseNativePhotoPaths(): Promise<string[]> {
  const selected = await open({
    title: 'Add photos to Starroom',
    multiple: true,
    directory: false,
    filters: [{
      name: 'Photos and camera RAW',
      extensions: ['jpg', 'jpeg', 'png', 'tif', 'tiff', 'nef', 'arw', 'cr2', 'cr3', 'dng', 'raf'],
    }],
  })
  return selected ? (Array.isArray(selected) ? selected : [selected]) : []
}

export const nativeThumbnailUrl = (path: string) => convertFileSrc(path)

export async function renderNativePreview(
  sourcePath: string,
  adjustments: Adjustments,
  curve: ToneCurvePoint[],
  mask: RadialMask,
  whiteBalanceMode: NativeWhiteBalanceMode = 'sourceDefault',
  whiteBalanceSample: NativeWhiteBalanceSample | null = null,
  toneCurves: NativeToneCurves = { master: curve, red: [], green: [], blue: [] },
  maxEdge = 1800,
) {
  assertNativeSupported(adjustments, mask)
  const frame = await invoke<ArrayBuffer | Uint8Array>('native_preview', {
    request: { sourcePath, maxEdge, settings: toNativeSettings(adjustments, curve, whiteBalanceMode, whiteBalanceSample, toneCurves) },
  })
  return parseNativePreviewFrame(frame)
}

export async function sampleNativeColor(sourcePath: string, x: number, y: number, adjustments: Adjustments,
  curve: ToneCurvePoint[], whiteBalanceMode: NativeWhiteBalanceMode, whiteBalanceSample: NativeWhiteBalanceSample | null,
  toneCurves: NativeToneCurves): Promise<NativeColorBand | null> {
  return invoke<NativeColorBand | null>('native_sample_color', {
    request: { sourcePath, x, y, settings: toNativeSettings(adjustments, curve, whiteBalanceMode, whiteBalanceSample, toneCurves) },
  })
}

export async function chooseNativeExportPath(sourceName: string) {
  const base = sourceName.replace(/\.[^.]+$/, '')
  return save({
    title: 'Export Starroom JPEG',
    defaultPath: `${base}-starroom.jpg`,
    filters: [{ name: 'JPEG image', extensions: ['jpg', 'jpeg'] }],
  })
}

export async function exportNativeJpeg(
  sourcePath: string,
  outputPath: string,
  adjustments: Adjustments,
  curve: ToneCurvePoint[],
  mask: RadialMask,
  whiteBalanceMode: NativeWhiteBalanceMode = 'sourceDefault',
  whiteBalanceSample: NativeWhiteBalanceSample | null = null,
  toneCurves: NativeToneCurves = { master: curve, red: [], green: [], blue: [] },
) {
  assertNativeSupported(adjustments, mask)
  return invoke<NativeExportResult>('native_export_jpeg', {
    request: { sourcePath, outputPath, quality: 94, settings: toNativeSettings(adjustments, curve, whiteBalanceMode, whiteBalanceSample, toneCurves) },
  })
}
