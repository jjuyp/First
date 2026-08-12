import { useEffect, useMemo, useRef, useState } from 'react'
import {
  Aperture, Blend, ChevronDown, Columns2, Contrast, Crop, Download, Folder,
  Grid2X2, ImagePlus, Library, PanelBottomClose, PanelBottomOpen, PanelLeftClose,
  PanelLeftOpen, Redo2, RotateCcw, RotateCw, ScanFace, ScanLine, Sparkles, Star,
  SunMedium, Trash2, Undo2, FlipHorizontal2, FlipVertical2, Move,
} from 'lucide-react'
import {
  type AdjustmentKey, type Adjustments, type Theme, type Tool,
  defaultAdjustments,
} from './editorState'
import {
  calculateHistogram, hasAdjustments, mapToneCurve, renderImageSource,
  type RadialMask, type ToneCurvePoint,
} from './imagePipeline'
import {
  chooseNativeExportPath, chooseNativePhotoPaths, exportNativeJpeg, nativeRuntimeAvailable,
  nativeThumbnailUrl, renderNativePreview, sampleNativeColor, type NativeToneCurves, type NativeWhiteBalanceMode, type NativeWhiteBalanceSample, type RenderBackend,
} from './nativeRender'

type LibraryFilter = 'all' | 'recent' | 'five-star' | 'edited'
type WorkspaceView = 'library' | 'edit' | 'compare'

interface PhotoItem {
  id: string
  name: string
  src: string
  sourcePath?: string
  renderBackend: RenderBackend
  imported: boolean
  rating: number
  adjustments: Adjustments
  curvePoints: ToneCurvePoint[]
  curveChannels: NativeToneCurves
  whiteBalanceMode: NativeWhiteBalanceMode
  whiteBalanceSample: NativeWhiteBalanceSample | null
  mask: RadialMask
  history: EditSnapshot[]
  future: EditSnapshot[]
}

interface EditSnapshot {
  adjustments: Adjustments
  curvePoints: ToneCurvePoint[]
  curveChannels: NativeToneCurves
  whiteBalanceMode: NativeWhiteBalanceMode
  whiteBalanceSample: NativeWhiteBalanceSample | null
  mask: RadialMask
}

const defaultCurvePoints: ToneCurvePoint[] = [
  { id: 'black', x: 0, y: 0 },
  { id: 'shadow', x: .25, y: .25 },
  { id: 'midtone', x: .5, y: .5 },
  { id: 'highlight', x: .75, y: .75 },
  { id: 'white', x: 1, y: 1 },
]
const defaultMask: RadialMask = { x: .5, y: .5, width: .42, height: .42, rotation: 0 }

const copyCurve = (points: ToneCurvePoint[]) => points.map((point) => ({ ...point }))
const defaultCurveChannels = (): NativeToneCurves => ({ master: copyCurve(defaultCurvePoints), red: [], green: [], blue: [] })
const copyCurveChannels = (curves: NativeToneCurves): NativeToneCurves => ({ master: copyCurve(curves.master), red: copyCurve(curves.red), green: copyCurve(curves.green), blue: copyCurve(curves.blue) })
const takeSnapshot = (photo: PhotoItem): EditSnapshot => ({
  adjustments: { ...photo.adjustments }, curvePoints: copyCurve(photo.curvePoints), curveChannels: copyCurveChannels(photo.curveChannels), whiteBalanceMode: photo.whiteBalanceMode,
  whiteBalanceSample: photo.whiteBalanceSample ? { ...photo.whiteBalanceSample } : null, mask: { ...photo.mask },
})
const applySnapshot = (photo: PhotoItem, snapshot: EditSnapshot) => ({
  ...photo, adjustments: { ...snapshot.adjustments }, curvePoints: copyCurve(snapshot.curvePoints), curveChannels: copyCurveChannels(snapshot.curveChannels),
  whiteBalanceMode: snapshot.whiteBalanceMode, whiteBalanceSample: snapshot.whiteBalanceSample ? { ...snapshot.whiteBalanceSample } : null, mask: { ...snapshot.mask },
})
const hasCurveEdits = (points: ToneCurvePoint[]) => points.length !== defaultCurvePoints.length
  || points.some((point, index) => Math.abs(point.x - defaultCurvePoints[index].x) > .0001 || Math.abs(point.y - defaultCurvePoints[index].y) > .0001)
const hasMaskGeometryEdits = (mask: RadialMask) => (Object.keys(defaultMask) as Array<keyof RadialMask>)
  .some((key) => Math.abs(mask[key] - defaultMask[key]) > .0001)
const hasPhotoEdits = (photo: PhotoItem) => hasAdjustments(photo.adjustments) || hasCurveEdits(photo.curvePoints) || hasMaskGeometryEdits(photo.mask)
const countPhotoEdits = (photo: PhotoItem) => (Object.keys(defaultAdjustments) as AdjustmentKey[])
  .filter((key) => photo.adjustments[key] !== defaultAdjustments[key]).length
  + (hasCurveEdits(photo.curvePoints) ? 1 : 0) + (hasMaskGeometryEdits(photo.mask) ? 1 : 0)

const demoPhoto: PhotoItem = {
  id: 'starroom-demo',
  name: 'Starroom Demo.svg',
  src: '/starroom-demo.svg',
  renderBackend: 'browserFallback',
  imported: false,
  rating: 0,
  adjustments: { ...defaultAdjustments },
  curvePoints: copyCurve(defaultCurvePoints),
  curveChannels: defaultCurveChannels(),
  whiteBalanceMode: 'sourceDefault',
  whiteBalanceSample: null,
  mask: { ...defaultMask },
  history: [],
  future: [],
}

const toolItems: Array<{ id: Tool; label: string; icon: typeof SunMedium }> = [
  { id: 'light', label: 'Light', icon: SunMedium },
  { id: 'color', label: 'Color', icon: Blend },
  { id: 'curve', label: 'Curve', icon: ScanLine },
  { id: 'detail', label: 'Detail', icon: Aperture },
  { id: 'masks', label: 'Masks', icon: ScanFace },
  { id: 'optics', label: 'Optics', icon: Contrast },
  { id: 'geometry', label: 'Geometry', icon: Crop },
]

const sliderGroups: Partial<Record<Tool, Array<{ key: AdjustmentKey; label: string; min: number; max: number; step: number; suffix?: string }>>> = {
  light: [
    { key: 'exposure', label: 'Exposure', min: -5, max: 5, step: .01, suffix: ' EV' },
    { key: 'contrast', label: 'Contrast', min: -100, max: 100, step: 1 },
    { key: 'highlights', label: 'Highlights', min: -100, max: 100, step: 1 },
    { key: 'shadows', label: 'Shadows', min: -100, max: 100, step: 1 },
    { key: 'whites', label: 'Whites', min: -100, max: 100, step: 1 },
    { key: 'blacks', label: 'Blacks', min: -100, max: 100, step: 1 },
  ],
  color: [
    { key: 'temperature', label: 'Temperature', min: -100, max: 100, step: 1 },
    { key: 'tint', label: 'Tint', min: -100, max: 100, step: 1 },
    { key: 'vibrance', label: 'Vibrance', min: -100, max: 100, step: 1 },
    { key: 'saturation', label: 'Saturation', min: -100, max: 100, step: 1 },
  ],
  detail: [
    { key: 'sharpness', label: 'Sharpness', min: -100, max: 100, step: 1 },
    { key: 'clarity', label: 'Clarity', min: -100, max: 100, step: 1 },
    { key: 'noiseReduction', label: 'Noise reduction', min: -100, max: 100, step: 1 },
  ],
  masks: [
    { key: 'maskExposure', label: 'Center exposure', min: -3, max: 3, step: .01, suffix: ' EV' },
    { key: 'maskFeather', label: 'Feather', min: 0, max: 100, step: 1 },
  ],
  optics: [
    { key: 'vignette', label: 'Vignette', min: -100, max: 100, step: 1 },
    { key: 'lensBrightness', label: 'Edge brightness', min: -100, max: 100, step: 1 },
  ],
  geometry: [
    { key: 'rotation', label: 'Rotation', min: -180, max: 180, step: .1, suffix: '°' },
  ],
}

