# Supported AI Model Formats

**IronVault** supports all major AI model formats for both Neural and Neurosymbolic AI models. Formats are automatically detected from file extensions, or can be explicitly specified.

## ✅ Currently Implemented Formats

### LLM-Centric Formats

| Format          | Extensions              | Description                          | Use Case                                       |
| --------------- | ----------------------- | ------------------------------------ | ---------------------------------------------- |
| **Safetensors** | `.safetensors`          | HuggingFace default for Transformers | Safe serialization, LLM checkpoints            |
| **GGUF**        | `.gguf`                 | Quantized LLM format                 | Local inference (llama.cpp, LM Studio, Ollama) |
| **PyTorch**     | `.pt`, `.pth`, `.bin`   | PyTorch state_dict files             | Training, research, production                 |
| **TensorRT**    | `.plan`                 | NVIDIA compiled engines              | Production deployment on NVIDIA GPUs           |
| **ONNX**        | `.onnx`                 | Interchange/serving format           | Cross-framework deployment                     |
| **MLX**         | `.npz`                  | Apple Silicon optimized              | Local inference on Apple devices               |
| **Core ML**     | `.mlmodel`              | iOS/macOS format                     | On-device inference (iPhone, iPad, Mac)        |
| **TorchScript** | `.pt` (traced/scripted) | PyTorch serialization                | Production PyTorch models                      |
| **TFLite**      | `.tflite`               | TensorFlow Lite                      | Mobile/edge deployment                         |

### General Deep Learning Formats

| Format         | Extensions       | Description               | Use Case                |
| -------------- | ---------------- | ------------------------- | ----------------------- |
| **TensorFlow** | `.pb`            | SavedModel format         | TensorFlow serving      |
| **Keras**      | `.h5`, `.keras`  | Keras model format        | Keras/TensorFlow models |
| **OpenVINO**   | `.xml`, `.bin`   | Intel optimization format | CPU/iGPU deployment     |
| **TVM**        | `.so`            | TVM compiled artifacts    | Custom accelerators     |
| **NCNN**       | `.param`, `.bin` | Mobile-optimized          | Android/embedded        |
| **MNN**        | `.mnn`           | Mobile Neural Network     | Mobile deployment       |
| **RKNN**       | `.rknn`          | Rockchip NPU format       | Rockchip devices        |

### Legacy Formats

| Format      | Extensions    | Description         | Status         |
| ----------- | ------------- | ------------------- | -------------- |
| **Caffe**   | `.caffemodel` | Caffe format        | Legacy support |
| **MXNet**   | `.params`     | Apache MXNet        | Legacy support |
| **Darknet** | `.weights`    | YOLO/Darknet format | Legacy support |

### Data Formats

| Format     | Extensions     | Description              | Use Case                     |
| ---------- | -------------- | ------------------------ | ---------------------------- |
| **HDF5**   | `.h5`, `.hdf5` | Hierarchical data format | Scientific data, checkpoints |
| **Pickle** | `.pkl`         | Python pickle            | Python object serialization  |
| **NumPy**  | `.npy`, `.npz` | NumPy arrays             | Numerical data               |

## Usage Examples

### Auto-Detection (Recommended)

```bash
# Format automatically detected from extension
iv store my-llm model.safetensors
iv store quantized-model model.gguf
iv store mobile-model model.tflite
iv store apple-model model.mlmodel
```

### Explicit Format Specification

```bash
# Explicitly specify format
iv store my-model model.bin --format pytorch
iv store my-model model.bin --format onnx
iv store my-model model.bin --format tensorrt
```

### Supported Format Names

When using `--format`, you can use these names:

**LLM Formats:**
- `safetensors`
- `gguf`
- `pytorch`, `pt`, `torch`
- `tensorrt`, `trt`
- `onnx`
- `mlx`
- `coreml`, `mlmodel`
- `torchscript`
- `tflite`, `tensorflow-lite`

**General DL:**
- `tensorflow`, `tf`, `savedmodel`
- `keras`, `h5`
- `openvino`
- `tvm`
- `ncnn`
- `mnn`
- `rknn`

**Legacy:**
- `caffe`
- `mxnet`
- `darknet`

**Data:**
- `hdf5`
- `pickle`, `pkl`
- `numpy`, `npy`

## Format Detection Priority

1. **Explicit `--format` flag** (highest priority)
2. **File extension** (automatic detection)
3. **Custom format** (fallback for unknown extensions)

## Platform-Specific Recommendations

### Server/Cloud (NVIDIA GPUs)
- Production: **TensorRT** (.plan)
- Serving: **ONNX** (.onnx)
- Development: **PyTorch** (.pt), **Safetensors** (.safetensors)

