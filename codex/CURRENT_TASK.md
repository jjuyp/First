# Current Task

## Current Milestone

M13 - Tile / Preview Pyramid / Render Scheduler. Build the bounded, cancellable native preview
planner and integrate it with the existing CPU-reference / wgpu acceleration paths before
advancing to M14.

## Goal

Keep one logical Native Render Graph. M13 owns only resolution selection, tile identity,
viewport priority, cancellation, cache and resource-budget policy. It must not introduce image
or color math in React/TypeScript or replace CPU/GPU stage semantics.

## Relevant modules

- `crates/starroom-render::scheduler` preview levels, tile regions/halos, LRU and generations
- `crates/starroom-render` graph halo/dependency declarations
- `src-tauri/src/lib.rs` request scheduling, explicit stale-output rejection and derived-frame cache
- `scripts/test-target-config.mjs` `tiles` targeted acceptance routing

## Required files

- `AGENTS.md`
- This file
- `docs/16_OPEN_SOURCE_FOUNDATION.md`
- M13 rows in `docs/19_MODULE_DEPENDENCY_MAP.md` and
  `docs/20_OPEN_SOURCE_IMPLEMENTATION_MAP.md`
- `docs/21_DEVELOPMENT_ACCELERATION.md`

## Open-source decision

Integrate no image-processing dependency for M13. `wgpu v30.0.0` remains the already-pinned
resource backend. Starroom owns scheduler/cache behavior because it is product architecture,
not a replacement for established color, RAW, tone, detail or geometry algorithms.

## Acceptance criteria

- 512/1024/2048/4096 preview levels are selected deterministically and source images are never
  downsampled for export.
- Every tile contains source/version, graph identity, level, output region and request generation;
  halo is supplied from the existing graph contract.
- Visible viewport tiles precede nearby and remaining tiles. A newer request supersedes old work
  and stale output is explicitly discarded.
- RAM/VRAM LRU limits are bounded, cache keys change for graph/source changes, and no cache value
  can change rendering semantics.
- Native preview actively schedules pyramid work and can reuse only a matching derived frame;
  GPU absence remains the explicit M12 CPU-reference fallback.

## Targeted tests

- `npm.cmd run test:tiles`
- `cargo test -p starroom-render scheduler` (Windows CI is authoritative on this workstation)
- Native preview contract Vitest and shared graph Level 2 acceptance

## Stop conditions

After M13 local and GitHub acceptance, continue directly to M14 under the continuous-execution
request. Never merge `main`, force-push or make PR #2 ready.