function usePersistedValue<T>(key: string, initial: T) {
  const [value, setValue] = useState<T>(() => {
    const saved = localStorage.getItem(key)
    return saved ? JSON.parse(saved) as T : initial
  })
  useEffect(() => localStorage.setItem(key, JSON.stringify(value)), [key, value])
  return [value, setValue] as const
}

function IconButton({ label, disabled, onClick, children }: { label: string; disabled?: boolean; onClick?: () => void; children: React.ReactNode }) {
  return <button className="icon-button" aria-label={label} title={label} disabled={disabled} onClick={onClick}>{children}</button>
}

function Slider({ label, value, min, max, step, suffix = '', onBeginEdit, onChange, onReset }: {
  label: string; value: number; min: number; max: number; step: number; suffix?: string
  onBeginEdit: () => void; onChange: (value: number) => void; onReset: () => void
}) {
  const [active, setActive] = useState(false)
  const percent = ((value - min) / (max - min)) * 100
  const display = step < 1 ? value.toFixed(step < .1 ? 2 : 1) : Math.round(value).toString()
  const [draft, setDraft] = useState(display)
  const [editing, setEditing] = useState(false)
  const commitDraft = () => {
    const parsed = Number(draft)
    if (Number.isFinite(parsed)) onChange(Math.min(max, Math.max(min, parsed)))
    setEditing(false)
  }
  return <div className="slider-row">
    <div className="slider-label"><span>{label}</span><label className="numeric-editor" title={`Type ${label} value`}>
      <input aria-label={`${label} value`} type="number" min={min} max={max} step={step} value={editing ? draft : display}
        onFocus={(event) => { onBeginEdit(); setEditing(true); setDraft(display); event.currentTarget.select() }}
        onChange={(event) => setDraft(event.target.value)} onBlur={commitDraft}
        onKeyDown={(event) => { if (event.key === 'Enter') event.currentTarget.blur(); if (event.key === 'Escape') { setDraft(display); event.currentTarget.blur() } }} />
      {suffix && <span>{suffix.trim()}</span>}
    </label></div>
    <div className={`slider-wrap ${active ? 'is-active' : ''}`} style={{ '--fill': `${percent}%` } as React.CSSProperties}>
      <input aria-label={label} type="range" min={min} max={max} step={step} value={value}
        onChange={(event) => onChange(Number(event.target.value))}
        onPointerDown={() => { onBeginEdit(); setActive(true) }} onPointerUp={() => setActive(false)}
        onBlur={() => setActive(false)} onDoubleClick={onReset} />
      <span className="value-bubble" style={{ left: `${percent}%` }}>{display}</span>
    </div>
  </div>
}

function Histogram({ values }: { values: number[] }) {
  return <div className="histogram" aria-label="Live photo histogram">
    {values.map((height, index) => <i key={index} style={{ height: `${Math.max(2, height * 100)}%` }} />)}
  </div>
}

function ToneCurveEditor({ points, selectedId, histogram, onSelect, onBeginEdit, onChange }: {
  points: ToneCurvePoint[]; selectedId: string | null
  histogram: number[]
  onSelect: (id: string) => void; onBeginEdit: () => void; onChange: (points: ToneCurvePoint[]) => void
}) {
  const svgRef = useRef<SVGSVGElement>(null)
  const [dragId, setDragId] = useState<string | null>(null)
  const sorted = [...points].sort((a, b) => a.x - b.x)
  const selected = sorted.find((point) => point.id === selectedId) ?? sorted[2] ?? sorted[0]
  const path = Array.from({ length: 61 }, (_, index) => {
    const x = index / 60
    const y = mapToneCurve(x, sorted)
    return `${index ? 'L' : 'M'} ${x * 300} ${(1 - y) * 120}`
  }).join(' ')
  const eventPoint = (event: React.PointerEvent<SVGSVGElement>) => {
    const rect = event.currentTarget.getBoundingClientRect()
    return {
      x: Math.max(0, Math.min(1, (event.clientX - rect.left) / rect.width)),
      y: Math.max(0, Math.min(1, 1 - (event.clientY - rect.top) / rect.height)),
    }
  }
  const updatePoint = (id: string, next: Partial<ToneCurvePoint>) => {
    const updated = points.map((point) => point.id === id ? { ...point, ...next } : point).sort((a, b) => a.x - b.x)
    onChange(updated)
  }
  const addPoint = (event: React.PointerEvent<SVGSVGElement>) => {
    if (event.button !== 0 || event.target !== event.currentTarget && (event.target as Element).tagName !== 'path') return
    const position = eventPoint(event)
    const point = { id: crypto.randomUUID(), ...position }
    onBeginEdit()
    onChange([...points, point].sort((a, b) => a.x - b.x))
    onSelect(point.id)
  }
  const removePoint = (event: React.MouseEvent, id: string) => {
    event.preventDefault()
    if (id === 'black' || id === 'white') return
    onBeginEdit()
    onChange(points.filter((point) => point.id !== id))
    onSelect('midtone')
  }

  return <>
    <div className="curve-presets"><button onClick={() => { onBeginEdit(); onChange(copyCurve(defaultCurvePoints)) }}>Identity</button><button onClick={() => { onBeginEdit(); onChange([{ id: 'black', x: 0, y: 0 }, { id: 'shadow', x: .25, y: .18 }, { id: 'midtone', x: .5, y: .5 }, { id: 'highlight', x: .75, y: .84 }, { id: 'white', x: 1, y: 1 }]) }}>S-curve</button><button onClick={() => { onBeginEdit(); onChange([{ id: 'black', x: 0, y: .10 }, { id: 'midtone', x: .5, y: .55 }, { id: 'white', x: 1, y: 1 }]) }}>Black fade</button></div>
    <svg ref={svgRef} className="curve-preview curve-editor" viewBox="0 0 300 120" preserveAspectRatio="none"
      aria-label="Editable tone curve. Left click to add a point; drag points to adjust; right click a point to delete."
      onPointerDown={addPoint}
      onPointerMove={(event) => {
        if (!dragId) return
        const position = eventPoint(event)
        const endpoint = dragId === 'black' || dragId === 'white'
        updatePoint(dragId, { x: endpoint ? (dragId === 'black' ? 0 : 1) : position.x, y: position.y })
      }}
      onPointerUp={(event) => { setDragId(null); if (event.currentTarget.hasPointerCapture(event.pointerId)) event.currentTarget.releasePointerCapture(event.pointerId) }}>
      <g className="curve-grid">
        <line x1="75" y1="0" x2="75" y2="120" /><line x1="150" y1="0" x2="150" y2="120" /><line x1="225" y1="0" x2="225" y2="120" />
        <line x1="0" y1="30" x2="300" y2="30" /><line x1="0" y1="60" x2="300" y2="60" /><line x1="0" y1="90" x2="300" y2="90" />
      </g>
      <g className="curve-histogram">{histogram.map((height, index) => <rect key={index} x={index * 300 / histogram.length} y={(1 - height) * 120} width={300 / histogram.length} height={height * 120} />)}</g>
      <line className="curve-baseline" x1="0" y1="120" x2="300" y2="0" />
      <path className="curve-hit-line" d={path} />
      <path className="curve-line" d={path} />
      {sorted.map((point) => <circle key={point.id} className={`curve-point ${point.id === selected?.id ? 'selected' : ''}`}
        cx={point.x * 300} cy={(1 - point.y) * 120} r="5"
        onPointerDown={(event) => { if (event.button !== 0) return; event.stopPropagation(); onSelect(point.id); onBeginEdit(); setDragId(point.id); svgRef.current?.setPointerCapture(event.pointerId) }}
        onContextMenu={(event) => removePoint(event, point.id)}>
        <title>Input {Math.round(point.x * 100)}, output {Math.round(point.y * 100)}{point.id === 'black' || point.id === 'white' ? ' (endpoint)' : ' · right click to delete'}</title>
      </circle>)}
    </svg>
    <div className="curve-help">Monotone curve · left click line to add · drag point · right click to delete</div>
    {selected && <div className="curve-values">
      <label>Input <input aria-label="Selected curve point input" type="number" min="0" max="100" step="1" value={Math.round(selected.x * 100)}
        disabled={selected.id === 'black' || selected.id === 'white'} onFocus={onBeginEdit}
        onChange={(event) => updatePoint(selected.id, { x: Number(event.target.value) / 100 })} /></label>
      <label>Output <input aria-label="Selected curve point output" type="number" min="0" max="100" step="1" value={Math.round(selected.y * 100)}
        onFocus={onBeginEdit} onChange={(event) => updatePoint(selected.id, { y: Number(event.target.value) / 100 })} /></label>
    </div>}
  </>
}

