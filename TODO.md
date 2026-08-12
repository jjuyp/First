# Complete Internal Build Plan

## Product principle — Open-source Foundation First

Before expanding advanced Starroom-specific features, baseline image quality must stand on mature open-source foundations wherever a proven implementation already exists.

Read `docs/16_OPEN_SOURCE_FOUNDATION.md` before foundation work.

Rules:
- do not replace a mature RAW/tone/color/denoise/optics/perspective implementation with a weaker custom prototype;
- prefer adapters/providers so third-party APIs do not leak into React or unrelated crates;
- CPU reference implementations are validation tools unless they meet the production replacement gate;
- Starroom-native replacements must be equal/better on relevant regression fixtures or be justified by platform, architecture, performance, maintenance, or licensing constraints;
- directly derived darktable code in the current private-use build must record provenance and be marked `GPL-derived / private-use`;
- a trait/struct/UI/placeholder is not a completed production feature.

## F0 Foundation Quality Gate — highest priority

- [x] Document Open-source First strategy and Codex decision rules.
- [x] CI green on current v0.2 branch after all native crates are enabled.
- [x] Create third-party provenance inventory: project, source file, upstream revision, license, integration mode, Starroom files. See `docs/17_THIRD_PARTY_PROVENANCE.md`.
- [x] Establish the executable Golden Image/regression specification and required-case manifest: portrait, dark portrait, white/black clothing, HDR, backlight, night, neon, ColorChecker, fine texture, high ISO and mixed color temperature. See `docs/18_GOLDEN_IMAGE_SPEC.md` and `fixtures/golden/manifest.json`. (Acquiring license-cleared active fixture files remains a later quality-report task.)
- [ ] Define per-stage quality comparison reports against the selected mature foundation.
- [ ] Do not advance a custom foundation replacement unless the replacement gate passes.

## Browser vertical slice — completed 2026-08-09
- [x] Direct numeric entry for every slider control.
- [x] Initial encoded-image white-balance control.
- [x] Direct curve editor: add, drag, numeric edit and right-click delete points.
- [x] Detail controls and browser pixel regression tests.
- [x] On-image radial mask placement, move, width/height resize and rotation handles.
- [x] Arbitrary -180…180° rotation plus 90° shortcuts and flips.
- [x] Wheel zoom (25–600%), drag-to-pan and Fit reset.
- [x] Removed duplicate Simple/Pro switch; one complete inspector remains.
- [x] Playwright interaction pass and zero browser console errors/warnings at the original slice milestone.

> This is a rendered-file CPU browser slice. It is not the production image engine and must not be used as the quality reference for RAW/ICC/tone/detail/optics work.

## M0 Workspace / CI
- [x] Rust workspace + Tauri 2 + React/TS.
- [x] crate boundaries, lint/format/test, Windows CI scaffold.
- [x] fixture manifest and source immutability hash test.
- [x] Keep all newly activated native crates green under fmt/clippy/test.

## M1 Native Render Foundation

### M1A Rendered-file I/O
- [x] Production JPEG/PNG/TIFF decoder connected to native render path.
- [x] Preserve embedded ICC/EXIF metadata through decode boundary.
- [x] Production JPEG/PNG/TIFF export path does not use preview pixels as source.

### M1B Color management — mature foundation required
- [x] Integrate real LittleCMS provider (`lcms2 6.1.1` + statically bundled LittleCMS 2.19).
- [x] Explicit ICC input -> linear Rec.2020 D65 working -> display/output transforms in the native shared graph.
- [x] Validate D50/D65 adaptation and all four ICC rendering intents.
- [x] Missing input profile is explicitly reported as assumed sRGB; invalid embedded/display/output profiles are typed errors and never silently substituted.
- [ ] Multi-monitor/display-profile test plan.

### M1C Native rendered-file pipeline
- [x] Native vertical slice: file-path decode -> input transform -> linear Rec.2020 D65 -> relative WB -> exposure/tone -> curve -> color -> preview/export output in one Rust graph.
- [x] Native preview and export entry points use the same logical processing graph and differ only by requested display/output profile.
- [x] Tauri 2 request/result contract sends source path + edit state as small JSON and returns preview JPEG through versioned binary `ipc::Response`; full-resolution export writes directly to the selected path.
- [x] Real desktop photo preview, Before/After and JPEG export use the Native CPU graph; Browser Canvas remains an explicitly labelled browser/demo fallback and is never selected after a native error.
- [x] Browser reference vs Native CPU regression fixtures cover identity, Exposure, relative WB, Tone and Curve with explicit migration tolerances.
- [ ] Move masks, optics and geometry into the native graph; Native M1C reports these unsupported edits instead of silently ignoring or falling back.
- [ ] Replace the temporary JPEG preview payload with a measured cache/shared-buffer strategy only if profiling proves decode/encode overhead is material; do not introduce shared memory without a cross-platform lifecycle design.
- [ ] Remove temporary Browser Canvas creative math after all remaining rendered-file tools have native coverage and regression acceptance.

