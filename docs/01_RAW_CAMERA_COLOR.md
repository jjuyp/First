# 01 — RAW + Camera Profile + Color Management

## Terms
RAW decoder reads proprietary compression/container and exposes sensor mosaic + metadata. CFA is the color-filter pattern. Demosaic reconstructs RGB. Camera profile maps sensor response into a defined colorimetric/working space. ICC profiles map device/color encodings through a standardized connection space.

## Decision
LibRaw handles format parsing, unpack, geometry, CFA, black/white levels, as-shot WB, camera metadata and embedded previews. Starroom owns production normalization, demosaic provider, camera profile and creative render.

## RAW pipeline
```text
RAW -> LibRaw open/unpack -> validate metadata -> active area
-> black subtraction -> linearization if present -> white normalization
-> bad pixels -> optional raw-domain AI NR -> demosaic -> as-shot WB
-> camera profile -> XYZ/working RGB -> creative graph
```
Normalize photosite as `(raw-black)/(white-black)`, where black may be channel/tile/row dependent. Preserve float headroom; do not immediately clamp 0..1.

## CFA
```rust
pub struct CfaPattern { pub period_x:u8, pub period_y:u8, pub cells:Vec<CfaColor> }
```
Must represent Bayer and X-Trans-like periodic patterns.

## DemosaicProvider
```rust
pub trait DemosaicProvider {
  fn supports(&self,cfa:&CfaPattern)->bool;
  fn required_halo(&self)->u32;
  fn demosaic(&self,input:&RawMosaic,out:&mut LinearRgbImage)->Result<()>;
}
```
Separate Bayer and generic-CFA providers are allowed. Unsupported patterns must produce a clear error, not a wrong image. Regression must cover zippering, false color, saturated edges and tile seams.

## RAW WB
Retain as-shot metadata and editable current gains separately. Save `as_shot_neutral`, source, gains and user Temp/Tint state.

## CameraProfile
Stores ID/make/model, one or two illuminants, ColorMatrix/ForwardMatrix/Calibration matrices where available, profile source and SHA-256. Resolution priority: valid embedded DNG data -> validated Starroom camera DB -> valid LibRaw camera color metadata -> explicit unsupported/profile-required status. Never invent a silent profile.

Dual-illuminant transforms must interpolate in a documented illuminant/chromaticity strategy and be chart-tested, not merely matrix-element averaged without validation.

## Working space
Baseline: unbounded linear Rec.2020 primaries, D65. Use OKLab/OKLCH for perceptual operations. ICC PCS D50/D65 adaptation must be explicit/validated.

## Rendered-file input
Embedded ICC has priority; standardized sRGB declaration next; documented metadata rules next; otherwise a documented fallback with ambiguity warning. Convert to working RGB through validated ICC transform.

## Windows display
Query display ICC using Windows Color System. Use `ColorProfileGetDisplayDefault` for modern/Advanced Color profile handling where needed. Cache transform by monitor/profile fingerprint and invalidate when the window moves monitors or the profile changes. SDR and HDR paths are distinct.

## Output / soft proof
Export sRGB and user-selected valid ICC profiles, plus validated/bundled wide-gamut profiles. Embed output profile. Architecture supports printer/output profile, rendering intent, black point compensation and gamut warning. Soft proof never mutates edit parameters.