function CurveChannelTabs({ value, onChange }: { value: keyof NativeToneCurves; onChange: (value: keyof NativeToneCurves) => void }) {
  return <div className="curve-tabs" aria-label="Tone curve channel">{(['master', 'red', 'green', 'blue'] as const).map((channel) => <button key={channel}
    className={value === channel ? 'active' : ''} onClick={() => onChange(channel)}>{channel === 'master' ? 'Master' : channel[0].toUpperCase() + channel.slice(1)}</button>)}</div>
}

type MaskDragMode = 'move' | 'width' | 'height' | 'rotate' | null

function MaskOverlay({ bounds, mask, onBeginEdit, onChange }: {
  bounds: { left: number; top: number; width: number; height: number }
  mask: RadialMask; onBeginEdit: () => void; onChange: (mask: RadialMask) => void
}) {
  const [dragMode, setDragMode] = useState<MaskDragMode>(null)
  const svgRef = useRef<SVGSVGElement>(null)
  const position = (event: React.PointerEvent<SVGSVGElement>) => {
    const rect = event.currentTarget.getBoundingClientRect()
    return { x: (event.clientX - rect.left) / rect.width, y: (event.clientY - rect.top) / rect.height }
  }
  const angle = mask.rotation * Math.PI / 180
  const rotatePoint = (localX: number, localY: number) => ({
    x: mask.x + localX * Math.cos(angle) - localY * Math.sin(angle),
    y: mask.y + localX * Math.sin(angle) + localY * Math.cos(angle),
  })
  const widthHandle = rotatePoint(mask.width / 2, 0)
  const heightHandle = rotatePoint(0, mask.height / 2)
  const rotationHandle = rotatePoint(0, -mask.height / 2 - .08)
  const beginDrag = (event: React.PointerEvent, mode: MaskDragMode) => {
    if (event.button !== 0) return
    event.stopPropagation()
    onBeginEdit()
    setDragMode(mode)
    svgRef.current?.setPointerCapture(event.pointerId)
  }

  return <svg ref={svgRef} className="mask-overlay" style={{ left: bounds.left, top: bounds.top, width: bounds.width, height: bounds.height }}
    viewBox="0 0 1000 1000" preserveAspectRatio="none" aria-label="Editable radial mask"
    onPointerDown={(event) => {
      if (event.button !== 0 || event.target !== event.currentTarget) return
      const next = position(event)
      onBeginEdit()
      onChange({ ...mask, x: Math.max(0, Math.min(1, next.x)), y: Math.max(0, Math.min(1, next.y)) })
    }}
    onPointerMove={(event) => {
      if (!dragMode) return
      const next = position(event)
      const dx = next.x - mask.x
      const dy = next.y - mask.y
      const localX = dx * Math.cos(-angle) - dy * Math.sin(-angle)
      const localY = dx * Math.sin(-angle) + dy * Math.cos(-angle)
      if (dragMode === 'move') onChange({ ...mask, x: Math.max(0, Math.min(1, next.x)), y: Math.max(0, Math.min(1, next.y)) })
      if (dragMode === 'width') onChange({ ...mask, width: Math.max(.04, Math.min(1.6, Math.abs(localX) * 2)) })
      if (dragMode === 'height') onChange({ ...mask, height: Math.max(.04, Math.min(1.6, Math.abs(localY) * 2)) })
      if (dragMode === 'rotate') onChange({ ...mask, rotation: Math.atan2(dy, dx) * 180 / Math.PI + 90 })
    }}
    onPointerUp={(event) => { setDragMode(null); if (event.currentTarget.hasPointerCapture(event.pointerId)) event.currentTarget.releasePointerCapture(event.pointerId) }}>
    <g transform={`rotate(${mask.rotation} ${mask.x * 1000} ${mask.y * 1000})`}>
      <ellipse className="mask-feather-ring" cx={mask.x * 1000} cy={mask.y * 1000} rx={mask.width * 580} ry={mask.height * 580} />
      <ellipse className="mask-ring" cx={mask.x * 1000} cy={mask.y * 1000} rx={mask.width * 500} ry={mask.height * 500}
        onPointerDown={(event) => beginDrag(event, 'move')} />
    </g>
    <line className="mask-rotation-line" x1={mask.x * 1000} y1={mask.y * 1000} x2={rotationHandle.x * 1000} y2={rotationHandle.y * 1000} />
    <circle className="mask-center-handle" cx={mask.x * 1000} cy={mask.y * 1000} r="9" onPointerDown={(event) => beginDrag(event, 'move')} />
    <circle className="mask-handle" cx={widthHandle.x * 1000} cy={widthHandle.y * 1000} r="11" onPointerDown={(event) => beginDrag(event, 'width')} />
    <circle className="mask-handle" cx={heightHandle.x * 1000} cy={heightHandle.y * 1000} r="11" onPointerDown={(event) => beginDrag(event, 'height')} />
    <circle className="mask-rotate-handle" cx={rotationHandle.x * 1000} cy={rotationHandle.y * 1000} r="12" onPointerDown={(event) => beginDrag(event, 'rotate')} />
  </svg>
}