## Active sequential foundation milestones (PR #2)

These milestone numbers supersede the older capability-group numbering below for the current acceptance sequence. A later milestone stays blocked until the preceding milestone's local checks and GitHub Actions are green.

### M2 RAW original-file processing

- [x] Vendor and compile real LibRaw 0.22.2 through a narrow Rust provider; select and record the CDDL-1.0 license path.
- [x] Decode NEF, ARW, CR2, CR3, native DNG and RAF from sensor data; embedded JPEG remains Library-thumbnail-only and is never called by Develop.
- [x] Preserve typed metadata for RAW/active dimensions, margins, orientation, black/white levels, Bayer CFA or X-Trans layout, As-Shot multipliers and Camera Neutral.
- [x] Keep the authoritative decode output at 16-bit-to-f32 precision and enter the Native graph as linear Rec.2020/D65 without an 8-bit intermediate.
- [x] Use LibRaw AHD/X-Trans demosaic behind `starroom-raw`; no custom weak demosaic was introduced.
- [x] Route Native Preview, Before/After and Export through the same RAW-aware Rust shared render graph with no browser or embedded-thumbnail fallback.
- [x] Add typed RAW errors, six immutable CC0 camera fixtures, byte/hash/license validation, real decode regressions and separate decode/first-preview/slider-rerender measurements.
- [x] GitHub Actions green for M2 on both push run `31513488590` and Draft PR run `31513493542`; M3 may now begin.

### M3 Camera profiles / RAW color

- [x] Production Camera Profile Resolver with DNG ColorMatrix1/2, ForwardMatrix1/2, CameraCalibration, CalibrationIlluminant and Camera Neutral metadata.
- [x] Explicit camera RGB -> XYZ -> linear Rec.2020/D65 stage with DNG D50 -> working D65 Bradford adaptation in the shared RAW path.
- [x] Extensible Nikon/Canon/Sony/Fujifilm families and explicit Generic Profile state; profile ID/version/SHA-256 is serializable in project state and the active ID is visible in Native UI/export results.
- [x] Add a retained-license colour-science v0.4.7 ColorChecker oracle plus identity/dual-illuminant/finite/round-trip and real RAW profile regressions; the CC0 Apple DNG fixture has no valid embedded profile and is asserted as visibly `Generic`, never silently substituted.
- [x] GitHub Actions and M3 color regressions green: push run `31561022046` and Draft PR run `31561024876` both passed; M4 may now begin when explicitly scheduled.

### M4 Professional Tone / Light

- [x] Directly adapt the stable darktable `sigmoid.c` generalized-loglogistic scene-linear shoulder under GPL-3.0-or-later, with source/revision/NOTICE and external-distribution obligations recorded.
- [x] Native luminance-ratio Tone Engine covers Exposure, Contrast, Highlights, Shadows, Whites and Blacks while preserving a true-black anchor, midtone pivot and unbounded working range.
- [x] Add native numerical portrait-low-key/backlight/HDR/night-style regression vectors: identity, targeted shadow lift, highlight roll-off, narrow white/black zones, hue/chroma ratio preservation and finite extremes.
- [x] GitHub Actions green and M4 acceptance complete: push run `31589000442` and Draft PR run `31589004929` passed; M5 may now begin.

### M5 Professional white balance / calibration

- [x] Separate typed Camera/As-Shot RAW WB from rendered-file Relative Temperature/Tint in the Native shared graph; invalid source/mode combinations are typed errors, never silent fallbacks.
- [x] Add an active deterministic gray-world Auto WB provider plus normalized Native Neutral Picker sampling; both operate before the shared creative graph.
- [x] Persist WB mode, relative Temperature/Tint and optional picker ROI in project state with backward-compatible defaults; UI transport keeps the state small and pixel-free.
- [x] Add neutral-gray, auto/extreme and semantic-error numerical regressions. M5 CI acceptance passed: push run `31591507530` and Draft PR run `31591511674` are green; M6 may now begin.

### M6 Professional tone curves

- [x] Native Master and R/G/B monotone cubic-Hermite curves with endpoint/HDR-safe extrapolation and backward-compatible legacy Master curve migration.
- [x] Channel-aware UI editing (tabs, add/drag/delete, numeric endpoint editing) and undo snapshots now serialize and send the full curve set to Native Preview/Export.
- [x] Add named Identity/S-curve/Black-fade presets and live histogram curve background; preset application is undoable and uses the same Native curve contract.
- [x] M6 acceptance implementation CI passed: push run `31594724778` and Draft PR run `31594728996` are green; acceptance record follows on the same Draft branch.

