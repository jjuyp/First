import { convertFileSrc, invoke, isTauri } from '@tauri-apps/api/core'
import { open, save } from '@tauri-apps/plugin-dialog'
import type { Adjustments } from './editorState'
import type { RadialMask, ToneCurvePoint } from './imagePipeline'

export type RenderBackend = 'native' | 'browserFallback'

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
  curve: Array<{ x: number; y: number }>
}

export interface NativePreviewResult {
  width: number
  height: number
  inputProfile: 'embedded ICC' | 'assumed sRGB' | 'LibRaw camera matrix'
  jpeg: Uint8Array
}

export interface NativeExportResult {
  outputPath: string
  width: number
  height: number
  inputProfile: string
  workingSpace: string
}

const HEADER_BYTES = 20

export const nativeRuntimeAvailable = () => isTauri()

export function assertNativeSupported(adjustments: Adjustments, mask: RadialMask) {
  const unsupported: string[] = []
  if (adjustments.clarity !== 0) unsupported.push('Clarity')
  if (adjustments.sharpness < 0) unsupported.push('negative Sharpness')
  if (adjustments.noiseReduction < 0) unsupported.push('negative Noise reduction')
  if (adjustments.maskExposure !== 0 || mask.x !== .5 || mask.y !== .5 || mask.width !== .42
    || mask.height !== .42 || mask.rotation !== 0) unsupported.push('Masks')
  if (adjustments.vignette !== 0 || adjustments.lensBrightness !== 0) unsupported.push('Optics')
  if (adjustments.rotation !== 0 || adjustments.flipHorizontal !== 0 || adjustments.flipVertical !== 0) unsupported.push('Geometry')
  if (unsupported.length) {
    throw new Error(`Native M1C does not support ${unsupported.join(', ')} yet; Browser fallback was not used.`)
  }
}

export function toNativeSettings(adjustments: Adjustments, curve: ToneCurvePoint[]): NativeEditSettings {
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
    curve: [...curve].sort((left, right) => left.x - right.x).map(({ x, y }) => ({ x, y })),
  }
}

export function parseNativePreviewFrame(value: ArrayBuffer | Uint8Array): NativePreviewResult {
  const bytes = value instanceof Uint8Array ? value : new Uint8Array(value)
  if (bytes.byteLength < HEADER_BYTES || String.fromCharCode(...bytes.subarray(0, 4)) !== 'SRP1') {
    throw new Error('Native preview returned an invalid binary frame.')
  }
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength)
  const version = view.getUint16(4, true)
  if (version !== 1) throw new Error(`Unsupported native preview contract version ${version}.`)
  const flags = view.getUint16(6, true)
  const width = view.getUint32(8, true)
  const height = view.getUint32(12, true)
  const payloadLength = view.getUint32(16, true)
  if (!width || !height || HEADER_BYTES + payloadLength !== bytes.byteLength) {
    throw new Error('Native preview returned inconsistent dimensions or payload length.')
  }
  return {
    width,
    height,
    inputProfile: flags & 2 ? 'LibRaw camera matrix' : flags & 1 ? 'embedded ICC' : 'assumed sRGB',
    jpeg: bytes.slice(HEADER_BYTES),
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
  maxEdge = 1800,
) {
  assertNativeSupported(adjustments, mask)
  const frame = await invoke<ArrayBuffer | Uint8Array>('native_preview', {
    request: { sourcePath, maxEdge, settings: toNativeSettings(adjustments, curve) },
  })
  return parseNativePreviewFrame(frame)
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
) {
  assertNativeSupported(adjustments, mask)
  return invoke<NativeExportResult>('native_export_jpeg', {
    request: { sourcePath, outputPath, quality: 94, settings: toNativeSettings(adjustments, curve) },
  })
}
