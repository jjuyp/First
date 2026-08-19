# Current Task

## Current Batch

M17 → M18 → M19 → M20 — **Complete / Accepted**.

M16 is complete and accepted. This batch completes professional skin retouch, healing, the local
semantic advisor and local AI masks through the existing Native shared render graph. Stop after
M20 final acceptance; do not begin M21.

## Goal

M17–M20 are complete without resetting the valid implementation drafts.
M20 extends M15 with reusable local AI masks: M16 portrait semantics, BiRefNet Subject/Background,
and SegFormer-B0 ADE20K Sky. All preview/export paths use native cache bindings; React transports
only compact metadata and interaction state.

## Relevant modules

- `crates/starroom-portrait` fixed-model registry, ONNX sessions, YuNet/BiSeNet and M20 adapters
- `crates/starroom-project` serializable `PortraitSemantic` MaskTree grammar
- `crates/starroom-pipeline` native semantic-raster resolution and layer compositing
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

Reuse `ort 2.0.0-rc.10` as the single local ONNX Runtime adapter. Model decisions and pins are
already recorded in `MODEL_PROVENANCE.md`; weights are local-only, Git-ignored, never packaged or
supplied to CI. No cloud, telemetry, browser canvas math, substitute model or M21 work is allowed.

## Acceptance criteria

- M17–M20 production implementation, targeted regressions, cross-milestone scenarios and Level 3
  acceptance all pass on the final acceptance commit.
- Preview, Before/After and Export retain one Native shared graph; unavailable providers remain
  explicit typed states rather than transparent/silent fallbacks.

## Targeted tests

- `npm.cmd run test:portrait`
- `npm.cmd run test:masks`
- Native preview/export contract and shared graph Level 2, then Level 3 acceptance

## Stop conditions

Do not merge `main`, force-push, make PR #2 ready, or begin M21. Only an unrecoverable repository
risk, model-artifact corruption that cannot be reproduced, licensing decision not already covered,
or architecture contradiction may stop this batch.
