# Current Task

## Current Milestone

M11 - Professional Geometry acceptance. Stop after green M7-M11 Level 3 acceptance; do not start M12.

## Goal

Ship crop, transform, perspective, image-derived Upright and explicit coordinate spaces in the Native shared Preview/Export graph.

## Relevant modules

- `crates/starroom-geometry`, `crates/starroom-pipeline`, `crates/starroom-project`
- `src-tauri/src/lib.rs`, `src/nativeRender.ts`, editor state/UI
- render graph and Golden architecture/geometry tests

## Required files

- `AGENTS.md`
- This file
- M11 rows in `docs/19_MODULE_DEPENDENCY_MAP.md` and `docs/20_OPEN_SOURCE_IMPLEMENTATION_MAP.md`
- Only crates/UI files listed for this milestone

## Open-source reference

darktable `ashift.c` at the pinned release-5.6.0 revision is the architecture/behavior reference. Starroom owns the typed projective transform, inverse resampler and coordinate mapper; no code is copied in M11.

## Acceptance criteria

- Free/original/common/custom crop, transform, keystone, four-point and Upright are production Native stages.
- Coordinate spaces are explicit and finite/boundary behavior is tested.
- State serializes and UI undo/redo sends compact settings to identical Native Preview/Export stages.
- M11 targeted, relevant Golden and Level 3 batch acceptance pass, then stop before M12.

## Targeted tests

- `npm run test:geometry`
- `npm run test:milestone -- geometry`
- `npm run test:full`

## Golden tags

`architecture,geometry`; only active assets execute while planned cases remain explicit contracts.

## Required documentation updates

Update milestone acceptance notes and TODO. Provenance changes only if upstream code/data is integrated or directly adapted.

## Stop conditions

Stop only after green M11 batch Full Acceptance. Never merge `main`, make PR #2 ready, force-push or start M12.
