# 02 — GPU Render Engine

## Decision
Photo compute uses `wgpu`; Windows prefers DX12. AI inference remains separate under Windows ML/ONNX because neural graphs and slider-driven image kernels have different execution needs.

## Formats
GPU RGB: RGBA16Float default. Masks: R16Float. CPU reference: f32. Use RGBA32Float only for demonstrated precision needs.

## Stage graph
Each stage declares ID, dependencies, parameter hash, halo, precision, CPU/GPU capability. Changing a parameter invalidates that stage and downstream only. Changing mask-local exposure does not rerasterize the mask; changing brush geometry does.

## Preview pyramid
Maintain full, 1/2, 1/4, 1/8 or adaptive levels. Fit view chooses an appropriate level; 100% uses source-resolution tiles.

## Full-resolution tiling
Do not allocate multiple entire 45–100MP RGBA16F images. Start benchmarking 1024/2048 tiles. Every spatial stage declares a halo. Process halo but write interior only to avoid seams.

## Shaders
WGSL under `shaders/{common,tone,color,gamut,masks,detail,geometry,histogram}`. Rust types mirror versioned shader parameters. Fusion is allowed only when reference output stays within tolerance.

## Scheduler
Priority: visible interactive preview -> visible mask -> histogram -> settled HQ preview -> thumbnails -> background export. Cancel stale drag jobs. Never queue every slider frame.

## Workgroups
Benchmark 8x8 and 16x16 on NVIDIA/AMD/Intel iGPU/Arc rather than assuming one universal size.

## Device loss / CPU fallback
Cancel jobs, preserve project state, release caches, reinitialize once, then fall back to CPU. CPU reference uses parallel tiles/SIMD where useful.

## Memory
Evictable LRU GPU cache tiers: Low ~512MB, Normal ~1GB, High ~2GB, plus auto/user diagnostics. Authoritative project state never exists only on GPU.

## Histogram
GPU atomic 256-bin R/G/B/Luma, update 10–15Hz while dragging and once after settle.