### Apple Silicon (M1/M2/M3)
- Local LLMs: **MLX** (.npz), **GGUF** (.gguf)
- On-device: **Core ML** (.mlmodel)
- Development: **PyTorch** (.pt)

### Mobile (Android/iOS)
- Android: **TFLite** (.tflite), **NCNN** (.param)
- iOS: **Core ML** (.mlmodel), **TFLite** (.tflite)

### Edge/Embedded
- General: **TFLite** (.tflite)
- Rockchip NPU: **RKNN** (.rknn)
- Intel CPU/iGPU: **OpenVINO** (.xml)
- ARM devices: **NCNN** (.param), **MNN** (.mnn)

### Local LLM Inference
- Quantized: **GGUF** (.gguf)
- Full precision: **Safetensors** (.safetensors)
- Apple: **MLX** (.npz)

## Notes on Format Popularity

Based on real-world usage as of October 2025:

### Most Popular LLM Formats
1. **Safetensors** (.safetensors) — HuggingFace default, emphasized in HF docs
2. **GGUF** (.gguf) — Dominant for local inference, six-figure catalog on HF
3. **PyTorch** (.pt/.pth/.bin) — Classic format, still everywhere on HF Hub

### Most Popular Production Formats
1. **TensorRT** (.plan) — NVIDIA production deployments
2. **ONNX** (.onnx) — Cross-framework interchange
3. **TFLite** (.tflite) — Multi-billion device reach (mobile/edge)

### Platform-Specific Leaders
- **Apple ecosystem**: Core ML, MLX
- **Mobile (Android)**: TFLite, NCNN
- **Intel hardware**: OpenVINO
- **Embedded**: TFLite, NCNN, MNN, RKNN

### Legacy but Still Encountered
- **Caffe** (.caffemodel) — Computer vision legacy
- **MXNet** (.params) — Apache MXNet projects
- **Darknet** (.weights) — YOLO models

## Future Format Support

IronVault is designed to be extensible. Additional formats can be added through:
1. Extending the `ModelFormat` enum in `src/formats.rs`
2. Adding extension mappings in `from_extension()`
3. Adding CLI format name mappings in `src/main.rs`

## References

* [HuggingFace Transformers Documentation](https://huggingface.co/docs/transformers)
* [GGUF Models on HuggingFace](https://huggingface.co/models?library=gguf)
* [ONNX Model Zoo](https://onnx.ai/models/)
* [Apple Core ML Documentation](https://developer.apple.com/documentation/coreml)
* [TensorFlow Lite Guide](https://www.tensorflow.org/lite)
* [OpenVINO Documentation](https://docs.openvino.ai/)

---

**Note**: While IronVault supports storing all these formats securely with encryption, version control, and compliance tracking, format *conversion* between formats is a separate feature that may require additional tooling (e.g., `optimum`, `onnx`, `coremltools`).


1. **Safetensors (.safetensors)** — the default on Hugging Face for new Transformer checkpoints; emphasized by HF docs (safe serialization on by default). ([Hugging Face][1])
2. **GGUF (.gguf)** — the dominant local-inference/quantized LLM format (llama.cpp, LM Studio, Ollama imports, etc.); Hugging Face has a dedicated “GGUF” library filter with a very large (six-figure) catalog. ([Hugging Face][2])
3. **PyTorch weights (.bin/.pt/.pth)** — classic Transformers/PyTorch state_dict files (still everywhere on the Hub). HF explicitly positions Transformers as the primary library with a massive checkpoint catalog. ([Hugging Face][3])
4. **TensorRT Engines (.plan)** — compiled LLM engines for NVIDIA deployment; widely used in production for latency/throughput. (Ecosystem-driven; no single central count.)
5. **ONNX (.onnx)** — common interchange/serving format; momentum has shifted toward HF for sharing and away from the legacy ONNX Model Zoo (now archived), but ONNX remains a key export/serve target. ([ONNX][4])
6. **MLX (Apple) checkpoints** — rising for Apple silicon local LLMs; supported as a library on the Hub. ([Hugging Face][2])
7. **Core ML (.mlmodel)** — used for on-device iOS/macOS inference after conversion. ([Apple Developer][5])
8. **TorchScript (.pt/.pth traced/scripted)** — still around, but eclipsed by eager + export pipelines.
9. **TFLite (.tflite)** — used for on-device/mobile LLM variants; huge device footprint overall (billions of devices run TFLite). ([ProX PC][6])

## General DL / CV / speech formats (most → less common)

1. **PyTorch weights (.pt/.pth/.bin)** — the prevailing format for training/research and many production pipelines (especially via HF Transformers). ([Hugging Face][3])
