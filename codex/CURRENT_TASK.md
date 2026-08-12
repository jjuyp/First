# Current Task

## Current Milestone

M7 — OKLCh Color Mixer. Continuous batch continues automatically through M11.

## Goal

Ship an eight-band, hue-locked OKLCh mixer in the Native shared Preview/Export graph with typed sampling, persistence and UI transport.

## Relevant modules

- `crates/starroom-color`, `crates/starroom-pipeline`, `crates/starroom-project`
- `src-tauri/src/lib.rs`, `src/nativeRender.ts`, editor state/UI
- render graph and Golden color/portrait/skin/neon/landscape tests

## Required files

- `AGENTS.md`
- This file
- The current milestone's rows in `docs/19_MODULE_DEPENDENCY_MAP.md` and `docs/20_OPEN_SOURCE_IMPLEMENTATION_MAP.md`
- Only crates/UI files listed for the current milestone

## Open-source reference

darktable `colorzones.c` at the pinned release-5.6.0 revision supplies mature zone-selection behavior as a studied reference. Starroom owns the OKLab/OKLCh math and typed adapter; no code is copied in M7.

## Files/modules not related to current task

M2–M6 internals are accepted and may only be touched at the shared settings/stage boundary required to insert M7.

## Acceptance criteria

- Eight bands each expose Hue/Chroma/Lightness with circular smooth overlap.
- Hue lock, achromatic/near-zero stability, sampling and finite/gamut behavior are numerical contracts.
- State serializes and UI undo/redo sends compact settings to identical Native Preview/Export stages.
- M7 targeted, relevant Golden and Level 2 acceptance pass; then update this file to M8 without stopping.

## Targeted tests

- `npm run test:color`
- `npm run test:milestone -- color`

## Golden tags

`color,portrait,skin,neon,landscape`; only active assets execute as photographic Goldens while planned cases remain explicit contracts.

## Required documentation updates

Update milestone acceptance notes and TODO. Provenance changes only if upstream code/data is integrated or directly adapted.

## Stop conditions

Do not stop after M7; continue to M8. Stop only after green M11 batch Full Acceptance. Never merge `main`, make PR #2 ready, force-push or start M12.

## Template for the next milestone

Before new feature work, replace every section above with that milestone's exact scope, modules, upstream pin, acceptance tests, Golden tags and stop conditions.
