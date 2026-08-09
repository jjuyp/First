import { describe, expect, it } from 'vitest'
import { calculateHistogram, hasAdjustments, processImageData } from './imagePipeline'
import { defaultAdjustments } from './editorState'

function pixels(values: number[]) {
  return { data: new Uint8ClampedArray(values), width: values.length / 4, height: 1 } as ImageData
}

describe('image pipeline', () => {
  it('keeps neutral adjustments pixel-exact', () => {
    const source = pixels([40, 80, 120, 255])
    expect(Array.from(processImageData(source, defaultAdjustments).data)).toEqual([40, 80, 120, 255])
  })

  it('changes rendered pixels when exposure changes', () => {
    const source = pixels([40, 80, 120, 255])
    const result = processImageData(source, { ...defaultAdjustments, exposure: 1 })
    expect(result.data[0]).toBeGreaterThan(40)
    expect(result.data[1]).toBeGreaterThan(80)
    expect(result.data[2]).toBeGreaterThan(120)
  })

  it('treats non-zero neutral defaults as unedited', () => {
    expect(hasAdjustments(defaultAdjustments)).toBe(false)
    expect(hasAdjustments({ ...defaultAdjustments, temperature: 6000 })).toBe(true)
  })

  it('applies tone-curve controls to output pixels', () => {
    const source = pixels([35, 35, 35, 255, 128, 128, 128, 255, 220, 220, 220, 255])
    const result = processImageData(source, defaultAdjustments, [
      { id: 'black', x: 0, y: 0 },
      { id: 'shadow', x: .25, y: .5 },
      { id: 'highlight', x: .75, y: .5 },
      { id: 'white', x: 1, y: 1 },
    ])
    expect(result.data[0]).toBeGreaterThan(35)
    expect(result.data[8]).toBeLessThan(220)
  })

  it('applies a visible sharpness change around an edge', () => {
    const values: number[] = []
    for (let index = 0; index < 9; index += 1) {
      const level = index === 4 ? 160 : 80
      values.push(level, level, level, 255)
    }
    const source = { data: new Uint8ClampedArray(values), width: 3, height: 3 } as ImageData
    const result = processImageData(source, { ...defaultAdjustments, sharpness: 100 })
    expect(result.data[16]).toBeGreaterThan(160)
  })

  it('uses Kelvin temperature values', () => {
    const source = pixels([100, 100, 100, 255])
    const result = processImageData(source, { ...defaultAdjustments, temperature: 10000 })
    expect(result.data[0]).toBeGreaterThan(result.data[2])
  })

  it('returns a normalized histogram', () => {
    const result = calculateHistogram(pixels([0, 0, 0, 255, 255, 255, 255, 255]), 8)
    expect(Math.max(...result)).toBe(1)
    expect(result[0]).toBeGreaterThan(0)
    expect(result[7]).toBeGreaterThan(0)
  })
})
