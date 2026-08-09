# Codex Start Here

Read all blueprint files before coding. Create the workspace, then implement `TODO.md` milestone-by-milestone.

Recommended layout:
```text
app/                    React + TypeScript
src-tauri/              Tauri command boundary
crates/starroom-core/
crates/starroom-color/
crates/starroom-raw/
crates/starroom-gpu/
crates/starroom-mask/
crates/starroom-project/
crates/starroom-export/
crates/starroom-ai/
native/winml-bridge/
models/manifests/
shaders/
fixtures/
tests/
docs/
schemas/
```

First internal vertical slice: Open JPEG -> read profile -> working RGB -> GPU preview -> Exposure -> Before/After -> color-managed display -> JPEG export -> prove original hash unchanged.

If external APIs have changed, use current official APIs while preserving architecture boundaries and document the change. If a model/dependency license is unclear, do not bundle it.
