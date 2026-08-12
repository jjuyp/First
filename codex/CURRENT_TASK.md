# Current Task

## Current Milestone

Development Acceleration Pass — final acceptance gate before M7. **M7 is not started.**

## Goal

Maintain fast, dependency-aware development loops without weakening Starroom's Native shared-graph, RAW, color, Golden or release gates.

## Relevant modules

- `scripts/test-target*.mjs`, `scripts/select-golden-fixtures.mjs`
- `scripts/ci-changed-paths.mjs`, `.github/workflows/blueprint-check.yml`
- `fixtures/golden/manifest.json`
- `docs/19_MODULE_DEPENDENCY_MAP.md`, `docs/20_OPEN_SOURCE_IMPLEMENTATION_MAP.md`

## Required files

- `AGENTS.md`
- This file
- The current milestone's rows in `docs/19_MODULE_DEPENDENCY_MAP.md` and `docs/20_OPEN_SOURCE_IMPLEMENTATION_MAP.md`
- Only crates/UI files listed for the current milestone

## Open-source reference

No new image algorithm is introduced by this pass. Future pinned references are recorded in `docs/20_OPEN_SOURCE_IMPLEMENTATION_MAP.md`; provenance remains authoritative for any integration or derivation.

## Files/modules not related to current task

M2 RAW decode and M3 camera-profile internals, M4 tone math, M5 WB semantics and M6 curve math are accepted foundations and must not be rewritten during workflow optimization.

## Acceptance criteria

- Canonical Level 1/2/3 commands execute rather than merely document a plan.
- Golden manifest has validated canonical tags and deterministic subset selection.
- CI classifies paths, broadens shared changes, caches safely, reports timing and retains an explicit Full Check.
- Full Acceptance passes; PR #2 stays Draft; `main` remains unmerged; M7 remains unstarted.

## Targeted tests

- `npm run test:infra`
- `npm run test:web`
- Representative Rust target: `npm run test:curve -- --rust-only`
- Final gate: `npm run test:full`

## Golden tags

Infrastructure validation covers the complete tag registry and verifies the future M7 selection `color,portrait,skin,neon,landscape`. No M7 image processing is executed.

## Required documentation updates

Update `TODO.md`, `docs/IMPLEMENTATION_NOTES.md`, Golden specification, acceleration guide, dependency map and implementation map in the acceptance commit. Provenance changes only when a dependency/source/derivation changes.

## Stop conditions

Stop after the acceleration acceptance commit and green Full CI. Do not start M7, merge `main`, make PR #2 ready, force-push, or weaken Full Acceptance.

## Template for the next milestone

Before new feature work, replace every section above with that milestone's exact scope, modules, upstream pin, acceptance tests, Golden tags and stop conditions.
