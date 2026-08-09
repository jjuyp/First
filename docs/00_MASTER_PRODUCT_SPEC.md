# 00 — Master Product Specification

## Promise
Professional editing power without making technical knowledge a prerequisite. Simple Mode expresses intent; Pro Mode exposes the same underlying typed parameters.

## Public 1.0
Files: JPEG, PNG, TIFF, DNG, NEF, RAF, CR3, ARW. Global: tone, WB, Master/RGB curves, HSL, grading, vibrance/saturation, Texture/Clarity/Dehaze, sharpen, classic NR, AI Denoise. Optics/geometry: crop, rotate, straighten, perspective, distortion, CA, vignette. Masks: manual ranges/gradients/brush plus AI Subject/Background/Sky/Person and supported person submasks. Workflow: undo/history/snapshot, compare, presets with Amount, copy/paste/sync, batch export, EXIF/rating/flag. Differentiators: Skin-aware Color, Look Engine, Style Mixer, Reference Match, Beginner/Pro.

## Architecture
```text
Project/Workflow
      |
RAW/RGB -> Render Graph -> wgpu/DX12 -> color-managed preview/export
      |          |
      |        Masks
      |
AI Orchestrator -> Windows ML / ONNX -> CPU/GPU/NPU
```

No cloud dependency is required for core editing.
