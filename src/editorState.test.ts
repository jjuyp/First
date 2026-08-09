import { describe, expect, it } from 'vitest'
import { editorReducer, initialEditorState } from './editorState'

describe('editorReducer', () => {
  it('stores reversible committed adjustments', () => {
    const changed = editorReducer(initialEditorState, {
      type: 'adjust', key: 'exposure', value: 1.25, commit: true,
    })
    expect(changed.adjustments.exposure).toBe(1.25)
    expect(editorReducer(changed, { type: 'undo' }).adjustments.exposure).toBe(0)
  })

  it('resets a parameter without touching source pixels', () => {
    const changed = editorReducer(initialEditorState, {
      type: 'adjust', key: 'contrast', value: 30,
    })
    expect(editorReducer(changed, { type: 'reset', key: 'contrast' }).adjustments.contrast).toBe(0)
  })
})
