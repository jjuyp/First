# Dependency Baseline Verified 2026-08-09

- LibRaw stable observed: 0.22.2. Its official site says RAW extraction/metadata are core and basic post-processing is not production-rendering focus.
- Adobe DNG page lists DNG 1.7.1.0 specification and DNG SDK 1.7.1 current 2026 builds.
- ICC current v4 page lists ICC.1:2022, profile version 4.4.0.0.
- LittleCMS official site lists 2.19 released 2026-04-17.
- wgpu docs observed 30.0.0; native Rust API with DX12/Vulkan on Windows.
- Microsoft positions Windows ML as unified local ONNX inference. Stable package observed 2.1.74 (2026-07-13) with ORT 1.24.6; hardware-optimized EP catalog availability depends on Windows 11 24H2+, device and drivers.

Codex must re-check official current releases before pinning and then freeze validated versions in lock/build files.
