# 03 — Local AI Mask + AI Denoise

## Principles
Core AI is local, editable, runtime-agnostic and model-versioned. CPU fallback always exists. Model license/weights license are mandatory release gates.

## Runtime
```text
Rust -> AiRuntime trait -> native Windows ML bridge -> ONNX Runtime -> CPU/GPU/NPU
```
Use a narrow C ABI to isolate Windows/vendor API churn. Guarantee offline operation with locally installed models and CPU path; hardware EPs are optional capability enhancements.

## Model manifest
Requires modelId, version, SHA-256, ONNX format, task, license ID, input/output contract, precision modes, runtime requirements and tile/halo rules.

## AI Mask roles
Do not use one model for everything.
1. Interactive segmentation provider: positive/negative points and optional box -> probability mask. A SAM2-class model is a valid benchmark candidate, but production needs ONNX/runtime/latency validation.
2. Semantic provider: Subject/Background/Sky/Person probability maps.
3. Optional person-detail provider: Face/Skin/Hair/Clothes/Body only when supported by the chosen model.

Postprocess: source-coordinate resize -> optional edge refinement -> island cleanup -> mask raster. AI mask becomes a normal mask node and supports invert/add/subtract/intersect/brush refinement.

## AI mask persistence
Save model ID/version/hash, semantic/prompt request, source hash, frozen lossless mask-cache reference and manual refinement. Frozen cache preserves project appearance if models later change. Suggested mask tiles: 16-bit alpha + lossless zstd + versioned header.

## AI Denoise domains
- RawMosaicDenoiser: after RAW normalization, before demosaic. Long-term sensor-aware quality path.
- LinearRgbDenoiser: after demosaic/camera transform, before creative tone/color. Preferred generic RGB model integration.
- DisplayRgbDenoiser: compatibility experiments only, not default.

`DenoiseProvider` declares domain, halo, model fingerprint and tile inference. Large images use overlap and smooth blending; overlap is model-manifest specific. Strength 0 must be exact input; baseline blend is `lerp(input,denoised,strength)`, with future detail-aware blending allowed if tested.

## Candidate policy
SwinIR is a useful Apache-2.0 RGB denoise reference. NAFNet-like architectures are useful benchmarks. No academic checkpoint is bundled without explicit weight/license validation.

## Queue/privacy
Priority: explicit mask -> visible denoise preview -> export denoise -> background suggestions. Jobs cancellable/progress-aware. No image/prompt upload. Optional telemetry may contain timing/model/device only, never pixels.
