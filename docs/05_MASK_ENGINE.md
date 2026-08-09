# Starroom V2 — Mask Engine Specification

## 1. Core Model
Each mask resolves to a weight map `M(x,y) in [0,1]`: 0 means no effect, 1 means full local effect, intermediate values feather influence.

## 2. Local Composition
Given original pixel O, adjusted pixel A, and mask M: `Result = O * (1 - M) + A * M`. Adjustment math is independent from mask generation.

## 3. Mask Component Types
Brush: points, radius, feather, flow, density, erase mode.
Linear Gradient: start/end, feather width.
Radial Gradient: center, radii, rotation, feather, invert.
Luminance Range: min/max plus low/high falloff.
Color Range: sampled colors, hue/chroma/lightness tolerance, softness.

## 4. Composition
Add: `A + B - A*B`
Subtract: `A * (1-B)`
Intersect: `A * B`
All results stay in [0,1].

## 5. Mask Tree
Example:
```text
Portrait Light
├─ Person
├─ Subtract Brush
└─ Intersect Luminance Range
```
AI nodes use the same tree interface.

## 6. Local Adjustments
Exposure, Contrast, Highlights, Shadows, Whites, Blacks, Temperature, Tint, Vibrance, Saturation, selective color, Texture, Clarity, Dehaze and future local detail controls.

## 7. Overlay
Red overlay, White on Black, Black on White, Grayscale. Shortcut target: `O` toggles overlay.

## 8. Performance
Masks cache independently from local adjustment settings. Changing local Exposure must not recompute brush geometry. Changing brush geometry invalidates mask raster cache, not unrelated global stages.

## 9. AI Compatibility
`MaskProvider` generates `MaskRaster + metadata` for Subject, Background, Sky, Person and supported Face/Skin/Hair/Clothing requests. AI masks remain editable through brush add/subtract.

## 10. Persistence
AI nodes persist model fingerprint, request, source hash, frozen raster cache and manual refinements so project appearance survives model changes.
