# Current Task

## Current Milestone

M15 accepted - Native mask tree and layer compositing. The typed, high-precision Rust evaluator
is connected to M14 layers; do not begin M16 without a new explicit task.

## Goal

Masks are authoritative serialized edit state and render in the Native shared graph. React owns
only drawing/interaction intent; it neither rasterizes nor composites a mask. Preview,
Before/After and Export receive identical tree descriptions and use the same native R16Float
semantic mask stage.

## Relevant modules

- `crates/starroom-project` serializable `MaskTree` grammar
- `crates/starroom-render` R16Float resource contract
- `crates/starroom-pipeline` CPU reference mask evaluator and layer compositing
- `src-tauri/src/lib.rs` native contract validation
- `src/App.tsx` mask/layer interaction state only

## Required files

- `AGENTS.md`
- This file
- `docs/16_OPEN_SOURCE_FOUNDATION.md`
- `docs/19_MODULE_DEPENDENCY_MAP.md`
- `docs/20_OPEN_SOURCE_IMPLEMENTATION_MAP.md`
- `docs/21_DEVELOPMENT_ACCELERATION.md`

## Open-source decision

No dependency is added for M15 expression-tree architecture. Starroom owns composable brush,
linear and radial mask coordinates; it uses the existing R16Float wgpu resource contract and a
native CPU reference for parity. Color/luminance sampling remains Rust-side.

## Acceptance criteria

- Native tree supports None, Brush, Linear, Radial and compositional Add/Subtract/Intersect;
  invalid/provider-only masks are explicit errors, never invisible fallbacks.
- Masks operate in oriented normalized image coordinates, preserve finite `0..1` weights and
  blend into M14 layers identically for Preview/Before-After/Export.
- UI can select and transform radial masks; native requests carry explicit mask intent.
- Regression covers operation algebra, feather/invert, bounds, layer opacity and shared graph
  parity. GPU R16Float allocation remains covered by the existing GPU resource contract.

## Targeted tests

- `npm.cmd run test:masks`
- `cargo test -p starroom-pipeline mask`
- Native preview contract and shared graph Level 2 acceptance

## Stop conditions

M15 passed GitHub Windows Level 3 Full Acceptance run `31713812877`. Stop here: do not begin
M16, merge `main`, force-push or make PR #2 ready.
