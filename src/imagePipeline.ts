import { defaultAdjustments, type Adjustments } from './editorState'

export interface ToneCurvePoint { id: string; x: number; y: number }
export interface RadialMask { x: number; y: number; width: number; height: number; rotation: number }

const clamp01 = (value: number) => Math.min(1, Math.max(0, value))

function srgbToLinear(value: number) {
  return value <= 0.04045 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4
}

function linearToSrgb(value: number) {
  const safe = clamp01(value)
  return safe <= 0.0031308 ? safe * 12.92 : 1.055 * safe ** (1 / 2.4) - 0.055
}

function smoothstep(edge0: number, edge1: number, value: number) {
  if (Math.abs(edge1 - edge0) < Number.EPSILON) return value < edge0 ? 0 : 1
  const t = clamp01((value - edge0) / (edge1 - edge0))
  return t * t * (3 - 2 * t)
}

function mapToneCurve(value: number, points?: ToneCurvePoint[]) {
  if (!points?.length) return value
  const sorted = [...points].sort((a, b) => a.x - b.x)
  if (value <= sorted[0].x) return sorted[0].y
  for (let index = 1; index < sorted.length; index += 1) {
    const left = sorted[index - 1]
    const right = sorted[index]
    if (value <= right.x) {
      const amount = (value - left.x) / Math.max(0.0001, right.x - left.x)
      return left.y + (right.y - left.y) * amount
    }
  }
  return sorted.at(-1)?.y ?? value
}

export function hasAdjustments(adjustments: Adjustments) {
  return (Object.keys(defaultAdjustments) as Array<keyof Adjustments>)
    .some((key) => adjustments[key] !== defaultAdjustments[key])
}

/**
 * Browser reference for the v0.2 Rust tone engine. Creative production math is moving to
 * starroom-color; this keeps the interactive vertical slice usable during the migration.
 * Tone controls remap luminance and scale RGB together instead of blending RGB toward white.
 */
function remapToneLuminance(luminance: number, adjustments: Adjustments) {
  let out = Math.max(0, luminance)
  // A narrow, bell-like shadow zone keeps true black anchored and fades before midtones.
  // This specifically prevents the v0.1 "white veil" failure mode.
  const shadowWeight = smoothstep(0.004, 0.012, out) * (1 - smoothstep(0.06, 0.18, out))
  const blackWeight = 1 - smoothstep(0, 0.11, out)
  const highlightWeight = smoothstep(0.34, 0.62, out) * (1 - smoothstep(1.10, 1.55, out))
  const whiteWeight = smoothstep(0.72, 1.02, out)
  const shadows = adjustments.shadows / 100
  const highlights = adjustments.highlights / 100
  const whites = adjustments.whites / 100
  const blacks = adjustments.blacks / 100

  if (shadows >= 0) out += shadows * shadowWeight * (0.24 + 0.18 * Math.sqrt(out)) * (1 - Math.min(1, out))
  else out *= 1 + shadows * shadowWeight * 0.72

  if (highlights < 0) {
    const compression = 1 + -highlights * highlightWeight * 1.35
    out = out / compression + Math.min(out, 0.22) * (1 - 1 / compression)
  } else out += highlights * highlightWeight * (1 - Math.min(1, out)) * 0.22

  if (blacks >= 0) out += blacks * blackWeight * 0.055
  else out *= 1 + blacks * blackWeight * 0.82

  if (whites >= 0) out += whites * whiteWeight * (0.10 + 0.10 * Math.min(1, out))
  else out *= 1 + whites * whiteWeight * 0.48

  const contrast = adjustments.contrast / 100
  if (Math.abs(contrast) > Number.EPSILON) {
    const pivot = 0.18
    const safe = Math.max(1e-6, out)
    const stops = Math.log2(safe / pivot)
    out = pivot * (2 ** (stops * (1 + contrast * 0.62)))
  }
  return Number.isFinite(out) ? Math.max(0, out) : 0
}