function PreviewCanvas({ photo, before, zoom, maskActive = false, onBeginMaskEdit, onMaskChange, onWhiteBalancePick, onColorSample, onHistogram, onStatus, onDimensions, metric = true }: {
  photo: PhotoItem; before: boolean; zoom: 'fit' | '100'
  maskActive?: boolean; onBeginMaskEdit?: () => void; onMaskChange?: (mask: RadialMask) => void
  onWhiteBalancePick?: (sample: NativeWhiteBalanceSample) => void
  onColorSample?: (x: number, y: number) => void
  onHistogram: (values: number[]) => void
  onStatus: (status: string) => void
  onDimensions: (dimensions: string) => void
  metric?: boolean
}) {
  const canvasRef = useRef<HTMLCanvasElement>(null)
  const [canvasBounds, setCanvasBounds] = useState({ left: 0, top: 0, width: 0, height: 0 })

  useEffect(() => {
    let cancelled = false
    const timeout = window.setTimeout(async () => {
      onStatus('Rendering…')
      try {
        const adjustments = before ? defaultAdjustments : photo.adjustments
        const curvePoints = before ? defaultCurvePoints : photo.curvePoints
        const mask = before ? defaultMask : photo.mask
        let rendered: CanvasImageSource
        let renderedWidth: number
        let renderedHeight: number
        let nativeProfile = ''
        let release: (() => void) | undefined
        if (photo.renderBackend === 'native') {
          if (!photo.sourcePath) throw new Error('Native photo is missing its source path; Browser fallback was not used.')
          const result = await renderNativePreview(photo.sourcePath, adjustments, curvePoints, mask,
            before ? 'sourceDefault' : photo.whiteBalanceMode, before ? null : photo.whiteBalanceSample,
            before ? defaultCurveChannels() : photo.curveChannels)
          const jpegBuffer = result.jpeg.buffer.slice(
            result.jpeg.byteOffset,
            result.jpeg.byteOffset + result.jpeg.byteLength,
          ) as ArrayBuffer
          const blobUrl = URL.createObjectURL(new Blob([jpegBuffer], { type: 'image/jpeg' }))
          release = () => URL.revokeObjectURL(blobUrl)
          const image = new Image()
          await new Promise<void>((resolve, reject) => {
            image.onload = () => resolve()
            image.onerror = () => reject(new Error('Native preview JPEG could not be decoded.'))
            image.src = blobUrl
          })
          rendered = image
          renderedWidth = result.width
          renderedHeight = result.height
          nativeProfile = result.cameraProfileId ?? result.inputProfile
        } else {
          const fallback = await renderImageSource(photo.src, adjustments, 1800, curvePoints, mask)
          rendered = fallback
          renderedWidth = fallback.width
          renderedHeight = fallback.height
        }
        if (cancelled || !canvasRef.current) {
          release?.()
          return
        }
        const canvas = canvasRef.current
        canvas.width = renderedWidth
        canvas.height = renderedHeight
        const context = canvas.getContext('2d', { willReadFrequently: true })
        if (!context) {
          release?.()
          throw new Error('Canvas 2D is unavailable.')
        }
        context.drawImage(rendered, 0, 0)
        release?.()
        window.requestAnimationFrame(() => {
          if (!canvasRef.current) return
          setCanvasBounds({ left: canvasRef.current.offsetLeft, top: canvasRef.current.offsetTop,
            width: canvasRef.current.clientWidth, height: canvasRef.current.clientHeight })
        })
        if (metric) {
          onHistogram(calculateHistogram(context.getImageData(0, 0, canvas.width, canvas.height)))
          onDimensions(`${renderedWidth} × ${renderedHeight}`)
          onStatus(photo.renderBackend === 'native'
            ? `Native CPU · ${nativeProfile}${before ? ' · original' : ''}`
            : `Browser fallback${before ? ' · original' : ''}`)
        }
      } catch (error) {
        if (!cancelled) onStatus(error instanceof Error ? error.message : 'Preview failed')
      }
    }, 30)

    return () => {
      cancelled = true
      window.clearTimeout(timeout)
    }
  }, [before, metric, onDimensions, onHistogram, onStatus, photo.adjustments, photo.curvePoints, photo.curveChannels, photo.whiteBalanceMode, photo.whiteBalanceSample,
    photo.mask, photo.renderBackend, photo.sourcePath, photo.src])

  useEffect(() => {
    const measure = () => canvasRef.current && setCanvasBounds({ left: canvasRef.current.offsetLeft, top: canvasRef.current.offsetTop,
      width: canvasRef.current.clientWidth, height: canvasRef.current.clientHeight })
    window.addEventListener('resize', measure)
    return () => window.removeEventListener('resize', measure)
  }, [])

  return <>
    <canvas ref={canvasRef} className={`photo-canvas zoom-${zoom}`} aria-label={`Edited preview of ${photo.name}`}
      onDoubleClick={(event) => {
        if (before) return
        const bounds = event.currentTarget.getBoundingClientRect()
        const pointX = Math.max(0, Math.min(1, (event.clientX - bounds.left) / bounds.width))
        const pointY = Math.max(0, Math.min(1, (event.clientY - bounds.top) / bounds.height))
        if (onColorSample) { onColorSample(pointX, pointY); return }
        if (photo.whiteBalanceMode !== 'neutralPicker' || !onWhiteBalancePick) return
        const size = .06
        const x = Math.max(0, Math.min(1 - size, pointX - size / 2))
        const y = Math.max(0, Math.min(1 - size, pointY - size / 2))
        onWhiteBalancePick({ x, y, width: size, height: size })
      }} />
    {maskActive && canvasBounds.width > 0 && onBeginMaskEdit && onMaskChange
      ? <MaskOverlay bounds={canvasBounds} mask={photo.mask} onBeginEdit={onBeginMaskEdit} onChange={onMaskChange} /> : null}
  </>
}

