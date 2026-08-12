# Current Task

## Current Milestone

M12 - GPU Render Engine. Complete the wgpu/DX12 acceleration backend and its
CPU-reference parity acceptance before advancing to M13.

## Goal

Keep the existing Rust Native Render Graph as the single semantic contract for
Preview, Before/After and Export. The CPU renderer remains the reference
oracle; the GPU is an explicit acceleration backend with typed status and CPU
fallback, never a second color-science implementation.

## Relevant modules

- `crates/starroom-render` GPU lifecycle, resource/cache boundaries and parity
- `crates/starroom-pipeline` shared graph integration
- `src-tauri/src/lib.rs`, `src/nativeRender.ts` native preview backend status
- GPU parity and Native Preview/Export tests

## Required files

- `AGENTS.md`
- This file
- `docs/16_OPEN_SOURCE_FOUNDATION.md`
- M12 rows in `docs/19_MODULE_DEPENDENCY_MAP.md` and
  `docs/20_OPEN_SOURCE_IMPLEMENTATION_MAP.md`
- `docs/21_DEVELOPMENT_ACCELERATION.md`

## Open-source decision

Integrate wgpu `v30.0.0` at the pinned M12 revision as a Rust-only backend.
The official wgpu API/WGSL validation is used directly; Starroom retains its
existing Native CPU processing as the parity oracle and owns the adapter,
scheduling and UI status contract.

## Acceptance criteria

- Windows DX12 is requested first, other backend selection is explicit, and
  typed CPU fallback covers missing adapters, device creation, loss, OOM,
  shader validation and unsupported capability.
- Linear Rec.2020 D65 RGBA16Float and R16Float mask resource contracts are
  finite-safe.
- The selected GPU nodes are exercised by Native preview / Before-After while
  export retains identical CPU-reference semantics when deterministic GPU
  export is not selected.
- Strict CPU/GPU parity covers neutral, portrait, skin, landscape, neon, HDR,
  shadows, highlights, saturation extremes and RAW/encoded fixtures.
- Existing CPU graph tests remain green.

## Targeted tests

- `npm run test:milestone -- gpu` (added with M12)
- `cargo test -p starroom-render gpu`
- GPU parity fixture selection and Native Preview contract tests

## Stop conditions

After M12 acceptance, continue directly to M13 under the continuous-execution
request. Never merge `main`, force-push or make PR #2 ready.