## M2 Tone / Color Foundation — use mature open-source behavior

### M2A Exposure / Tone
Preferred foundation: darktable exposure/tone implementations and validated scene-referred math.

- [ ] Record selected upstream modules/revisions and integration approach.
- [ ] Exposure ±5 EV.
- [ ] Highlights / Shadows / Whites / Blacks share one coherent tone model.
- [ ] Preserve black anchor and highlight detail where mathematically possible.
- [ ] Contrast uses documented pivot/curve semantics.
- [ ] Compare against mature foundation on Golden Image fixtures.
- [ ] Remove temporary browser tone math after native acceptance.

### M2B White balance / calibration
Preferred foundation: mature darktable color-calibration/channel-mixer concepts plus standard color science.

- [ ] Encoded JPEG/PNG/TIFF uses relative Temperature/Tint semantics, not fake physical Kelvin.
- [ ] RAW uses camera/metadata/profile-aware WB path.
- [ ] Calibration remains separate from creative grading.

### M2C Curves
Preferred foundation: mature darktable curve behavior where useful plus Starroom monotone requirements.

- [ ] Master curve.
- [ ] R/G/B curves.
- [ ] Monotone interpolation / no unintended overshoot.
- [ ] UI curve matches actual render curve.

### M2D Selective color — Starroom differentiator
- [ ] Eight-band OKLCh Color Mixer UI: Red/Orange/Yellow/Green/Aqua/Blue/Purple/Magenta.
- [ ] Hue / Chroma / Lightness per band.
- [ ] Smooth circular overlap.
- [ ] Hue-lock behavior validated with gamut compression.

### M2E Color grading
Preferred foundation: darktable `colorbalancergb` as mature reference/port source.

- [ ] Shadows/Midtones/Highlights/Global grading.
- [ ] Balance/Blending.
- [ ] Record GPL-derived provenance if code is directly adapted.

## M3 RAW Foundation — mature decoder first
Preferred foundation: LibRaw or another proven RAW decoder abstraction.

- [ ] LibRaw bridge and RAW metadata model.
- [ ] CFA, active area, black/white levels, normalized mosaic.
- [ ] Bad-pixel stage.
- [ ] Bayer demosaic provider.
- [ ] Generic CFA/X-Trans interface.
- [ ] Camera-profile resolver, RAW WB, DNG matrices.
- [ ] NEF/RAF/CR3/ARW/DNG fixtures.
- [ ] Nikon/Fujifilm/Sony/Canon regression samples where legally usable.
- [ ] Do not claim RAW support until real files render through the shared graph.

## M4 GPU Graph
- [x] Render graph stage/dependency/cache-key/invalidation foundation exists.
- [x] Halo-aware tile-planning foundation exists.
- [ ] wgpu device/surface and Windows DX12 preferred path.
- [ ] RGBA16Float working preview and R16Float masks.
- [ ] Preview pyramid.
- [ ] Tiled full-resolution rendering/export.
- [ ] Device-loss recovery.
- [ ] CPU fallback.
- [ ] CPU/GPU parity tests for every migrated image stage.

## M5 Layers / Masks — Starroom-owned architecture
- [x] Versioned AdjustmentLayer data model: mask, adjustments, blend mode, enabled, opacity, order.
- [x] Mask tree data model with Add/Subtract/Intersect.
- [ ] Brush/Eraser/Linear/Radial/Luminance/Color Range production masks.
- [ ] Independent raster-mask cache.
- [ ] Layer drag reorder and per-layer invalidation.
- [ ] Local tone/color/detail semantics match global controls.
- [ ] Frozen AI-mask persistence.
- [ ] Full Layer Manager UI.

## M6 Detail Foundation — mature open-source first

### M6A Sharpen
Preferred foundation: darktable `sharpen.c` and other proven sharpening references as appropriate.

- [ ] Record upstream module/revision and integration approach.
- [ ] Amount / Radius / Detail / Masking production model.
- [ ] Avoid halos and color shifts on regression fixtures.

### M6B Denoise
Preferred foundation: mature darktable classic/profiled denoise implementations where suitable.

- [ ] Luminance/color noise controls.
- [ ] Preserve edges/fine texture.
- [ ] High ISO fixture suite.
- [ ] Do not use a generic blur as production denoise.
- [ ] Keep AI denoise as a later local provider, not a substitute for a good classic baseline.

### M6C Texture / Clarity / Dehaze
- [ ] Multi-scale local-contrast foundation.
- [ ] Verify no severe halos at edges/high-contrast boundaries.

## M7 Optics / Geometry Foundation

### M7A Lens correction
Preferred foundation: Lensfun.