function Inspector({ tool, values, curvePoints, curveChannel, histogram, onCurveChannel, selectedCurvePoint, mask, renderBackend, whiteBalanceMode, onAdjust, onBeginAdjustment, onReset,
  onCurveSelect, onCurveBegin, onCurveChange, onCurvePresetSave, onCurvePresetLoad, canLoadCurvePreset, onMaskBegin, onMaskChange, onWhiteBalanceMode, onCopyWhiteBalance, onPasteWhiteBalance,
  mixerBand, onMixerBand, mixerPicking, onMixerPicking }: {
  tool: Tool; values: Adjustments; curvePoints: ToneCurvePoint[]; curveChannel: keyof NativeToneCurves; histogram: number[]; onCurveChannel: (channel: keyof NativeToneCurves) => void; selectedCurvePoint: string | null; mask: RadialMask; renderBackend: RenderBackend
  onAdjust: (key: AdjustmentKey, value: number, recordHistory?: boolean) => void
  onBeginAdjustment: () => void
  onReset: (key: AdjustmentKey) => void
  onCurveSelect: (id: string) => void; onCurveBegin: () => void; onCurveChange: (points: ToneCurvePoint[]) => void
  onCurvePresetSave: () => void; onCurvePresetLoad: () => void; canLoadCurvePreset: boolean
  onMaskBegin: () => void; onMaskChange: (mask: RadialMask) => void
  whiteBalanceMode: NativeWhiteBalanceMode; onWhiteBalanceMode: (mode: NativeWhiteBalanceMode) => void
  onCopyWhiteBalance: () => void; onPasteWhiteBalance: () => void
  mixerBand: string; onMixerBand: (band: string) => void; mixerPicking: boolean; onMixerPicking: () => void
}) {
  const mixerBands = ['Red', 'Orange', 'Yellow', 'Green', 'Cyan', 'Blue', 'Purple', 'Magenta'] as const
  const sliders = sliderGroups[tool] ?? []
  const normalizeAngle = (value: number) => ((value + 180) % 360 + 360) % 360 - 180
  return <section className="inspector-content" aria-label={`${tool} inspector`}>
    <div className="inspector-head"><div><span className="eyebrow">Live CPU preview</span><h2>{tool}</h2></div><ChevronDown size={16} /></div>
    {renderBackend === 'native' && ['masks', 'optics', 'geometry'].includes(tool)
      && <div className="tool-note">This tool is outside the M1C Native slice. Applying it raises an explicit error; Starroom will not silently use Browser Canvas.</div>}
    {tool === 'color' && <>
      <div className="tool-note">Encoded-image Temperature/Tint are relative corrections, not physical Kelvin. RAW Camera/As-Shot uses LibRaw metadata.</div>
      {renderBackend === 'native' && <div className="wb-controls"><label>White balance mode<select value={whiteBalanceMode}
        onFocus={onBeginAdjustment} onChange={(event) => onWhiteBalanceMode(event.target.value as NativeWhiteBalanceMode)}>
        <option value="sourceDefault">Source default</option><option value="asShot">As Shot (RAW)</option>
        <option value="camera">Camera (RAW)</option><option value="auto">Auto (gray world)</option>
        <option value="neutralPicker">Neutral picker</option><option value="relative">Relative (encoded)</option>
      </select></label><div><button onClick={onCopyWhiteBalance}>Copy WB</button><button onClick={onPasteWhiteBalance}>Paste WB</button></div>
      <small>{whiteBalanceMode === 'neutralPicker' ? 'Double-click a neutral area in the preview to sample it.' : 'Mode is recorded with this non-destructive edit.'}</small></div>}
      <div className="mixer-panel" aria-label="Eight-band Color Mixer">
        <div className="mixer-heading"><strong>Color Mixer</strong><button className={mixerPicking ? 'active' : ''} onClick={onMixerPicking}>Target</button><label><input type="checkbox" checked={values.mixerHueLock !== 0}
          onFocus={onBeginAdjustment} onChange={(event) => onAdjust('mixerHueLock', event.target.checked ? 1 : 0)} /> Hue lock</label></div>
        <div className="mixer-tabs" role="tablist" aria-label="Color Mixer bands">
          {mixerBands.map((band) => <button key={band} role="tab" aria-selected={band === mixerBand}
            className={band === mixerBand ? `active band-${band.toLowerCase()}` : `band-${band.toLowerCase()}`}
            onClick={() => onMixerBand(band)}>{band}</button>)}
        </div>
        {([['Hue', -30, 30, 1, '°'], ['Chroma', -100, 100, 1, ''], ['Lightness', -100, 100, 1, '']] as const)
          .map(([control, min, max, step, suffix]) => {
            const key = `mixer${mixerBand}${control}` as AdjustmentKey
            return <Slider key={key} label={`${mixerBand} ${control}`} value={values[key]} min={min} max={max} step={step} suffix={suffix}
              onBeginEdit={onBeginAdjustment} onChange={(value) => onAdjust(key, value, false)} onReset={() => onReset(key)} />
          })}
        <small>Targeted edits are calculated in native OKLCh with circular, overlapping hue bands.</small>
      </div>
    </>}
    {tool === 'masks' && <div className="tool-note">Click the photo to place the mask. Drag inside to move; drag side handles to resize; drag the top handle to rotate.</div>}
    {tool === 'curve' && <><CurveChannelTabs value={curveChannel} onChange={onCurveChannel} /><ToneCurveEditor points={curvePoints} selectedId={selectedCurvePoint} onSelect={onCurveSelect}
      histogram={histogram} onBeginEdit={onCurveBegin} onChange={onCurveChange} /><div className="curve-presets"><button onClick={onCurvePresetSave}>Save custom</button><button disabled={!canLoadCurvePreset} onClick={onCurvePresetLoad}>Load custom</button></div></>}
    {sliders.map(({ key, ...slider }) => <Slider key={key} {...slider} value={values[key]} onBeginEdit={onBeginAdjustment}
      onChange={(value) => onAdjust(key, value, false)} onReset={() => onReset(key)} />)}
    {tool === 'masks' && <div className="mask-values">
      {([
        ['Center X', 'x', mask.x * 100, 0, 100, '%'], ['Center Y', 'y', mask.y * 100, 0, 100, '%'],
        ['Width', 'width', mask.width * 100, 4, 160, '%'], ['Height', 'height', mask.height * 100, 4, 160, '%'],
        ['Angle', 'rotation', mask.rotation, -180, 180, '°'],
      ] as const).map(([label, key, value, min, max, suffix]) => <label key={key}>{label}<span><input aria-label={`Mask ${label} value`} type="number"
        min={min} max={max} step={key === 'rotation' ? .1 : 1} value={Math.round(value * 10) / 10}
        onFocus={onMaskBegin} onChange={(event) => {
          const next = Math.min(max, Math.max(min, Number(event.target.value)))
          onMaskChange({ ...mask, [key]: key === 'rotation' ? next : next / 100 })
        }} />{suffix}</span></label>)}
    </div>}
    {tool === 'geometry' && <div className="geometry-controls">
      <button onClick={() => onAdjust('rotation', normalizeAngle(values.rotation - 90))}><RotateCcw size={16} /> Rotate left</button>
      <button onClick={() => onAdjust('rotation', normalizeAngle(values.rotation + 90))}><RotateCw size={16} /> Rotate right</button>
      <button className={values.flipHorizontal ? 'active' : ''} onClick={() => onAdjust('flipHorizontal', values.flipHorizontal ? 0 : 1)}><FlipHorizontal2 size={16} /> Flip horizontal</button>
      <button className={values.flipVertical ? 'active' : ''} onClick={() => onAdjust('flipVertical', values.flipVertical ? 0 : 1)}><FlipVertical2 size={16} /> Flip vertical</button>
    </div>}
    <div className="intent-card"><Sparkles size={17} /><div><strong>Non-destructive edits</strong><span>Type values directly or double-click a slider to reset</span></div></div>
  </section>
}

function AppHeader({ view, setView, theme, setTheme, before, setBefore, canUndo, canRedo, undo, redo, onExport }: {
  view: WorkspaceView; setView: (view: WorkspaceView) => void
  theme: Theme; setTheme: (theme: Theme) => void
  before: boolean; setBefore: (value: boolean) => void; canUndo: boolean; canRedo: boolean
  undo: () => void; redo: () => void; onExport: () => void
}) {
  return <header className="topbar">
    <div className="brand"><span className="brand-mark"><Aperture size={18} /></span><strong>Starroom</strong></div>
    <nav aria-label="Workspace">
      <button className={view === 'library' ? 'active' : ''} onClick={() => setView('library')}>Library</button>
      <button className={view === 'edit' ? 'active' : ''} onClick={() => setView('edit')}>Edit</button>
      <button className={view === 'compare' ? 'active' : ''} onClick={() => setView('compare')}>Compare</button>
    </nav>
    <div className="top-actions">
      <button className={before ? 'text-button active' : 'text-button'} onClick={() => setBefore(!before)}><Columns2 size={15} /> Before</button>
      <IconButton label="Undo" disabled={!canUndo} onClick={undo}><Undo2 size={17} /></IconButton>
      <IconButton label="Redo" disabled={!canRedo} onClick={redo}><Redo2 size={17} /></IconButton>
      <select aria-label="Theme" value={theme} onChange={(event) => setTheme(event.target.value as Theme)}><option value="dark">Dark</option><option value="gray">Gray</option><option value="light">Light</option></select>
      <button className="export-button" onClick={onExport}><Download size={15} /> Export JPEG</button>
    </div>
  </header>
}

