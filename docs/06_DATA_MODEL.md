# 06 — Data Model

Projects are schema-versioned. Authoritative state is source image + edit parameters + profile/model identities. Rebuildable caches include preview pyramid/histogram/GPU textures. AI frozen masks are reproducibility-sensitive caches.

Project records source content hash, decoder, camera-profile ID/hash, global adjustments, masks, AI denoise model ID/version/hash, and history head. Profile/model migration is explicit, never silent.

Single-image edits may use sidecar JSON. Multi-photo sessions should use SQLite indexing while keeping serialized edit schema storage-independent. History stores parameter diffs/snapshots, not full image copies.
