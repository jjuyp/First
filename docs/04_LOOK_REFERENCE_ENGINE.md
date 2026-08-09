# 04 — Look Engine + Style Mixer + Reference Match

## Look Engine
High-level `LookDescriptor` axes: Clean↔Moody, Soft↔Crisp, Warm↔Cool, Natural↔Cinematic, Airy↔Rich, Modern↔Vintage, normalized -1..1.

Initial mapping is deterministic and independently authored: `theta = theta0 + B * phi(z)`, where `B` is a hand-authored parameter basis and `phi` includes selected safe cross terms. Looks affect curve/HSL/grading/density/presence unless explicitly allowed; crop/geometry/source profile/masks are protected by default.

`.srlook` stores schema version, name, author, engine compatibility, parameter deltas, curve/grading, protected categories, amount and mixer metadata.

## Amount / Style Mixer
Interpolation is parameter-aware:
- additive scalars: linear deltas
- gains: log-domain
- hue: circular
- curves: sample 64/128 points -> weighted blend -> enforce monotonic constraints -> reconstruct spline
- grading: perceptual opponent-vector interpolation
- booleans/categories: explicit policy

Style Mixer normalizes weights. By default it excludes absolute exposure/WB, crop/rotate/geometry, masks and lens correction.

## Reference Match
Goal is look transfer, not scene/content reproduction. No spatial correspondence is assumed.

Preprocess target/reference into the same working space and 1024-ish analysis previews, OKLab/OKLCH copies, clipped masks and optional semantic masks.

Features:
- luminance percentiles 1/5/10/25/50/75/90/95/99, median, robust contrast, shoulder/toe
- neutral candidate a/b center for WB
- smooth hue-band mass/median hue/chroma/lightness
- shadow/mid/high perceptual color centroids
- chroma-vs-lightness density relation
- optional skin/sky/vegetation/subject/background stats when both images have confident matching semantics

Staged solver:
1. exposure from robust midtone quantiles
2. WB from neutral candidates; low confidence means no forced WB
3. monotonic tone curve from sparse quantile mapping (5/25/50/75/95) with slope limits
4. selective color by supported hue bands only
5. grading from shadow/mid/high centroids
6. deterministic bounded coordinate refinement

Objective combines luminance distance, color-distribution distance, grading distance, optional semantic distance, regularization and protection penalties. Penalties prevent clipping growth, skin oversaturation, extreme hue rotation and invalid curves.

Modes: Tone, Color, Full Look. Never match crop/geometry/lens/mask geometry. Strength 0–100 uses the same parameter-aware mixer and 0% must exactly reproduce pre-match state.

Explain report lists parameter changes and per-subsystem confidence. Closed-loop tests generate a reference by applying known Starroom edits and test recovery.
