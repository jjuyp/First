# Local AI model provenance

This file is the M16 source-of-truth for model identity, privacy and redistribution review.
Starroom performs portrait inference locally through the Rust `ort` adapter; it makes no cloud
request, uploads no image/landmark/mask data, and requires no telemetry.

| Model | Purpose | Exact upstream pin and model URL | Local SHA-256 / size | License and usage decision | Repository / CI policy |
| --- | --- | --- | --- | --- | --- |
| OpenCV Zoo YuNet `face_detection_yunet_2026may.onnx` | Multi-face bounding boxes, confidence and five landmarks | [`opencv/opencv_zoo`](https://github.com/opencv/opencv_zoo) commit `47534e27c9851bb1128ccc0102f1145e27f23f98`; [fixed binary](https://media.githubusercontent.com/media/opencv/opencv_zoo/47534e27c9851bb1128ccc0102f1145e27f23f98/models/face_detection_yunet/face_detection_yunet_2026may.onnx) | `ebafce4e3c118d6554634be5c27ab333b4c047a9a8c3faf1d7cf93101c22f0f0`; 229,738 bytes | OpenCV Zoo model repository: MIT. **Approved** for this GPL/private-use project after the ordinary notice review. | Local `models/local/` only and Git-ignored. Never fetched at runtime; not sent to GitHub CI. A future public binary distribution must retain the model notice and recheck the exact model license. |
| yakhyo Face Parsing BiSeNet **ResNet18** `resnet18.onnx` | 19-class facial semantic logits for Face/Skin/Eyes/Brows/Lips/Mouth/Hair soft masks | [`yakhyo/face-parsing`](https://github.com/yakhyo/face-parsing) commit `8a4729d95118d0e97c44185f9bdef3d6bfeaaf99`; [release asset](https://github.com/yakhyo/face-parsing/releases/download/weights/resnet18.onnx) | `0d9bd318e46987c3bdbfacae9e2c0f461cae1c6ac6ea6d43bbe541a91727e33f`; 53,205,364 bytes | Upstream code is MIT, but pretrained-data/model provenance includes CelebAMask-HQ. **NON_COMMERCIAL_ONLY; REVIEW_REQUIRED_BEFORE_PUBLIC_RELEASE.** No ResNet34 model is used. | Local `models/local/` only and Git-ignored. Never fetched at runtime, committed, packaged, or uploaded to CI. It must be removed/replaced or separately cleared before any public release. |

## Runtime pin

`ort = 2.0.0-rc.10` (MIT OR Apache-2.0) is the single local ONNX Runtime binding. The Rust
adapter verifies each model file SHA-256 before opening a session, attempts the explicitly
requested DirectML provider, and creates a documented CPU session when DirectML is unavailable.
Missing/invalid models, invalid inference output and initialization failures have typed errors;
there is no browser, cloud or placeholder-model fallback.

## Model installation contract

Place only the two reviewed binaries in the ignored local directory (or set
`STARROOM_LOCAL_MODELS` to an alternate local-only directory):

```text
models/local/face_detection_yunet_2026may.onnx
models/local/bisenet_resnet18.onnx
```

The application verifies the hashes above. Any changed file reports `modelHashMismatch` rather
than silently accepting a different model. A no-model workstation reports an explicit unavailable
state and continues safely without portrait detection.
