# 07 — Test / Validation

Fixtures: synthetic gradients/charts, permissively licensed photos, privately created RAW samples. RAW tests verify geometry/CFA/black-white/WB/profile/demosaic/highlights/color. Color tests cover ICC v2/v4, sRGB/wide-gamut, untagged fallback, monitor transforms and neutral preservation.

GPU stages compare against CPU reference with stage-specific numeric/perceptual tolerances. Masks test bounds/feather/boolean ops/persistence/tile seams. AI Mask uses IoU/boundary score, latency/memory and manual visual gates. AI Denoise measures PSNR/SSIM plus texture, edge halos, chroma artifacts, skin and small-detail preservation.

Reference Match has closed-loop known-look recovery plus different-content tests. Fuzz corrupt RAW/ICC/EXIF/project/model manifests and absurd dimensions.

No public release with source overwrite, broken color management, silent profile fallback, broken undo, frequent supported-GPU crashes, or unreproducible AI project state without warning.
