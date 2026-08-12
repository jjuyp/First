# Implementation Notes
Record deviations, dependency-version changes, GPU/backend issues, camera exceptions, model substitutions, benchmarks and unresolved quality tradeoffs. Do not rewrite specification history to hide compromises.

## 2026-08-12 — M3 camera profile / RAW color candidate

- The LibRaw bridge still owns sensor parsing, black/white normalization, As-Shot WB and mature demosaic, but no longer performs the final camera-to-working transform. It emits 16-bit linear, white-balanced camera RGB with an identity `rgb_cam`; Rust converts to authoritative `f32`, resolves a typed camera profile, executes camera RGB -> XYZ D65 -> linear Rec.2020 D65, and only then enters the shared creative graph. Preview, Before/After and Export therefore use the same explicit profile stage.
- `CameraProfileResolver` consumes LibRaw's public `cam_xyz` plus both public `dng_color` records. Embedded DNG ForwardMatrix is preferred; ColorMatrix is combined with CameraCalibration and inverted when ForwardMatrix is absent. Two calibration sets are selected/interpolated in reciprocal-temperature (mired) space using the As-Shot neutral estimate, and DNG's D50 PCS result is adapted to D65 with the tested Bradford stage.
- Native Nikon, Canon, Sony and Fujifilm inputs use LibRaw's identified camera matrix through an extensible family resolver. A DNG with valid embedded matrices is resolved independently of make. An unknown camera or invalid/missing matrix enters a visibly reported `Generic RAW Profile` using the documented linear-sRGB generic basis; it is never labeled as a known profile or silently substituted.
- Every descriptor carries stable ID, resolver version and SHA-256 over the exact matrix/policy fields. `Project.cameraProfile` persists ID/version/hash/status with backward-compatible optional deserialization. Native SRP2 preview transport appends only the small UTF-8 profile ID before the JPEG payload; pixels remain binary, and export reports the profile ID/hash.
- Added the `colour-science/colour` v0.4.7 `DATA_BABELCOLOR_AVERAGE` 24-patch xyY reference under its retained BSD-3-Clause license. Its source URL, tag object, source blob SHA-1, illuminant, observer and published precision are recorded. The Golden image scene remains `planned` because no chart photograph/baseline has been accepted; only the numerical oracle is active.
- M3 limitations: the current production bridge supports three-channel camera RGB, which covers the active NEF/ARW/CR2/CR3/DNG/RAF fixtures. Unusual four-channel sensors must receive an explicit matrix/demosaic contract before being claimed. No proprietary DCP profiles or silent make/model substitutions are bundled. GPU parity remains a later milestone.

## 2026-08-12 — M2 LibRaw sensor pipeline candidate

- Integrated the real LibRaw 0.22.2 source at peeled commit `b93f6e45c194f5df9b02a43b1af9a54b4f41f33f` (annotated tag object `24fa7e5463cbf8b8615dbd2b16c933a294d52400`) under the selected CDDL-1.0 path. Upstream source and notices are vendored unchanged; `starroom-raw` owns an original narrow C ABI bridge instead of leaking LibRaw structs through the Rust workspace.
- Develop uses `open_buffer` -> `unpack` -> sensor-buffer validation -> `dcraw_process` -> 16-bit memory image. The bridge never calls `unpack_thumb` or any embedded-JPEG API. Library thumbnails remain a separate future optimization.
- LibRaw owns format parsing, black subtraction/scaling, camera As-Shot WB and mature AHD/X-Trans demosaic. It emits linear Rec.2020/D65 (`output_color = 8`, unity gamma, no auto-brightening) at 16-bit precision; Rust converts those samples to authoritative `f32` without an 8-bit intermediate. Preview may request LibRaw half-size sensor processing and then a Lanczos3 bound; export reopens and fully develops the immutable source.
- `RawMetadata` records format, make/model, decoder, RAW/active sizes and margins, orientation, Bayer filter or 6x6 X-Trans layout, black/channel-black/white levels, As-Shot multipliers, derived green-normalized Camera Neutral, pre-multipliers, demosaic provider and LibRaw version. Errors distinguish unsupported extension/input, corrupt/invalid sensor data, decoder failure, invalid RGB and non-finite output.
- Native Tauri Preview, Before/After and Export now dispatch through `DecodedSourceImage` and the same Rust shared processing graph for rendered or RAW files. RAW is explicitly reported as `LibRaw camera matrix`; a Native RAW failure is surfaced and never triggers Browser Canvas fallback.
- Added six byte-for-byte CC0 files from raw.pixls.us commit `6f997cac925e9fe7dbf2a41d8e242398d8c9d4d4`: Nikon D1 NEF, Sony DSLR-A100 ARW, Canon PowerShot S2 IS CR2, Canon PowerShot G5 X Mark II CR3, Apple iPhone SE native DNG and Fujifilm X-Pro1 X-Trans RAF. `fixtures/raw/manifest.json` records source, archive/project attribution, CC0 license, camera, format, size and SHA-256. JavaScript validation and Rust regressions reject missing/mutated/mislabeled fixtures.
- RAW regressions perform full sensor decode/demosaic for all six sources, require finite non-empty Rec.2020 output, active-area and level sanity, positive WB/Camera Neutral, correct CFA family, source immutability and pinned binary version. A shared-graph integration test measures decode time, first preview render and slider-only rerender independently; thresholds are deliberately generous CI safety ceilings, while emitted `RAW_METRIC` lines provide actual measurements.
- M2 acceptance passed on 2026-08-12: GitHub Actions push run `31513488590` and Draft PR run `31513493542` both completed green. The Windows PR run passed format, warning-denied clippy and 79 Rust tests; the full six-camera sensor regression took 48.94 seconds and the Native preview/shared-graph regression took 2.64 seconds. Web acceptance passed manifest validation, lint, 20 Vitest tests and the production build. The timing tests independently measure decoder, first-preview and slider-rerender intervals and enforce 120 s / 30 s / 30 s CI ceilings; these are safety limits rather than performance targets.
- Deliberate M2 limitation: camera color currently uses LibRaw's identified camera matrix and As-Shot path. DNG dual illuminants, ForwardMatrix, explicit D50/D65 profile resolution, Generic Profile state and persisted profile fingerprint belong to M3 and are not claimed here. The CC0 decoder fixtures validate formats, not portrait/HDR/night visual quality; Golden scene entries therefore remain honestly `planned` until matching license-cleared scenes and reviewed baselines exist.