export function App() {
  const [theme, setTheme] = usePersistedValue<Theme>('starroom-theme', 'dark')
  const [leftOpen, setLeftOpen] = usePersistedValue('starroom-left-panel', true)
  const [filmstripOpen, setFilmstripOpen] = usePersistedValue('starroom-filmstrip', true)
  const [photos, setPhotos] = useState<PhotoItem[]>([demoPhoto])
  const [selectedId, setSelectedId] = useState(demoPhoto.id)
  const [filter, setFilter] = useState<LibraryFilter>('all')
  const [view, setView] = useState<WorkspaceView>('edit')
  const [tool, setTool] = useState<Tool>('light')
  const [selectedCurvePoint, setSelectedCurvePoint] = useState<string | null>('midtone')
  const [curveChannel, setCurveChannel] = useState<keyof NativeToneCurves>('master')
  const [before, setBefore] = useState(false)
  const [zoom, setZoom] = useState<'fit' | '100'>('fit')
  const [zoomScale, setZoomScale] = useState(1)
  const [pan, setPan] = useState({ x: 0, y: 0 })
  const panStart = useRef<{ x: number; y: number; panX: number; panY: number } | null>(null)
  const [histogram, setHistogram] = useState(() => Array.from({ length: 48 }, () => 0))
  const [renderStatus, setRenderStatus] = useState('Ready')
  const [dimensions, setDimensions] = useState('—')
  const [notice, setNotice] = useState('')
  const [copiedWhiteBalance, setCopiedWhiteBalance] = useState<Pick<PhotoItem, 'whiteBalanceMode' | 'whiteBalanceSample'> | null>(null)
  const [savedCurvePreset, setSavedCurvePreset] = usePersistedValue<NativeToneCurves | null>('starroom-custom-curve-preset', null)
  const [mixerBand, setMixerBand] = useState('Red')
  const [mixerPicking, setMixerPicking] = useState(false)
  const fileInput = useRef<HTMLInputElement>(null)
  const objectUrls = useRef(new Set<string>())

  useEffect(() => () => objectUrls.current.forEach((url) => URL.revokeObjectURL(url)), [])
  useEffect(() => {
    if (!notice) return
    const timeout = window.setTimeout(() => setNotice(''), 3500)
    return () => window.clearTimeout(timeout)
  }, [notice])

  function selectPhoto(id: string) {
    setSelectedId(id)
    setZoom('fit')
    setZoomScale(1)
    setPan({ x: 0, y: 0 })
  }

  const selected = photos.find((photo) => photo.id === selectedId) ?? photos[0]
  const filteredPhotos = useMemo(() => photos.filter((photo) => {
    if (filter === 'recent') return photo.imported
    if (filter === 'five-star') return photo.rating === 5
    if (filter === 'edited') return hasPhotoEdits(photo)
    return true
  }), [filter, photos])

  const counts = useMemo(() => ({
    all: photos.length,
    recent: photos.filter((photo) => photo.imported).length,
    five: photos.filter((photo) => photo.rating === 5).length,
    edited: photos.filter(hasPhotoEdits).length,
  }), [photos])

  function chooseFilter(next: LibraryFilter) {
    setFilter(next)
    const first = photos.find((photo) => next === 'all' || (next === 'recent' && photo.imported) || (next === 'five-star' && photo.rating === 5) || (next === 'edited' && hasPhotoEdits(photo)))
    if (first) selectPhoto(first.id)
  }

  function importPhotos(files: FileList | null) {
    if (!files?.length) return
    const supported = [...files].filter((file) => file.type.startsWith('image/'))
    if (!supported.length) {
      setNotice('No browser-readable images were selected. Use JPEG, PNG or WebP.')
      return
    }
    const imported = supported.map<PhotoItem>((file) => {
      const src = URL.createObjectURL(file)
      objectUrls.current.add(src)
      return { id: crypto.randomUUID(), name: file.name, src, renderBackend: 'browserFallback', imported: true, rating: 0,
        adjustments: { ...defaultAdjustments }, curvePoints: copyCurve(defaultCurvePoints), curveChannels: defaultCurveChannels(), whiteBalanceMode: 'sourceDefault', whiteBalanceSample: null, mask: { ...defaultMask }, history: [], future: [] }
    })
    setPhotos((current) => [...imported, ...current])
    selectPhoto(imported[0].id)
    setFilter('all')
    setView('edit')
    setBefore(false)
    setNotice(`${imported.length} photo${imported.length === 1 ? '' : 's'} imported`)
  }

  async function requestPhotoImport() {
    if (!nativeRuntimeAvailable()) {
      fileInput.current?.click()
      return
    }
    try {
      const paths = await chooseNativePhotoPaths()
      if (!paths.length) return
      const imported = paths.map<PhotoItem>((sourcePath) => ({
        id: crypto.randomUUID(),
        name: sourcePath.split(/[\\/]/).at(-1) ?? sourcePath,
        src: nativeThumbnailUrl(sourcePath),
        sourcePath,
        renderBackend: 'native',
        imported: true,
        rating: 0,
        adjustments: { ...defaultAdjustments },
        curvePoints: copyCurve(defaultCurvePoints),
        curveChannels: defaultCurveChannels(),
        whiteBalanceMode: 'sourceDefault',
        whiteBalanceSample: null,
        mask: { ...defaultMask },
        history: [],
        future: [],
      }))
      setPhotos((current) => [...imported, ...current])
      selectPhoto(imported[0].id)
      setFilter('all')
      setView('edit')
      setBefore(false)
      setNotice(`${imported.length} photo${imported.length === 1 ? '' : 's'} imported into Native preview`)
    } catch (error) {
      setNotice(error instanceof Error ? error.message : 'Native photo picker failed')
    }
  }

  function updateSelected(mutator: (photo: PhotoItem) => PhotoItem) {
    setPhotos((current) => current.map((photo) => photo.id === selected.id ? mutator(photo) : photo))
  }

  function removePhoto(id: string) {
    if (photos.length <= 1) {
      setNotice('Keep at least one photo in the workspace')
      return
    }
    const removed = photos.find((photo) => photo.id === id)
    const remaining = photos.filter((photo) => photo.id !== id)
    setPhotos(remaining)
    if (selectedId === id) selectPhoto(remaining[0].id)
    if (removed?.src.startsWith('blob:')) {
      URL.revokeObjectURL(removed.src)
      objectUrls.current.delete(removed.src)
    }
    setBefore(false)
    setNotice(`${removed?.name ?? 'Photo'} removed from Starroom · source file was not deleted`)
  }

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      const target = event.target as HTMLElement | null
      if (event.key !== 'Delete' || target?.matches('input, select, textarea')) return
      removePhoto(selectedId)
    }
    window.addEventListener('keydown', onKeyDown)
    return () => window.removeEventListener('keydown', onKeyDown)
  })

  function adjust(key: AdjustmentKey, value: number, recordHistory = true) {
    updateSelected((photo) => {
      if (photo.adjustments[key] === value) return photo
      return {
        ...photo,
        adjustments: { ...photo.adjustments, [key]: value },
        history: recordHistory ? [...photo.history, takeSnapshot(photo)].slice(-100) : photo.history,
        future: [],
      }
    })
    setBefore(false)
  }

  function beginInteractiveEdit() {
    updateSelected((photo) => ({ ...photo, history: [...photo.history, takeSnapshot(photo)].slice(-100), future: [] }))
  }

  function updateCurve(points: ToneCurvePoint[]) {
    updateSelected((photo) => ({ ...photo, curvePoints: curveChannel === 'master' ? copyCurve(points) : photo.curvePoints,
      curveChannels: { ...photo.curveChannels, [curveChannel]: copyCurve(points) }, future: [] }))
    setBefore(false)
  }

  function saveCurvePreset() {
    setSavedCurvePreset(copyCurveChannels(selected.curveChannels))
    setNotice('Custom curve preset saved')
  }

  function loadCurvePreset() {
    if (!savedCurvePreset) return
    updateSelected((photo) => ({ ...photo, curvePoints: copyCurve(savedCurvePreset.master), curveChannels: copyCurveChannels(savedCurvePreset),
      history: [...photo.history, takeSnapshot(photo)].slice(-100), future: [] }))
    setBefore(false)
    setNotice('Custom curve preset loaded')
  }

  function updateWhiteBalance(mode: NativeWhiteBalanceMode, sample: NativeWhiteBalanceSample | null = null) {
    updateSelected((photo) => ({ ...photo, whiteBalanceMode: mode, whiteBalanceSample: sample,
      history: [...photo.history, takeSnapshot(photo)].slice(-100), future: [] }))
    setBefore(false)
  }

  function copyWhiteBalance() {
    setCopiedWhiteBalance({ whiteBalanceMode: selected.whiteBalanceMode,
      whiteBalanceSample: selected.whiteBalanceSample ? { ...selected.whiteBalanceSample } : null })
    setNotice('White balance copied')
  }

  function pasteWhiteBalance() {
    if (!copiedWhiteBalance) { setNotice('Copy a white balance first'); return }
    updateWhiteBalance(copiedWhiteBalance.whiteBalanceMode,
      copiedWhiteBalance.whiteBalanceSample ? { ...copiedWhiteBalance.whiteBalanceSample } : null)
    setNotice('White balance pasted')
  }

  async function pickMixerBand(x: number, y: number) {
    if (!selected.sourcePath || selected.renderBackend !== 'native') {
      setNotice('Color Mixer targeting requires a Native photo; no Browser color fallback was used.')
      return
    }
    try {
      const band = await sampleNativeColor(selected.sourcePath, x, y, selected.adjustments, selected.curvePoints,
        selected.whiteBalanceMode, selected.whiteBalanceSample, selected.curveChannels)
      if (!band) { setNotice('The sampled area is neutral; no color band was selected.'); return }
      setMixerBand(`${band[0].toUpperCase()}${band.slice(1)}`)
      setMixerPicking(false)
      setNotice(`${band} band selected from Native working color`)
    } catch (error) {
      setNotice(error instanceof Error ? error.message : 'Native Color Mixer sampling failed')
    }
  }

  function updateMask(mask: RadialMask) {
    updateSelected((photo) => ({ ...photo, mask: { ...mask }, future: [] }))
    setBefore(false)
  }

  function resetAdjustment(key: AdjustmentKey) {
    adjust(key, defaultAdjustments[key])
  }

  function undo() {
    updateSelected((photo) => {
      const previous = photo.history.at(-1)
      if (!previous) return photo
      return { ...applySnapshot(photo, previous), history: photo.history.slice(0, -1), future: [takeSnapshot(photo), ...photo.future] }
    })
  }

  function redo() {
    updateSelected((photo) => {
      const next = photo.future[0]
      if (!next) return photo
      return { ...applySnapshot(photo, next), history: [...photo.history, takeSnapshot(photo)], future: photo.future.slice(1) }
    })
  }

  function toggleRating() {
    updateSelected((photo) => ({ ...photo, rating: photo.rating === 5 ? 0 : 5 }))
  }

  function resetAll() {
    if (!hasPhotoEdits(selected)) return
    updateSelected((photo) => ({ ...photo, adjustments: { ...defaultAdjustments }, curvePoints: copyCurve(defaultCurvePoints), curveChannels: defaultCurveChannels(), whiteBalanceMode: 'sourceDefault', whiteBalanceSample: null, mask: { ...defaultMask },
      history: [...photo.history, takeSnapshot(photo)], future: [] }))
  }

  async function exportJpeg() {
    setRenderStatus(selected.renderBackend === 'native' ? 'Native full-resolution export…' : 'Browser fallback export…')
    try {
      if (selected.renderBackend === 'native') {
        if (!selected.sourcePath) throw new Error('Native photo is missing its source path.')
        const outputPath = await chooseNativeExportPath(selected.name)
        if (!outputPath) {
          setRenderStatus('Export cancelled')
          return
        }
        const result = await exportNativeJpeg(selected.sourcePath, outputPath, selected.adjustments, selected.curvePoints, selected.mask,
          selected.whiteBalanceMode, selected.whiteBalanceSample, selected.curveChannels)
        setNotice(`Native JPEG exported · ${result.width} × ${result.height} · ${result.inputProfile}`)
        setRenderStatus(`Native CPU · ${result.workingSpace}`)
        return
      }
      const canvas = await renderImageSource(selected.src, selected.adjustments, Number.POSITIVE_INFINITY, selected.curvePoints, selected.mask)
      const blob = await new Promise<Blob>((resolve, reject) => canvas.toBlob((value) => value ? resolve(value) : reject(new Error('JPEG encoding failed.')), 'image/jpeg', .94))
      const url = URL.createObjectURL(blob)
      const anchor = document.createElement('a')
      const base = selected.name.replace(/\.[^.]+$/, '')
      anchor.href = url
      anchor.download = `${base}-starroom.jpg`
      anchor.click()
      window.setTimeout(() => URL.revokeObjectURL(url), 1000)
      setNotice('Browser fallback JPEG exported without overwriting the original')
      setRenderStatus('Browser fallback preview')
    } catch (error) {
      setNotice(error instanceof Error ? error.message : 'Export failed')
      setRenderStatus('Export failed')
    }
  }

  return <main className={`app theme-${theme}`} data-theme={theme}>
    <AppHeader view={view} setView={(next) => { setView(next); setBefore(false) }} theme={theme} setTheme={setTheme} before={before} setBefore={setBefore}
      canUndo={selected.history.length > 0} canRedo={selected.future.length > 0} undo={undo} redo={redo} onExport={exportJpeg} />
    <div className={`workspace view-${view} ${leftOpen ? '' : 'left-collapsed'} ${filmstripOpen ? '' : 'filmstrip-collapsed'}`}>
      <aside className="library-panel">
        <div className="panel-title"><span>Library</span><IconButton label="Collapse library" onClick={() => setLeftOpen(false)}><PanelLeftClose size={17} /></IconButton></div>
        <button className="import-button" onClick={requestPhotoImport}><ImagePlus size={16} /> Add photos</button>
        <input ref={fileInput} type="file" accept="image/jpeg,image/png,image/webp,image/svg+xml" multiple hidden onChange={(event) => { importPhotos(event.target.files); event.target.value = '' }} />
        <span className="format-note">Native: JPEG · PNG · TIFF · NEF · ARW · CR2/CR3 · DNG · RAF</span>
        <div className="library-group"><span className="eyebrow">Workspace</span>
          <button className={`library-item ${filter === 'all' ? 'selected' : ''}`} onClick={() => chooseFilter('all')}><Grid2X2 size={16} /> All Photos <small>{counts.all}</small></button>
          <button className={`library-item ${filter === 'recent' ? 'selected' : ''}`} onClick={() => chooseFilter('recent')}><Folder size={16} /> Recent Imports <small>{counts.recent}</small></button>
        </div>
        <div className="library-group"><span className="eyebrow">Smart albums</span>
          <button className={`library-item ${filter === 'five-star' ? 'selected' : ''}`} onClick={() => chooseFilter('five-star')}><Star size={16} /> Five Stars <small>{counts.five}</small></button>
          <button className={`library-item ${filter === 'edited' ? 'selected' : ''}`} onClick={() => chooseFilter('edited')}><Contrast size={16} /> Edited <small>{counts.edited}</small></button>
        </div>
        <div className="library-summary"><Library size={15} /><span>{filteredPhotos.length} visible photos</span></div>
      </aside>
      {!leftOpen && <button className="edge-toggle left" aria-label="Open library" onClick={() => setLeftOpen(true)}><PanelLeftOpen size={17} /></button>}

      {view === 'library' ? <section className="library-browser" aria-label="Photo library">
        <div className="library-browser-head"><div><span className="eyebrow">Photo workspace</span><h1>{filteredPhotos.length} photos</h1></div>
          <button className="import-button compact" onClick={requestPhotoImport}><ImagePlus size={16} /> Add photos</button></div>
        <div className="photo-grid">{filteredPhotos.map((photo) => <article key={photo.id} className={photo.id === selected.id ? 'photo-card selected' : 'photo-card'}>
          <button className="photo-card-preview" onClick={() => { selectPhoto(photo.id); setView('edit'); setBefore(false) }} title={`Edit ${photo.name}`}>
            <img src={photo.src} alt={photo.name} />
          </button>
          <div><span title={photo.name}>{photo.name}</span><small>{photo.renderBackend === 'native' ? 'Native CPU' : 'Browser fallback'} · {hasPhotoEdits(photo) ? 'Edited' : 'Original'}</small></div>
          <button className="card-delete" aria-label={`Remove ${photo.name}`} title="Remove from Starroom (source stays on disk)" disabled={photos.length <= 1} onClick={() => removePhoto(photo.id)}><Trash2 size={15} /></button>
        </article>)}</div>
      </section> : <section className="canvas-area">
          <div className="canvas-toolbar"><span>{selected.name}</span><div>
            <button className={selected.rating === 5 ? 'active rating-button' : 'rating-button'} onClick={toggleRating} title="Toggle five-star rating"><Star size={12} fill={selected.rating === 5 ? 'currentColor' : 'none'} /> {selected.rating === 5 ? '5★' : 'Rate'}</button>
            <button className="remove-selected" disabled={photos.length <= 1} onClick={() => removePhoto(selected.id)} title="Remove from Starroom; does not delete source"><Trash2 size={12} /> Remove</button>
            <button className={zoom === 'fit' && zoomScale === 1 ? 'active' : ''} onClick={() => { setZoom('fit'); setZoomScale(1); setPan({ x: 0, y: 0 }) }}>Fit</button>
            <button className={zoom === '100' ? 'active' : ''} onClick={() => { setZoom('100'); setZoomScale(1); setPan({ x: 0, y: 0 }) }}>100%</button>
          </div></div>
          {view === 'compare' ? <div className="compare-stage">
            <div className="compare-pane"><PreviewCanvas photo={selected} before zoom={zoom} metric={false} onHistogram={setHistogram} onStatus={setRenderStatus} onDimensions={setDimensions} /><span>Original</span></div>
            <div className="compare-pane"><PreviewCanvas photo={selected} before={false} zoom={zoom} onHistogram={setHistogram} onStatus={setRenderStatus} onDimensions={setDimensions} /><span>Edited</span></div>
          </div> : <div className={`photo-stage ${before ? 'show-before' : ''} zoom-stage-${zoom} ${zoomScale > 1 ? 'is-zoomed' : ''} ${tool === 'masks' ? 'mask-mode' : ''}`}
            onWheel={(event) => {
              const next = Math.max(.25, Math.min(6, zoomScale * Math.exp(-event.deltaY * .0015)))
              setZoom('fit')
              setZoomScale(next)
              if (next <= 1) setPan({ x: 0, y: 0 })
            }}
            onPointerDown={(event) => {
              if (tool === 'masks' || zoomScale <= 1 || event.button !== 0) return
              panStart.current = { x: event.clientX, y: event.clientY, panX: pan.x, panY: pan.y }
              event.currentTarget.setPointerCapture(event.pointerId)
            }}
            onPointerMove={(event) => {
              if (!panStart.current) return
              setPan({ x: panStart.current.panX + event.clientX - panStart.current.x, y: panStart.current.panY + event.clientY - panStart.current.y })
            }}
            onPointerUp={(event) => { panStart.current = null; if (event.currentTarget.hasPointerCapture(event.pointerId)) event.currentTarget.releasePointerCapture(event.pointerId) }}>
            <div className="photo-frame" style={{ transform: `translate(${pan.x}px, ${pan.y}px) scale(${zoomScale})` }}>
              <PreviewCanvas photo={selected} before={before} zoom={zoom} maskActive={tool === 'masks' && !before}
                onBeginMaskEdit={beginInteractiveEdit} onMaskChange={updateMask}
                onWhiteBalancePick={(sample) => updateWhiteBalance('neutralPicker', sample)}
                onColorSample={tool === 'color' && mixerPicking ? pickMixerBand : undefined}
                onHistogram={setHistogram} onStatus={setRenderStatus} onDimensions={setDimensions} />
              <span className="preview-badge">{selected.renderBackend === 'native' ? 'Native CPU' : 'Browser fallback'} · {before ? 'Original' : hasPhotoEdits(selected) ? `${countPhotoEdits(selected)} edits` : 'Original'}</span>
            </div>
          </div>}
          <div className="canvas-footer"><span>{zoomScale !== 1 ? `${Math.round(zoomScale * 100)}%` : zoom === 'fit' ? 'Fit' : '100%'}</span><span className="status-dot" /><span>{renderStatus}</span><span>· {dimensions}</span>
            <span className="zoom-help"><Move size={12} /> Wheel to zoom · drag to pan</span>
            <button aria-label="Toggle filmstrip" onClick={() => setFilmstripOpen(!filmstripOpen)}>{filmstripOpen ? <PanelBottomClose size={16} /> : <PanelBottomOpen size={16} />}</button></div>
          <div className="filmstrip" aria-label="Filmstrip">
            {filteredPhotos.length ? filteredPhotos.map((photo) => <div key={photo.id} className="thumb-shell">
              <button className={photo.id === selected.id ? 'thumb active' : 'thumb'} onClick={() => { selectPhoto(photo.id); setBefore(false) }} title={photo.name}>
                <img src={photo.src} alt={photo.name} /><span>{photo.rating === 5 ? '★' : hasPhotoEdits(photo) ? 'E' : ''}</span>
              </button>
              <button className="thumb-delete" aria-label={`Remove ${photo.name}`} title="Remove from Starroom (source stays on disk)" disabled={photos.length <= 1} onClick={() => removePhoto(photo.id)}><Trash2 size={13} /></button>
            </div>) : <div className="empty-filmstrip">No photos match this album.</div>}
          </div>
        </section>}

      <aside className="inspector-panel">
        <div className="histogram-wrap"><Histogram values={histogram} /><div><span>LIVE</span><span>{dimensions}</span><span>CPU</span></div></div>
        <div className="tool-layout">
          <nav className="tool-rail" aria-label="Editing tools">{toolItems.map(({ id, label, icon: Icon }) => <button key={id}
            className={tool === id ? 'active' : ''} aria-label={label}
            title={label} onClick={() => setTool(id)}><Icon size={18} /><span>{label}</span></button>)}</nav>
          <Inspector tool={tool} values={selected.adjustments} curvePoints={selected.curveChannels[curveChannel]} curveChannel={curveChannel} histogram={histogram} onCurveChannel={(channel) => { setCurveChannel(channel); setSelectedCurvePoint(null) }} selectedCurvePoint={selectedCurvePoint} renderBackend={selected.renderBackend}
            whiteBalanceMode={selected.whiteBalanceMode}
            mask={selected.mask} onAdjust={adjust} onBeginAdjustment={beginInteractiveEdit} onReset={resetAdjustment} onCurveSelect={setSelectedCurvePoint}
            onCurveBegin={beginInteractiveEdit} onCurveChange={updateCurve} onMaskBegin={beginInteractiveEdit} onMaskChange={updateMask}
            onCurvePresetSave={saveCurvePreset} onCurvePresetLoad={loadCurvePreset} canLoadCurvePreset={savedCurvePreset !== null}
            onWhiteBalanceMode={(mode) => updateWhiteBalance(mode)} onCopyWhiteBalance={copyWhiteBalance} onPasteWhiteBalance={pasteWhiteBalance}
            mixerBand={mixerBand} onMixerBand={setMixerBand} mixerPicking={mixerPicking} onMixerPicking={() => setMixerPicking(!mixerPicking)} />
        </div>
        <button className="reset-all" disabled={!hasPhotoEdits(selected)} onClick={resetAll}><RotateCcw size={14} /> Reset all edits</button>
      </aside>
    </div>
    {notice && <button className="notice" onClick={() => setNotice('')} aria-label="Dismiss notice">{notice}</button>}
    <div className="compact-warning"><SunMedium size={18} /><span>Starroom needs a wider window for the full editing workspace.</span></div>
  </main>
}
