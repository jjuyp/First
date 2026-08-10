# Starroom v0.2 — Core Quality Update

## Goal
Turn the v0.1 browser vertical slice into a trustworthy editing baseline before RAW/GPU/AI expansion. v0.2 prioritizes image-quality correctness, explainable state, reusable layer/mask architecture, and regression protection.

## Open-source strategy
Starroom v0.2 is currently a private/personal-use build.

- darktable (GPL-3.0): direct study, adaptation and porting of useful implementations is allowed for this private-use build. Every directly derived component must be clearly marked `GPL-derived / private-use` with source file, upstream revision and license note so a future distribution decision is explicit rather than accidental.
- Oklab/OKLCh: use published color-science equations and Starroom-native Rust/wgpu implementations for hue-locked selective editing.
- Lensfun: integrate the library/database as the optics provider instead of rebuilding the lens database.
- colour-science: use as a development/validation oracle where useful; production advisor logic remains native Rust and offline.
- MediaPipe: integrate behind `FaceLandmarkProvider`; the Starroom core stays provider-neutral so face runtime can be replaced later.

If Starroom is ever distributed outside private use, perform a complete GPL/LGPL/CC dependency and source audit before shipping.

## darktable integration map
| Starroom panel | Primary upstream reference / source |
| --- | --- |
| Basic / Light | `src/iop/exposure.c` plus relevant tone operators |
| Tone Curve | `src/iop/basecurve.c` and tone-curve spline implementation |
| Color Mixer | `src/iop/colorzones.c` selection/overlap ideas; Starroom output model uses OKLab/OKLCh |
| Color Grading | `src/iop/colorbalancergb.c` |
| Detail | `src/iop/sharpen.c` plus denoise modules |
| Lens Correction | Lensfun plus darktable lens integration patterns |
| Geometry | darktable `ashift` perspective module |
| Effects | `src/iop/vignette.c`, `src/iop/bloom.c` |
| Calibration | `channelmixerrgb` / color calibration module |

## v0.2 architecture
```text
React/TypeScript UI
        |
        v
Tauri IPC / commands
        |
        +-- starroom-core       graph/state primitives
        +-- starroom-color      CPU reference tone + OKLab/OKLCh
        +-- starroom-project    versioned layers/masks/sidecars
        +-- starroom-advisor    deterministic local suggestions
        +-- starroom-portrait   skin/face provider contracts + frequency split reference
        +-- future starroom-gpu / raw / optics / export
```

## P0 — Image quality correctness
- [x] Replace browser shadow/highlight/white/black RGB-to-white interpolation with luminance remapping and RGB scaling.
- [x] Preserve a black anchor when Shadows are raised.
- [x] Add Rust `starroom-color` CPU reference engine.
- [x] Add Rec.2020/D65 luminance basis.
- [x] Add Oklab/OKLCh conversion and hue rotation reference.
- [x] Add eight-band OKLCh Color Mixer core with smooth overlapping hue regions.
- [x] Add monotone cubic Hermite tone-curve reference.
- [x] Add bounded-output OKLCh chroma gamut compression reference.
- [x] Add regression tests preventing the known Shadows white-veil failure.
- [ ] Replace legacy encoded-image Kelvin UI with relative Temperature/Tint controls.
- [ ] Add master/R/G/B curve UI wired to the monotone curve engine.
- [ ] Add ICC/LittleCMS file/display/output boundary.
- [ ] Add Golden Image fixtures for portrait, dark portrait, HDR, neon, ColorChecker, fine texture and high ISO.

## P1 — Core architecture
- [x] Add versioned `AdjustmentLayer` model with opacity, blend mode, order, mask and parameter map.
- [x] Keep old v0.1 projects readable when `layers` is absent.
- [x] Activate `starroom-color`, `starroom-advisor` and `starroom-portrait` as tested workspace crates.
- [ ] Implement Begin/Preview/Commit/Cancel edit transactions for sliders, curves, masks, crop and healing.
- [ ] Remove duplicate frontend history systems.
- [ ] Split oversized `App.tsx` into workspace/library/tools/state/bridge modules.
- [ ] Retire creative math from `src/imagePipeline.ts` after native renderer parity is reached.
- [ ] Add render graph stage dependencies, cache keys and downstream invalidation.
- [ ] Add preview pyramid and tiled full-resolution rendering.

## P2 — Lightroom-style editing modules
- [ ] Light: exposure, highlights, shadows, whites, blacks, contrast with shared tone semantics.
- [ ] Curve: master/R/G/B monotone curves.
- [x] Color Mixer engine core: eight smooth overlapping OKLCh hue bands, each Hue/Chroma/Lightness.
- [ ] Color Mixer UI and per-band visualization.
- [ ] Color Grading: shadows/midtones/highlights/global wheels plus blending/balance.
- [ ] Detail: amount/radius/detail/masking sharpen plus luminance/color NR.
- [ ] Optics: Lensfun-backed distortion, lateral CA and vignetting correction.
- [ ] Geometry: crop, straighten, perspective/keystone and transform.
- [ ] Effects: vignette, grain, bloom/glow.
- [ ] Calibration: camera/input calibration separated from creative grading.

## Semantic Advisor V1
- [x] Native Rust deterministic rule engine skeleton.
- [x] Suggestions contain control, bounded value, confidence and explanation.
- [x] No API/network dependency.
- [x] Add native linear-RGB analysis for shadow/highlight concentration, clipping, median luminance and broad warmth bias.
- [ ] Connect render-analysis samples to the advisor background job.
- [ ] Add UI enable/disable switch and Apply/Ignore actions.
- [ ] Validate numerical helpers against colour-science reference calculations where applicable.

## Layer / Mask Manager V1
- [x] Layer data model supports independent order, enable, opacity, blend mode, mask and adjustments.
- [x] Mask definition supports none/radial/linear/brush/provider forms.
- [x] Add composable Add/Subtract/Intersect mask tree nodes while preserving legacy leaf-mask JSON.
- [ ] Cache runtime raster masks independently from layer adjustments.
- [ ] Add drag reorder and per-layer recomputation/invalidation.

## Portrait / Skin Retouch V1
- [x] Introduce provider-neutral face-landmark interface.
- [x] Add frequency-separation semantics with CPU reference tests.
- [ ] Integrate MediaPipe adapter after dependency/runtime review.
- [ ] Build skin ROI from landmarks and exclude eyes/lips/brows/hair regions.
- [ ] Refine with OKLCh skin likelihood and editable brush mask.
- [ ] GPU separable Gaussian/edge-aware low-frequency stage.
- [ ] Controls: Skin Smooth, Texture Preserve, Tone Evenness, Face Exposure, Skin Hue, Skin Chroma.
- [ ] Healing V1: nearby texture source + low-frequency color/luminance adaptation + feather blend.

## Validation gates
Every completed rendering stage must include:
1. neutral/identity test,
2. finite-value/NaN guard,
3. extreme-control test,
4. CPU reference test,
5. golden-image or perceptual regression when the stage becomes visual,
6. later GPU/CPU parity test before replacing CPU preview.

## v0.2 distribution boundary
- Current target: private/personal use.
- GPL-derived darktable source/ports are permitted within this private-use target and must remain traceable.
- No copied Lightroom implementation, assets, profiles or presets.
- No cloud AI requirement.
- No AI inpainting in V1 healing.
- No claim of physical Kelvin for encoded JPEG/PNG/TIFF editing.
- Before any external distribution, complete a license audit and decide whether to comply with GPL distribution obligations or replace the GPL-derived components.