- [ ] Integrate real Lensfun library/database provider.
- [ ] Lens identification.
- [ ] Distortion.
- [ ] Lateral chromatic aberration.
- [ ] Vignetting profile correction.
- [ ] Lensfun result feeds Starroom CPU/GPU renderer through adapter boundary.

### M7B Geometry / Perspective
Preferred foundation: darktable `ashift` and proven projective math.

- [ ] Crop.
- [ ] Rotate / straighten.
- [ ] Perspective / keystone.
- [ ] Architecture fixture regression set.
- [ ] Record GPL-derived provenance if directly adapted.

## M8 Effects / Calibration
- [ ] Vignette using mature reference where appropriate.
- [ ] Grain.
- [ ] Bloom/glow using mature reference where appropriate.
- [ ] Color calibration implementation separated from creative color grading.

## M9 Semantic Advisor — Starroom differentiator
- [x] Deterministic local rule-engine foundation.
- [x] Basic image statistics foundation.
- [ ] Run analysis as cancellable/background native job.
- [ ] UI switch.
- [ ] Explainable suggestions with Apply/Ignore.
- [ ] Validate numerical helpers against colour-science where useful.
- [ ] Advisor never silently mutates edits.

## M10 Portrait / Skin / Healing — Starroom workflow
- [x] Provider-neutral FaceLandmarkProvider contract.
- [x] Frequency-separation CPU reference foundation.
- [x] Healing CPU reference foundation.
- [ ] Integrate MediaPipe adapter after runtime/license/privacy review.
- [ ] Face ROI and facial-feature exclusions.
- [ ] Continuous skin likelihood/refinement without a single hard skin-color rule.
- [ ] Editable skin mask.
- [ ] Skin Smooth / Texture Preserve / Tone Evenness / Face Exposure / Skin Hue / Skin Chroma.
- [ ] Production healing brush UI and history transactions.
- [ ] AI inpainting remains post-V1.

## M11 AI Runtime
- [ ] Windows ML/ONNX bridge, model manifest, device enumeration.
- [ ] Offline CPU fallback, async/cancellable jobs, model cache/fingerprints.
- [ ] Local-first only for core AI.

## M12 AI Mask
- [ ] MaskProvider, interactive point/box provider.
- [ ] Subject / Background / Sky / Person.
- [ ] Face / Skin / Hair where supported.
- [ ] Editable manual refinement and frozen persistence.

## M13 AI Denoise
- [ ] DenoiseProvider, LinearRGB path, tiled overlap/blending.
- [ ] Strength/detail protection, benchmark suite.
- [ ] Optional RawMosaic hook, model-version persistence.

## M14 Look Engine
- [ ] `.srlook`, LookDescriptor, mood axes, basis mapping.
- [ ] Parameter-aware Amount and Style Mixer.
- [ ] Protected categories and regression gallery.

## M15 Reference Match
- [ ] Analyzer, exposure/WB estimation, quantile tone mapping.
- [ ] Monotonic curve, hue-band/grading estimates.
- [ ] Optional semantic matching, bounded refinement, confidence/explain report.

## M16 Workflow
- [ ] Strip, ratings/flags, copy/paste/sync, batch export.
- [ ] Unified Begin/Preview/Commit/Cancel transaction system.
- [ ] Remove duplicate frontend history logic.
- [ ] History/snapshot/compare/survey/metadata/presets.

## M17 UI Refactor / Design System
- [ ] Split oversized `App.tsx` into workspace/library/tools/state/bridge modules.
- [ ] Load `design/STARROOM_DESIGN_DNA.json`.
- [ ] Semantic theme tokens: Dark / Gray / Light.
- [ ] Brand gradient tokens and functional accent subset.
- [ ] Collapsible Library, Filmstrip, hybrid Inspector.
- [ ] Resizable left/right panels.
- [ ] Category icon rail + accordion inspector.
- [ ] Slider numeric bubble and direct numeric entry.
- [ ] Proof background switch independent from theme.
- [ ] Mask floating toolbar + right mask tree.
- [ ] UI quality modes Auto / High / Balanced / Performance.
- [ ] Reduced-motion support.
- [ ] Screenshot regression tests for all three themes.
- [ ] UI quality mode cannot change image/export output.

## M18 Release Candidate
- [ ] Camera/GPU/AI matrices.
- [ ] Multi-monitor color tests.
- [ ] Corrupt input/fuzz.
- [ ] Complete dependency/model/GPL/LGPL/CC provenance audit.
- [ ] 24/45/60/100MP performance and memory suite.
- [ ] Golden-image regressions.
- [ ] Installer/update/uninstall validation.

## Product acceptance rule
The foundation wins before novelty.

Do not call Starroom production-ready if its RAW, tone, color, denoise, sharpening, optics, or perspective baseline is materially worse than the mature open-source foundation selected for that stage, even if advanced Starroom features are already impressive.