function applyDetail(imageData: ImageData, noiseReduction: number, sharpness: number) {
  if (!noiseReduction && !sharpness) return
  const { width, height, data } = imageData
  const source = new Uint8ClampedArray(data)
  const denoiseMix = noiseReduction / 100
  const sharpenMix = sharpness / 100

  for (let y = 1; y < height - 1; y += 1) {
    for (let x = 1; x < width - 1; x += 1) {
      const index = (y * width + x) * 4
      for (let channel = 0; channel < 3; channel += 1) {
        let sum = 0
        for (let offsetY = -1; offsetY <= 1; offsetY += 1) {
          for (let offsetX = -1; offsetX <= 1; offsetX += 1) {
            sum += source[((y + offsetY) * width + x + offsetX) * 4 + channel]
          }
        }
        const blurred = sum / 9
        const original = source[index + channel]
        const denoised = denoiseMix >= 0
          ? original * (1 - denoiseMix * .9) + blurred * denoiseMix * .9
          : original + (original - blurred) * -denoiseMix * .9
        const detailed = sharpenMix >= 0
          ? denoised + (denoised - blurred) * sharpenMix * 3.2
          : denoised * (1 + sharpenMix) + blurred * -sharpenMix
        data[index + channel] = Math.round(Math.min(255, Math.max(0, detailed)))
      }
    }
  }
}

export function processImageData(imageData: ImageData, adjustments: Adjustments, curvePoints?: ToneCurvePoint[], mask?: RadialMask) {
  const curveEdited = curvePoints?.some((point) => Math.abs(point.y - point.x) > 0.0001) ?? false
  if (!hasAdjustments(adjustments) && !curveEdited) return imageData

  const pixels = imageData.data
  const exposure = 2 ** adjustments.exposure
  const warmth = Math.max(-1, Math.min(1, (adjustments.temperature - 6500) / 4500))
  const tint = adjustments.tint / 100
  const saturation = 1 + adjustments.saturation / 100
  const maskExposure = 2 ** adjustments.maskExposure
  const maskFeather = Math.max(0.02, adjustments.maskFeather / 100 * 0.7)
  const vignette = adjustments.vignette / 100
  const lensBrightness = adjustments.lensBrightness / 100
  const clarity = adjustments.clarity / 100
  const activeMask = mask ?? { x: .5, y: .5, width: .5, height: .5, rotation: 0 }
  const maskAngle = -activeMask.rotation * Math.PI / 180

  for (let index = 0; index < pixels.length; index += 4) {
    const pixel = index / 4
    const x = pixel % imageData.width
    const y = Math.floor(pixel / imageData.width)
    const normalizedX = x / Math.max(1, imageData.width - 1)
    const normalizedY = y / Math.max(1, imageData.height - 1)
    const deltaX = normalizedX - activeMask.x
    const deltaY = normalizedY - activeMask.y
    const maskX = deltaX * Math.cos(maskAngle) - deltaY * Math.sin(maskAngle)
    const maskY = deltaX * Math.sin(maskAngle) + deltaY * Math.cos(maskAngle)
    const maskDistance = Math.hypot(maskX / Math.max(.02, activeMask.width / 2), maskY / Math.max(.02, activeMask.height / 2))
    const maskWeight = 1 - smoothstep(1, 1 + maskFeather * 2.2, maskDistance)
    const distance = Math.min(1, Math.hypot(normalizedX - .5, normalizedY - .5) / .707)
    const radialScale = 1 + lensBrightness * distance * distance * 0.65 - vignette * distance * distance * 0.78
    const localExposure = 1 + (maskExposure - 1) * maskWeight
    let red = srgbToLinear(pixels[index] / 255)
    let green = srgbToLinear(pixels[index + 1] / 255)
    let blue = srgbToLinear(pixels[index + 2] / 255)

    red *= exposure * localExposure * radialScale * (1 + warmth * 0.22 + tint * 0.08)
    green *= exposure * localExposure * radialScale * (1 - tint * 0.1)
    blue *= exposure * localExposure * radialScale * (1 - warmth * 0.22 + tint * 0.08)

    let luminance = 0.2627 * red + 0.678 * green + 0.0593 * blue
    const targetLuminance = remapToneLuminance(luminance, adjustments)
    const toneScale = luminance > 1e-7 ? targetLuminance / luminance : 0
    red *= toneScale
    green *= toneScale
    blue *= toneScale
    luminance = targetLuminance

    // Transitional clarity preview. Production v0.2+ detail math moves to the Rust/GPU detail stage.
    const midtoneWeight = Math.max(0, 1 - Math.abs(clamp01(luminance) - 0.5) * 2)
    const clarityContrast = 1 + clarity * midtoneWeight * 0.65
    red = 0.18 + (red - 0.18) * clarityContrast
    green = 0.18 + (green - 0.18) * clarityContrast
    blue = 0.18 + (blue - 0.18) * clarityContrast

    red = linearToSrgb(red)
    green = linearToSrgb(green)
    blue = linearToSrgb(blue)

    red = mapToneCurve(red, curvePoints)
    green = mapToneCurve(green, curvePoints)
    blue = mapToneCurve(blue, curvePoints)

    luminance = 0.2126 * red + 0.7152 * green + 0.0722 * blue
    const chroma = Math.max(red, green, blue) - Math.min(red, green, blue)
    const vibrance = 1 + (adjustments.vibrance / 100) * (1 - chroma) * 0.8
    const colorScale = Math.max(0, saturation * vibrance)

    pixels[index] = Math.round(clamp01(luminance + (red - luminance) * colorScale) * 255)
    pixels[index + 1] = Math.round(clamp01(luminance + (green - luminance) * colorScale) * 255)
    pixels[index + 2] = Math.round(clamp01(luminance + (blue - luminance) * colorScale) * 255)
  }

  applyDetail(imageData, adjustments.noiseReduction, adjustments.sharpness)
  return imageData
}

