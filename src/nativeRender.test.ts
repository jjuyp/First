import { describe, expect, it } from 'vitest'
import { assertNativeSupported, parseNativePreviewFrame, toNativeSettings } from './nativeRender'
import { defaultAdjustments } from './editorState'

const defaultMask = { x: .5, y: .5, width: .42, height: .42, rotation: 0 }

describe('native preview contract', () => {
  it('parses the versioned binary frame without JSON pixel arrays', () => {
    const payload = new Uint8Array([0xff, 0xd8, 0xff])
    const profile = new TextEncoder().encode('dng-forward-matrix:test:camera')
    const frame = new Uint8Array(24 + profile.length + payload.length)
    frame.set([83, 82, 80, 50])
    const view = new DataView(frame.buffer)
    view.setUint16(4, 2, true)
    view.setUint16(6, 2, true)
    view.setUint32(8, 640, true)
    view.setUint32(12, 480, true)
    view.setUint16(16, profile.length, true)
    view.setUint32(20, payload.length, true)
    frame.set(profile, 24)
    frame.set(payload, 24 + profile.length)
    expect(parseNativePreviewFrame(frame)).toEqual({
      width: 640,
      height: 480,
      inputProfile: 'resolved RAW camera profile',
      cameraProfileId: 'dng-forward-matrix:test:camera',
      jpeg: payload,
    })
  })

  it('serializes exposure, WB, tone and curve without creative image math in TypeScript', () => {
    const settings = toNativeSettings(
      { ...defaultAdjustments, exposure: .75, temperature: 25, shadows: 30 },
      [{ id: 'white', x: 1, y: 1 }, { id: 'black', x: 0, y: 0 }],
    )
    expect(settings.exposure).toBe(.75)
    expect(settings.temperature).toBe(25)
    expect(settings.shadows).toBe(30)
    expect(settings.curve).toEqual([{ x: 0, y: 0 }, { x: 1, y: 1 }])
  })

  it('never silently ignores edits outside the M1C native slice', () => {
    expect(() => assertNativeSupported({ ...defaultAdjustments, clarity: 10 }, defaultMask))
      .toThrow(/Browser fallback was not used/)
  })
})
