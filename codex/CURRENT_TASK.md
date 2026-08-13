# Current Task

## Current Milestone

M16 in progress — local YuNet face detection and BiSeNet ResNet18 semantic portrait masks. M15
remains the only compositing system: M16 contributes compact editable `PortraitSemantic` leaves
and native cached values, then stops. Do not begin M17.

## Goal

Use fixed, local-only YuNet and BiSeNet ResNet18 ONNX models through Rust ONNX Runtime to create
multi-face, source-coordinate, editable soft semantic masks. Preview/Before-After/Export resolve
the same cached source R16Float-compatible mask through the M15 Native shared graph. React only
presents compact face/cache references and interaction state; it never receives parser pixels or
implements landmark, semantic or image math.

## Relevant modules

- `crates/starroom-portrait` fixed-model registry, ONNX sessions, YuNet/BiSeNet adapters
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

Integrate `ort 2.0.0-rc.10` as the single local ONNX Runtime adapter. YuNet is pinned and MIT
approved. BiSeNet ResNet18 is pinned but local-only, non-commercial and must be reviewed before
any public release. Neither weight is committed, packaged or supplied to CI. No cloud, telemetry,
browser canvas or substitute model is permitted.

## Acceptance criteria

- YuNet returns validated multi-face bbox/confidence/five landmarks with per-image stable IDs.
- BiSeNet uses the exact 512 RGB / normalization / CHW input and preserves soft probabilities
  until source-mask refinement; Skin excludes eyes/brows/lips/hair/eyeglass semantics.
- M15 Add/Subtract/Intersect/Invert can operate on a `PortraitSemantic` leaf. Missing native
  cache/model is a typed error, never a transparent mask.
- Detection/parse cache keys record source identity, model hash, face ID and crop transform.
- UI provides detection status, All Faces/individual selection and semantic mask creation only;
  it does not begin M17 retouch tools.

## Targeted tests

- `npm.cmd run test:portrait`
- `npm.cmd run test:masks`
- Native preview/export contract and shared graph Level 2, then Level 3 acceptance

## Stop conditions

Do not begin M17, merge `main`, force-push, or make PR #2 ready. M16 is accepted only after its
dedicated commit and GitHub Windows Level 3 Full Acceptance are green.
