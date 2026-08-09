# AGENTS.md — Starroom Engineering Contract

## Mission
Build a professional-quality, non-destructive desktop photo editor. Optimize for correctness, image quality, responsiveness, saved-edit portability, privacy, and understandable UX.

## Invariants
1. Never overwrite source images.
2. Every adjustment is serializable and reversible.
3. Preview and export share one logical graph.
4. Full-resolution export never uses preview-resolution pixels as source.
5. Global and local edits share adjustment semantics.
6. UI code never implements color-science or image math.
7. AI outputs editable artifacts, never hidden destructive mutations.
8. Projects record schema, camera-profile and AI-model versions/hashes.
9. Avoid hard RGB clipping until explicitly bounded final output.
10. No NaN/Inf crosses a stage boundary.
11. Never silently substitute an incorrect camera/display profile or AI model.
12. Processing-order changes require regression tests.
13. Every dependency/model needs a recorded license review.
14. Do not copy proprietary Lightroom algorithms, assets, profiles, presets, or private behavior.

## Boundaries
React/TypeScript owns UI and interaction. Rust owns project state, image graph, mask graph, color, RAW, GPU scheduling, caching and export. A narrow native Windows AI bridge may wrap Windows ML/ONNX APIs behind a stable C ABI.

## Precision
- RAW and CPU reference: f32
- GPU RGB preview: RGBA16Float by default
- GPU mask: R16Float by default
- Perceptual color: OKLab/OKLCH
- authoritative edit state is never 8-bit

## Color
Internal baseline is unbounded linear wide-gamut RGB using Rec.2020 primaries/D65. ICC/LittleCMS is used at file/display/output boundaries. D50/D65 adaptation must be validated.

## RAW
LibRaw is a decoder and metadata source, not Starroom's final rendering engine. Starroom owns normalization, demosaic-provider abstraction, camera profile, working-space conversion and creative processing.

## GPU
Photo rendering uses wgpu. Windows prefers DX12. Critical stages have CPU reference/fallback paths.

## AI
Core AI is local. Runtime priority is hardware-optimized Windows ML EP when validated, then compatible GPU path, then CPU fallback. Model ID/version/SHA/license/input/output/precision/benchmark are mandatory metadata.

## Render invalidation
Each stage declares dependencies, parameter hash, halo, cache key, CPU/GPU availability. Invalidate only changed and downstream stages.

## Codex
Implement one milestone at a time. Run tests/lints after each milestone and record deviations in `docs/IMPLEMENTATION_NOTES.md`.
