# Third-party provenance, version and license inventory

Status: F0 quality-gate baseline, verified 2026-08-11. Versions in this document are review pins, not permission to integrate a project. Any version change must update this file and `Cargo.lock`/`package-lock.json` in the same change.

## Foundation selections

| Project | Intended use | Upstream and fixed revision | License | Direct derivation / port | Enters shipped binary | External-distribution risk and required action |
|---|---|---|---|---|---|---|
| darktable | Quality reference for scene-referred tone, color calibration/grading, sharpening, denoise and perspective behavior | [darktable-org/darktable](https://github.com/darktable-org/darktable), `release-5.6.0`, tag object `f89bf9231fb21db0a53b3c279ff164caef48cef8` | GPL-3.0-or-later (upstream repository identifies GPL-3.0) | No code copied or ported at this gate; behavioral/reference use only | No | **High if code is copied, translated or linked.** Any direct derivation makes the affected distribution GPL-compatible and needs notices/source obligations. Record exact source files and mark `GPL-derived / private-use` before doing so. Clean-room behavioral comparison alone remains preferred. |
| LibRaw | Future mature RAW decode and camera metadata boundary | [LibRaw/LibRaw](https://github.com/LibRaw/LibRaw), `0.22.2`, tag object `24fa7e5463cbf8b8615dbd2b16c933a294d52400` | LGPL-2.1-or-later **or** CDDL-1.0; bundled third-party demosaic portions have additional permissive notices | No | No, planned | **Medium.** Choose and document one license path, retain notices, expose relinking/source as required for LGPL if linked, and audit optional demosaic packs separately. Do not imply LibRaw supplies production-quality rendering beyond decode. |
| LittleCMS | ICC v2/v4 input, working-space, display and output transforms | [mm2/Little-CMS](https://github.com/mm2/Little-CMS), vendored 2.19 content at `21c582a594fe5279f90c0b93437c398f93bf62b0` (`LCMS_VERSION 2190`) through `lcms2-sys 4.0.7` | MIT | No algorithm copied; linked through the safe Rust provider | **Yes**, statically compiled by the `lcms2` `static` feature | **Low.** Ship the LittleCMS and Rust-wrapper MIT notices. Security/version review is required before bumping the C engine or wrapper. |
| Lensfun | Future lens identification, distortion, lateral CA and vignetting correction | [lensfun/lensfun](https://github.com/lensfun/lensfun), `v0.3.4`, tag object `101c745e847a5de4a1e569a94368ce2027198598` | LGPL-3.0-or-later for library; lens database/content must be audited separately | No | No, planned | **Medium/high.** Dynamic linking is preferred for external distribution. Preserve LGPL notices and relinking rights; separately verify database licensing and attribution before bundling it. |
| MediaPipe | Future local face landmarks / portrait masks provider | [google-ai-edge/mediapipe](https://github.com/google-ai-edge/mediapipe), `v1.0.0`, tag object `6d31f1ebc3284db74d211d62bdc4f0a0c29ea120` | Apache-2.0 | No | No, planned | **Medium.** Retain LICENSE/NOTICE, review model licenses separately, document telemetry/privacy behavior for the selected runtime, and do not bundle a model merely because framework code is Apache-2.0. |
| colour-science | Offline validation oracle for matrices, chromatic adaptation and ColorChecker metrics; not runtime image processing | [colour-science/colour](https://github.com/colour-science/colour), `v0.4.7`, tag object `7082a604a6d988d314576c0343c7f5008b4b5171` | BSD-3-Clause | No; reference calculations/tests only | No | **Low.** Retain attribution if fixtures or generated reference data are distributed; record the generating script, version and numeric precision. |

## Current Rust image/color dependency closure

The authoritative resolved versions are in `Cargo.lock`. “Direct” means declared by a Starroom crate; transitive codecs still enter the binary and therefore remain listed.

| Dependency | Purpose / Starroom integration | Fixed version | License | Derivation | Binary | Distribution note |
|---|---|---:|---|---|---|---|
| `lcms2` | Safe ICC provider API in `starroom-color-management` | 6.1.1, tag `03972e3b4e6a3e7ebc76765079a98d5e6f8c6b9a` | MIT | No | Yes | Ship MIT notice. |
| `lcms2-sys` | FFI and static LittleCMS build | 4.0.7, tag `2aff9f7ac9576327efbb112de3f6ec1adf1aa2af` | MIT; vendored LittleCMS MIT | No | Yes | Ship both wrapper and LittleCMS notices; static build is intentional for reproducibility. |
| `image` | Rendered JPEG/PNG/TIFF decode and JPEG encode in `starroom-imageio` | 0.25.10 | MIT OR Apache-2.0 | No | Yes | Record chosen notice set; enabled features are exactly `jpeg`, `png`, `tiff`. |
| `zune-jpeg`, `zune-core` | JPEG codec selected transitively by `image` | 0.5.15 / 0.5.3 | MIT OR Apache-2.0 OR Zlib | No | Yes | Retain chosen permissive notices. |
| `png` | PNG codec; `0.18.1` is in the image path and `0.17.16` is also resolved elsewhere in the desktop closure | 0.18.1 and 0.17.16 | MIT OR Apache-2.0 | No | Yes | Keep both resolved versions visible until dependency deduplication. |
| `tiff` | TIFF codec selected transitively by `image` | 0.11.3 | MIT | No | Yes | Retain MIT notice. |
| `moxcms`, `pxfm` | Numeric/color support selected transitively by `image` | 0.8.1 / 0.1.30 | BSD-3-Clause OR Apache-2.0 | No | Yes | This is not Starroom's ICC provider; retain chosen notices. |
| `fdeflate`, `flate2`, `miniz_oxide` | PNG/DEFLATE implementation closure | 0.3.7 / 1.1.9 / 0.8.9 | permissive multi-license | No | Yes | Retain the selected MIT/Apache/Zlib notices. |
| `byteorder-lite` | TIFF/image byte-order support | 0.1.0 | Unlicense OR MIT | No | Yes | Prefer MIT notice for predictable external distribution records. |

## Current JavaScript image-related dependencies

There is currently **no third-party JavaScript image-processing dependency**. The browser slice uses platform `CanvasRenderingContext2D`, `Image`, `ImageData`, Blob and object-URL APIs in `src/imagePipeline.ts` and `src/App.tsx`. React, Lucide, Vite and test/lint packages are UI/tooling dependencies, not image algorithms. Their exact versions remain pinned by `package-lock.json` and require the normal release notice audit, but they are outside this image-foundation inventory.

## Provenance rule for future integrations

Every new adapter or port must add: upstream file/module, immutable tag/SHA, original license header, integration mode (`link`, `adapter`, `behavioral reference`, `direct port`, or `generated reference data`), Starroom destination files, binary/data packaging, modifications, and external-distribution decision. “Looked at upstream” is not enough to classify a direct translation as independent work.
