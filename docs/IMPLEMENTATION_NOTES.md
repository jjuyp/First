# Implementation Notes
Record deviations, dependency-version changes, GPU/backend issues, camera exceptions, model substitutions, benchmarks and unresolved quality tradeoffs. Do not rewrite specification history to hide compromises.

## 2026-08-11 — v0.2 native workspace quality gate

- Repaired the `starroom-heal` test indexing rejected by Rust 1.97 Clippy and retained explicit `(width, x, y)` coordinate semantics through a shared test helper.
- Corrected the OKLab inverse XYZ matrix sign for the S contribution to X. The previous negative sign caused measurable lightness and chroma drift after hue rotation; a Rec.2020 RGB -> OKLab -> Rec.2020 RGB round-trip regression now protects the conversion chain.
- Truncated two color-matrix literals only beyond meaningful `f32` precision, as required by the current Clippy `excessive_precision` lint.
- Regenerated `Cargo.lock` after activating rendered-image I/O dependencies. Rust CI now uses `--locked` for Clippy and tests, and checks formatting without mutating the checkout.
- Local native compilation remains unavailable because this workstation has no Visual C++ linker. Two independent Windows GitHub Actions runs passed workspace format, Clippy and tests before the stricter reproducibility gate was enabled; the updated gate remains the authoritative Windows validation.

## 2026-08-09 — M0 workspace and interactive shell

- Added the React/TypeScript/Vite application shell, theme tokens, persisted Dark/Gray/Light theme, persisted Simple/Pro mode, collapsible Library/Filmstrip, tool rail, inspector controls, local image import, Before state, and reversible UI adjustment history.
- Added a Rust workspace with `starroom-core`, `starroom-project`, and a narrow Tauri 2 command boundary.
- Added source-identity SHA-256 verification and a test proving identity reads do not mutate source bytes.
- Added frontend and Windows Rust CI gates.
- The current canvas uses a non-color-managed browser preview. Adjustment controls intentionally do not use CSS image filters; decoded-image color management and shared CPU/GPU render output remain M1 work.
- Pinned TypeScript to the current 6.0 series because `typescript-eslint` 8.66 does not yet accept TypeScript 7. Frontend validation used React 19.2.8, Vite 8.2.1 and Vitest 4.1.10.
- Vite/Rollup exits after transform when the repository is addressed through its Unicode Windows path. `scripts/windows-safe-build.mjs` creates a temporary ASCII `subst` alias for production builds and removes it afterward; Linux/ASCII paths run directly.
- Installed Rust 1.97.1 through official rustup and verified `cargo fmt --all --check`. Local Rust compilation is blocked by the machine's missing Visual C++ linker (`link.exe`). The GNU fallback also lacks a working external MinGW `dlltool` chain. Windows CI is the independent compile/test path; installing the multi-gigabyte Visual Studio C++ Build Tools was intentionally not performed implicitly.
- Added `開啟 Starroom.cmd` and `scripts/start-starroom.ps1` as the supported double-click Windows entry point. Direct `file://` access to the Vite source `index.html` now shows a launch explanation instead of a blank page; the launcher verifies/builds `dist`, starts the local preview service, waits for readiness and opens `http://127.0.0.1:4173`.

## 2026-08-09 functional browser editing slice

- Replaced the decorative library, filmstrip, histogram and adjustment controls with a working browser implementation. Imported JPEG, PNG, WebP and SVG files now become independent photo items with selection, rating, edit counts, history and library filters.
- Added a deterministic CPU canvas pipeline for exposure, contrast, highlights, shadows, whites, blacks, temperature, tint, vibrance and saturation. The histogram is calculated from the rendered preview rather than placeholder data.
- Added per-photo undo/redo, Before/original preview, reset, Fit/100% display and full-resolution JPEG export. Export creates a new download and never overwrites the source file.
- Browser previews are limited to a 1,800-pixel longest edge to keep interaction responsive; export re-renders from the decoded source dimensions.
- This slice is rendered-file editing only. RAW decoding, ICC-aware color management, GPU/wgpu rendering, crop/geometry, curves, masks, optics and detail tools remain future milestones. Unimplemented tool buttons are visibly disabled instead of simulating edits.

## 2026-08-09 complete interactive shell pass

- Made the Library, Edit and Compare header workspaces functional. Library presents a photo grid, Edit presents the single-photo editor, and Compare renders original and edited output side by side.
- Added safe removal from the Starroom workspace through the editor toolbar, filmstrip thumbnails, library cards and the Delete key. Removal revokes browser object URLs but never deletes the source file from disk; the last remaining photo is protected.
- Enabled every inspector tool with reversible CPU-rendered controls: three-region tone curve, sharpness/clarity/noise reduction, a feathered radial center mask, vignette/edge brightness, 90-degree rotation and horizontal/vertical flips.
- These are intentionally bounded first implementations, not claims of full production equivalents. Curve control is three-region rather than a freeform spline; Masks currently supplies one centered radial mask; Optics does not yet use camera/lens profiles; Geometry does not yet crop or correct perspective.
- Replaced the initial decorative curve graphic with a parameter-driven SVG. Its smooth line and Shadow/Midtone/Highlight control points now update from the same authoritative values used by the pixel pipeline and stay clipped within the graph viewport.

## 2026-08-09 direct-manipulation editing pass

- All range controls now pair with bounded numeric inputs. White balance uses a 2,000–12,000 Kelvin value with 6,500 K as neutral. The redundant Simple/Pro switch was removed so there is one complete inspector.
- Replaced the fixed three-region curve UI with a serializable point list used by both preview and export. Left-click adds points, dragging changes input/output, right-click removes non-endpoints, and the selected point also exposes numeric input/output fields.
- Detail uses a signed -100…100 contract for Sharpness, Clarity and Noise Reduction. Positive sharpness now applies a stronger unsharp term; negative sharpness blends toward the local blur. Pixel tests cover visible sharpening and Kelvin channel response.
- Radial mask geometry is normalized and serializable. The photo overlay supports click-to-place, interior drag, independent width/height handles and a rotation handle; the inspector exposes numeric center, size and angle values. The same rotated ellipse drives CPU preview and export.
- Geometry now accepts arbitrary -180…180 degree values, calculates a non-clipping output canvas, and retains 90-degree shortcut and flip controls.
- The editor viewport supports 25–600% wheel zoom, Fit reset and left-button pan while zoomed. Zoom/pan remain view state and do not alter exported pixels.
- Edit history snapshots now include adjustments, freeform curve points and mask geometry. Source files remain untouched.
- Playwright validated numeric input, 9,000 K, curve add/drag/right-click-delete, on-photo mask placement/resize/rotation, 33.5-degree rotation, 47%/212% wheel zoom and drag pan. The final browser console check reported zero errors and zero warnings.
- Remaining production gaps are unchanged: no RAW decoding, ICC display/output management, wgpu path, multi-mask stack/brush masks, lens-profile correction, crop or perspective correction.
