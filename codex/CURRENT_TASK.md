# Current Task

## Current Batch

M21 → M22 → M23 — **Implementation candidate; final Level 3 acceptance pending**.

M1-M20 remain accepted. This batch adds fixed local NAFNet denoise, Native perceptual reference
matching and portable `.srlook` workflows through the existing Native shared render graph. Stop
after M23 final acceptance; do not begin M24.

## Goal

Complete M21 local NAFNet-SIDD denoise, M22 perceptual/statistical Reference Match and M23
portable Look Engine without resetting prior native pipelines. All Preview/Before-After/Export
paths keep native cache bindings and one shared graph; React transports only compact intent.

## Relevant modules

- `crates/starroom-ai-denoise` fixed NAFNet registry, domain, tiling and residual adjustment
- `crates/starroom-reference` Native analysis and existing-parameter match recipe
- `crates/starroom-look` `.srlook` schema, semantic blending, grain and vignette
- `crates/starroom-pipeline` shared precreative residual and finishing stages
- `src-tauri/src/lib.rs` local cache and compact native IPC
- `src/nativeRender.ts`, `src/App.tsx` interaction/state presentation only

## Required files

- `AGENTS.md`
- This file
- `docs/16_OPEN_SOURCE_FOUNDATION.md`
- `docs/19_MODULE_DEPENDENCY_MAP.md`
- `docs/20_OPEN_SOURCE_IMPLEMENTATION_MAP.md`
- `docs/21_DEVELOPMENT_ACCELERATION.md`
- `MODEL_PROVENANCE.md`

## Open-source decision

Reuse `ort 2.0.0-rc.10` for the pinned NAFNet-SIDD width-32 ONNX. The checkpoint and export remain
local-only, Git-ignored, unbundled and absent from CI. M22/M23 reuse existing Native adjustment
stages and add no browser color science, cloud, telemetry or substitute model.

## Acceptance criteria

- M21–M23 production implementation, targeted regressions, cross-milestone scenarios and Level 3
  acceptance all pass on the final acceptance commit.
- Preview, Before/After and Export retain one Native shared graph; unavailable providers remain
  explicit typed states rather than transparent/silent fallbacks.

## Targeted tests

- `npm.cmd run test:ai`
- `npm.cmd run test:detail`
- `npm.cmd run test:color`
- Native preview/export contract and shared graph Level 2, then Level 3 acceptance

## Stop conditions

Do not merge `main`, force-push, make PR #2 ready, or begin M24. Only an unrecoverable repository
risk, model-artifact corruption that cannot be reproduced, licensing decision not already covered,
or architecture contradiction may stop this batch.