## 2026-08-11 — M1C Native preview vertical slice

- Desktop JPEG/PNG/TIFF imports now retain the dialog-selected source path and use Rust for the actual preview: bounded decode in `starroom-imageio`, LittleCMS input transform, linear Rec.2020 D65 working graph, relative WB, exposure/tone, curve/color/detail stages, sRGB output, then JPEG encoding. Export reopens the full-resolution source and enters the same `render_shared_graph`; it never promotes preview pixels into export input.
- The Tauri 2 IPC contract deliberately keeps pixel arrays out of JSON. `native_preview` receives a small serializable request (`sourcePath`, `maxEdge`, edit state) and returns a versioned `SRP1` binary frame through `tauri::ipc::Response` (20-byte header plus JPEG payload). `native_export_jpeg` receives source/output paths and writes the encoded result directly. Tauri channels are reserved for future progressive/tiled streaming; cross-platform shared memory was rejected for this slice because ownership and lifecycle complexity are not justified before profiling.
- React/TypeScript only maps serializable UI values, parses the binary envelope and displays the returned JPEG. No new color-science or creative pixel math was added. Existing `src/imagePipeline.ts` math is marked deprecated and remains solely as an explicit `Browser fallback` for browser-hosted imports and the bundled SVG demo.
- Native failures never invoke Browser Canvas. The visible status and preview badge identify `Native CPU` or `Browser fallback`; unsupported M1C edits (Masks, Optics, Geometry, Clarity, and signed-negative detail modes) produce an explicit error instead of being ignored. Those tools remain subsequent native-graph work, not completed foundation claims.
- Before/After uses two requests to the Native graph: neutral serializable settings for Original and current settings for Edited. Native JPEG export also checks that its destination is not the source path.
- Added a shared frozen fixture at `tests/fixtures/m1c/browser-native-reference.json`. Vitest regenerates the Browser reference, while a Rust integration test compares Native CPU output against documented per-case thresholds for neutral identity, Exposure, relative WB, Tone and Curve. These tolerances describe the migration gap and must tighten rather than disappear as mature tone/WB foundations replace temporary references.
- Added official `@tauri-apps/api 2.11.1`, `@tauri-apps/plugin-dialog 2.7.2` and Rust `tauri-plugin-dialog 2.7.2` for binary IPC and scoped desktop path selection. Local Rust link tests remain blocked by the missing MSVC `link.exe`; Windows CI is authoritative for native compilation/tests.

## 2026-08-11 — F0 provenance and Golden Image specification

- Added the first reviewed third-party inventory in `docs/17_THIRD_PARTY_PROVENANCE.md`. It separates reference-only foundations from binary dependencies and records purpose, immutable upstream version/SHA, license, derivation status, binary inclusion and external-distribution risk.
- Added the Golden Image contract in `docs/18_GOLDEN_IMAGE_SPEC.md` and the machine-validated required-case manifest in `fixtures/golden/manifest.json`. All eleven required scenes carry identity, extreme-control, finite-number and tone/color-regression obligations; CPU/GPU parity is reserved as a mandatory future assertion when GPU stages arrive.
- Golden source photographs are deliberately not fabricated or downloaded without redistribution review. Entries remain `planned` until their hashes, licenses, ICC/EXIF metadata, ROIs, settings vectors and reviewed baseline artifacts exist.
- CI now validates the Golden manifest structure and required case IDs before frontend checks.

## 2026-08-11 — M1B LittleCMS provider and shared color graph

- Added the production `LittleCmsProvider` using pinned `lcms2 6.1.1`, `lcms2-sys 4.0.7` and its statically compiled LittleCMS 2.19 source (`LCMS_VERSION 2190`). The binary version has a regression assertion tied to the provenance inventory.
- Input pixels now use an embedded RGB ICC profile when present; missing profiles take the explicit, reported `assumedSrgb` fallback. Invalid embedded profiles are typed errors and do not fall through to sRGB.
- LittleCMS converts encoded input RGB through ICC PCS into a generated linear Rec.2020/D65 working profile. After shared creative/detail stages, the same graph converts working RGB to either the sRGB fallback, a supplied display ICC or a supplied export ICC.
- Native preview and export have named entry points over one `render_shared_graph` implementation. Their transform report records actual input/output profile sources and the working space.
- Tests cover Bradford D50/D65 mapping and round-trip, all four ICC rendering intents, embedded ICC equivalence, missing-profile fallback, invalid input/output profile errors, NaN/Inf rejection, display-profile preview, supplied export profile and preview/export graph identity.
- The browser Canvas pipeline remains a temporary interactive slice; connecting file import/export UI to the native Tauri graph is still M1C work and is not claimed by this provider milestone.
- Local Rust linking remains blocked by the workstation's missing Visual C++ linker. Formatting is checked locally; Windows GitHub Actions is the authoritative Clippy/compile/test gate for this native change.

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
