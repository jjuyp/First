# 09 — Dependency / Model / License Policy

Verified architecture baseline, not automatic version pins:
- LibRaw: RAW decode/metadata behind abstraction; comply with selected dual-license mode.
- LittleCMS: ICC/soft-proof/gamut; official project states MIT.
- wgpu: GPU abstraction; pin an exact regression-tested release.
- Windows ML/ONNX: local neural inference behind native bridge; offline CPU fallback mandatory.

Before bundling any model verify code license, weight/checkpoint license, relevant usage restrictions, source URL, SHA-256, redistribution/commercial status and attribution/NOTICE requirements. Open-source code does not automatically clear model weights.

SAM2 is an interactive-segmentation reference candidate with official Apache-2.0 project/checkpoint statement. SwinIR official repo states Apache-2.0 and is an RGB-denoise reference. NAFNet is benchmark material until license/checkpoint redistribution is explicitly cleared.

Do not choose Starroom's own public-source license until business model is decided.
