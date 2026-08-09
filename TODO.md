# Complete Internal Build Plan

## Browser vertical slice — completed 2026-08-09
- [x] Direct numeric entry for every slider control
- [x] Kelvin white-balance input (2,000–12,000 K)
- [x] Direct curve editor: add, drag, numeric edit and right-click delete points
- [x] Detail controls use -100…100; sharpen/soften and NR have pixel regression tests
- [x] On-image radial mask placement, move, width/height resize and rotation handles
- [x] Arbitrary -180…180° rotation plus 90° shortcuts and flips
- [x] Wheel zoom (25–600%), drag-to-pan and Fit reset
- [x] Removed duplicate Simple/Pro switch; one complete inspector remains
- [x] Playwright interaction pass and zero browser console errors/warnings

> This is the rendered-file CPU browser slice. It does not complete the production RAW/ICC/wgpu, multi-mask, camera-profile or perspective requirements below.

## M0 Workspace / CI
- [x] Rust workspace + Tauri 2 + React/TS
- [x] crate boundaries, lint/format/test, Windows CI
- [x] fixture manifest and source immutability hash test

## M1 Render Spine
- [ ] JPEG/PNG/TIFF decoder abstraction
- [ ] embedded ICC and fallback rules
- [ ] LittleCMS wrapper + working-space transform
- [ ] wgpu device/surface, Exposure GPU + CPU reference
- [ ] Before/After, display transform, basic export

## M2 Tone / Color
- [ ] WB, Exposure, Highlights, Shadows, Whites, Blacks, Contrast
- [ ] Master/RGB curves, OKLab/OKLCH, HSL mixer, skin protection
- [ ] Vibrance/Saturation, gamut compression, histogram

## M3 RAW
- [ ] LibRaw bridge and RAW metadata model
- [ ] CFA, active area, black/white levels, normalized mosaic
- [ ] bad-pixel stage, Bayer demosaic provider, generic CFA/X-Trans interface
- [ ] camera-profile resolver, RAW WB, DNG matrices
- [ ] NEF/RAF/CR3/ARW/DNG fixtures

## M4 GPU Graph
- [ ] stage graph/cache keys/invalidation
- [ ] preview pyramid, tile renderer, halo-aware filters
- [ ] device-loss recovery, CPU fallback, tiled full-res export

## M5 Masks
- [ ] Brush/Eraser/Linear/Radial/Luminance/Color Range
- [ ] Add/Subtract/Intersect, overlay, mask cache
- [ ] local tone/color, frozen AI-mask format

## M6 Detail/Optics/Geometry
- [ ] Texture/Clarity/Dehaze, classic NR, sharpen
- [ ] lens interface, distortion/CA/vignette
- [ ] crop/rotate/straighten/perspective

## M7 AI Runtime
- [ ] Windows ML/ONNX bridge, model manifest, device enumeration
- [ ] offline CPU fallback, async/cancellable jobs, model cache/fingerprints

## M8 AI Mask
- [ ] MaskProvider, interactive point/box provider
- [ ] semantic provider interface: Subject/Background/Sky/Person
- [ ] Face/Skin/Hair contract where supported
- [ ] editable manual refinement and frozen persistence

## M9 AI Denoise
- [ ] DenoiseProvider, LinearRGB path, tiled overlap/blending
- [ ] strength/detail protection, benchmark suite
- [ ] optional RawMosaic hook, model-version persistence

## M10 Look Engine
- [ ] `.srlook`, LookDescriptor, mood axes, basis mapping
- [ ] parameter-aware Amount and Style Mixer
- [ ] protected categories and regression gallery

## M11 Reference Match
- [ ] analyzer, exposure/WB estimation, quantile tone mapping
- [ ] monotonic curve, hue-band/grading estimates
- [ ] optional semantic matching, bounded refinement, confidence/explain report

## M12 Workflow
- [ ] strip, ratings/flags, copy/paste/sync, batch export
- [ ] history/snapshot/compare/survey/metadata/presets

## M13 Beginner / Pro
- [ ] one shared state and Adjustment Inspector (Simple/Pro split removed after user validation)

## M14 Release Candidate
- [ ] camera/GPU/AI matrices, multi-monitor color tests
- [ ] corrupt input/fuzz, model/dependency license audits
- [ ] 24/45/60/100MP performance and memory suite
- [ ] golden-image regressions, installer/update/uninstall validation

## UI Design System — required before feature-polish
- [ ] Load `design/STARROOM_DESIGN_DNA.json`
- [ ] Implement semantic theme tokens: Dark / Gray / Light
- [ ] Implement brand gradient tokens and functional accent subset
- [ ] Implement app shell with collapsible Library, Filmstrip, hybrid Inspector
- [ ] Implement resizable left/right panels
- [ ] Implement category icon rail + accordion inspector
- [ ] Implement B slider + hover/drag numeric bubble
- [ ] Implement Simple/Pro shared-state toggle
- [ ] Implement proof background switch independent from theme
- [ ] Implement Mask floating toolbar + right mask tree
- [ ] Implement UI quality modes Auto / High / Balanced / Performance
- [ ] Respect reduced motion
- [ ] Add screenshot regression tests for all three themes
- [ ] Validate that UI quality mode cannot change image/export output
