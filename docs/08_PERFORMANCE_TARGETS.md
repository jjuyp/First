# 08 — Performance Targets

Engineering targets: 60FPS UI; Fit-view slider visible response ideally <=50ms; stale jobs cancelled; histogram 10–15Hz during drag. RAW open may show embedded preview first, progressively replaced without UI blocking.

AI Mask uses analysis resolution appropriate to model, normally <=2048 long edge when valid. AI denoise previews visible/crop region first; full image is tiled with visible progress.

GPU working-cache tiers: 512MB/1GB/2GB. Benchmarks cover 24/45/60/100MP and Intel iGPU, Intel Arc, NVIDIA midrange, AMD midrange and CPU-only fallback. Record first preview, exposure drag, five-mask composite, AI mask, AI denoise, export, RAM and GPU allocation estimate.