export function calculateHistogram(imageData: ImageData, bins = 48) {
  const values = Array.from({ length: bins }, () => 0)
  const pixels = imageData.data
  const stride = Math.max(4, Math.floor(pixels.length / 300_000 / 4) * 4)

  for (let index = 0; index < pixels.length; index += stride) {
    const luminance = 0.2126 * pixels[index] + 0.7152 * pixels[index + 1] + 0.0722 * pixels[index + 2]
    const bin = Math.min(bins - 1, Math.floor((luminance / 256) * bins))
    values[bin] += 1
  }

  const maximum = Math.max(...values, 1)
  return values.map((value) => value / maximum)
}

export async function renderImageSource(source: string, adjustments: Adjustments, maxEdge = Number.POSITIVE_INFINITY, curvePoints?: ToneCurvePoint[], mask?: RadialMask) {
  const image = new Image()
  image.decoding = 'async'
  image.src = source
  await image.decode()

  const scale = Math.min(1, maxEdge / Math.max(image.naturalWidth, image.naturalHeight))
  const canvas = document.createElement('canvas')
  canvas.width = Math.max(1, Math.round(image.naturalWidth * scale))
  canvas.height = Math.max(1, Math.round(image.naturalHeight * scale))
  const context = canvas.getContext('2d', { willReadFrequently: true })
  if (!context) throw new Error('Canvas 2D is unavailable.')

  context.drawImage(image, 0, 0, canvas.width, canvas.height)
  const normalizedRotation = ((adjustments.rotation % 360) + 360) % 360
  let output = canvas
  if (normalizedRotation || adjustments.flipHorizontal || adjustments.flipVertical) {
    const transformed = document.createElement('canvas')
    const angle = normalizedRotation * Math.PI / 180
    transformed.width = Math.max(1, Math.ceil(Math.abs(canvas.width * Math.cos(angle)) + Math.abs(canvas.height * Math.sin(angle))))
    transformed.height = Math.max(1, Math.ceil(Math.abs(canvas.width * Math.sin(angle)) + Math.abs(canvas.height * Math.cos(angle))))
    const transformedContext = transformed.getContext('2d')
    if (!transformedContext) throw new Error('Canvas 2D is unavailable.')
    transformedContext.translate(transformed.width / 2, transformed.height / 2)
    transformedContext.scale(adjustments.flipHorizontal ? -1 : 1, adjustments.flipVertical ? -1 : 1)
    transformedContext.rotate(angle)
    transformedContext.drawImage(canvas, -canvas.width / 2, -canvas.height / 2)
    output = transformed
  }

  const outputContext = output.getContext('2d', { willReadFrequently: true })
  if (!outputContext) throw new Error('Canvas 2D is unavailable.')
  const imageData = outputContext.getImageData(0, 0, output.width, output.height)
  outputContext.putImageData(processImageData(imageData, adjustments, curvePoints, mask), 0, 0)
  return output
}
