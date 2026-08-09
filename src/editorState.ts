export type EditorMode = 'simple' | 'pro'
export type Theme = 'dark' | 'gray' | 'light'
export type Tool = 'light' | 'color' | 'curve' | 'detail' | 'masks' | 'optics' | 'geometry'

export type AdjustmentKey =
  | 'exposure'
  | 'contrast'
  | 'highlights'
  | 'shadows'
  | 'whites'
  | 'blacks'
  | 'temperature'
  | 'tint'
  | 'vibrance'
  | 'saturation'
  | 'sharpness'
  | 'clarity'
  | 'noiseReduction'
  | 'maskExposure'
  | 'maskFeather'
  | 'vignette'
  | 'lensBrightness'
  | 'rotation'
  | 'flipHorizontal'
  | 'flipVertical'

export type Adjustments = Record<AdjustmentKey, number>

export const defaultAdjustments: Adjustments = {
  exposure: 0,
  contrast: 0,
  highlights: 0,
  shadows: 0,
  whites: 0,
  blacks: 0,
  temperature: 6500,
  tint: 0,
  vibrance: 0,
  saturation: 0,
  sharpness: 0,
  clarity: 0,
  noiseReduction: 0,
  maskExposure: 0,
  maskFeather: 50,
  vignette: 0,
  lensBrightness: 0,
  rotation: 0,
  flipHorizontal: 0,
  flipVertical: 0,
}

export interface EditorSnapshot {
  adjustments: Adjustments
}

export interface EditorState extends EditorSnapshot {
  history: EditorSnapshot[]
  future: EditorSnapshot[]
}

export const initialEditorState: EditorState = {
  adjustments: defaultAdjustments,
  history: [],
  future: [],
}

export type EditorAction =
  | { type: 'adjust'; key: AdjustmentKey; value: number; commit?: boolean }
  | { type: 'reset'; key: AdjustmentKey }
  | { type: 'undo' }
  | { type: 'redo' }

export function editorReducer(state: EditorState, action: EditorAction): EditorState {
  if (action.type === 'undo') {
    const previous = state.history.at(-1)
    if (!previous) return state
    return {
      adjustments: previous.adjustments,
      history: state.history.slice(0, -1),
      future: [{ adjustments: state.adjustments }, ...state.future],
    }
  }

  if (action.type === 'redo') {
    const next = state.future[0]
    if (!next) return state
    return {
      adjustments: next.adjustments,
      history: [...state.history, { adjustments: state.adjustments }],
      future: state.future.slice(1),
    }
  }

  const key = action.key
  const value = action.type === 'reset' ? defaultAdjustments[key] : action.value
  if (state.adjustments[key] === value) return state

  return {
    adjustments: { ...state.adjustments, [key]: value },
    history: action.type === 'reset' || action.commit
      ? [...state.history, { adjustments: state.adjustments }].slice(-100)
      : state.history,
    future: [],
  }
}
