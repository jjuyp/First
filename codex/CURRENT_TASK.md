# Current Task

## Current Milestone

M14 - Non-destructive adjustment layers. Build typed, ordered native layer evaluation and the
layer-stack interaction surface before advancing to M15.

## Goal

Layers are product architecture, so Starroom owns their ordering, opacity, persistence and undo
semantics. The native shared graph evaluates enabled Normal layers in order; React only edits
serializable layer intent and never calculates a pixel.

## Relevant modules

- `crates/starroom-project` persisted layer model and project compatibility
- `crates/starroom-pipeline` native layer evaluator and shared preview/export graph
- `src-tauri/src/lib.rs` compact native layer request contract
- `src/App.tsx` layer interaction, project state and reversible history
- `scripts/test-target-config.mjs` layer acceptance routing

## Required files

- `AGENTS.md`
- This file
- `docs/16_OPEN_SOURCE_FOUNDATION.md`
- `docs/19_MODULE_DEPENDENCY_MAP.md`
- `docs/20_OPEN_SOURCE_IMPLEMENTATION_MAP.md`
- `docs/21_DEVELOPMENT_ACCELERATION.md`

## Open-source decision

No mature external library owns Starroom's non-destructive layer document model. Use the existing
serde-backed `starroom-project` model and native linear-light blend semantics. Do not introduce
image math in TypeScript or substitute Browser Canvas for the shared graph.

## Acceptance criteria

- Ordered add/delete/rename/duplicate/enable/reorder/opacity operations persist and undo/redo.
- Native Preview and Export receive the same compact layer-stack settings and evaluate active
  Normal layers in order. Source pixels remain immutable.
- Layer adjustment values are finite and typed; unsupported blend modes are explicit errors,
  never silent substitutions.
- Layer-state changes are reflected in render/cache identity and regressions cover order, opacity,
  persistence and native preview/export parity.

## Targeted tests

- `npm.cmd run test:layers`
- `cargo test -p starroom-pipeline layers`
- Native request-contract Vitest and shared graph Level 2 acceptance

## Stop conditions

After M14 local and GitHub acceptance, continue directly to M15. Never merge `main`, force-push
or make PR #2 ready.
