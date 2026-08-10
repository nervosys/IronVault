# Model Providers & Formats - Quick Reference

## Supported Formats (23+)

### LLM Formats
- **Safetensors** (`.safetensors`) - HuggingFace standard
- **GGUF** (`.gguf`) - Quantized LLMs (Ollama, LM Studio)
- **PyTorch** (`.pt`, `.pth`, `.bin`) - Training/development
- **TensorRT** (`.plan`) - NVIDIA GPU optimized
- **ONNX** (`.onnx`) - Cross-platform
- **MLX** (`.npz`) - Apple Silicon
- **Core ML** (`.mlmodel`) - iOS/macOS
- **TorchScript** (`.pt`) - PyTorch production
- **TFLite** (`.tflite`) - Mobile/edge

### General DL
- **TensorFlow** (`.pb`), **Keras** (`.h5`)
- **OpenVINO** (`.xml`), **TVM** (`.so`)
- **NCNN** (`.param`), **MNN** (`.mnn`), **RKNN** (`.rknn`)

### Legacy
- **Caffe** (`.caffemodel`), **MXNet** (`.params`), **Darknet** (`.weights`)

### Data
- **HDF5** (`.h5`), **Pickle** (`.pkl`), **NumPy** (`.npy`)

---

## Provider → Format Map

| Provider           | Primary Format  | Use Case            |
| ------------------ | --------------- | ------------------- |
| 🤗 HuggingFace      | Safetensors     | Open-source models  |
| 🦙 Ollama           | GGUF            | Local CPU inference |
| 🎙️ LM Studio        | GGUF            | Desktop GUI         |
| 🚀 llama.cpp        | GGUF            | CPU optimization    |
| 🖼️ Stable Diffusion | Safetensors     | Image generation    |
| ⚡ TensorRT         | TensorRT Engine | NVIDIA GPU          |
| 🍎 MLX              | MLX             | Apple Silicon       |
| 📱 iOS              | Core ML         | Mobile (Apple)      |
| 🤖 Android          | TFLite          | Mobile (Google)     |
| 🔧 Intel            | OpenVINO        | Edge devices        |

---

## Format Selection Cheat Sheet

```
Desktop (NVIDIA)    → TensorRT > GGUF > Safetensors > ONNX
Apple Silicon       → MLX > Core ML > GGUF+Metal > ONNX
iOS                 → Core ML > TFLite > ONNX Mobile
Android             → TFLite > NNAPI > NCNN > MNN
Edge (Intel)        → OpenVINO > ONNX > TensorRT
Edge (ARM)          → NCNN > MNN > TFLite
Cloud               → ONNX > TensorRT > Safetensors > TorchScript
Research            → PyTorch > Safetensors > ONNX
```

---

## Common Conversions

```
PyTorch      →  Safetensors     (safetensors.torch.save_file)
Safetensors  →  GGUF            (llama.cpp convert.py)
PyTorch      →  ONNX            (torch.onnx.export)
ONNX         →  TensorRT        (trtexec)
PyTorch      →  Core ML         (coremltools.convert)
PyTorch      →  TFLite          (ai_edge_torch)
PyTorch      →  MLX             (mlx.convert)
ONNX         →  OpenVINO        (mo.py)
TensorFlow   →  TFLite          (TFLiteConverter)
```

---

## Quantization Guide

### GGUF Quantization Types

| Type   | Bits/Weight | Size (7B) | Quality   | Use Case         |
| ------ | ----------- | --------- | --------- | ---------------- |
| Q4_0   | 4.0         | ~3.5 GB   | Good      | Fast, low memory |
| Q4_K_M | 4.5         | ~4.1 GB   | Better    | **Recommended**  |
| Q5_K_M | 5.5         | ~4.8 GB   | High      | Quality balance  |
| Q8_0   | 8.0         | ~7.0 GB   | Very High | Near-original    |
| F16    | 16.0        | ~14 GB    | Original  | No loss          |

**Recommendation:** Q4_K_M for production, Q8_0 for quality-critical

---

## LLM Inference Guide

### CPU Inference (7B model)

| Format      | Quant  | RAM   | Speed       | Tool         |
| ----------- | ------ | ----- | ----------- | ------------ |
| GGUF        | Q4_K_M | 5 GB  | 10-30 tok/s | Ollama       |
| GGUF        | Q8_0   | 8 GB  | 5-15 tok/s  | llama.cpp    |
| Safetensors | F16    | 14 GB | 2-8 tok/s   | Transformers |

### GPU Inference (7B model)

| Format      | VRAM  | Speed       | Tool           |
| ----------- | ----- | ----------- | -------------- |
| TensorRT    | 8 GB  | 100+ tok/s  | TensorRT-LLM   |
| Safetensors | 14 GB | 50+ tok/s   | vLLM           |
| GGUF        | 8 GB  | 30-60 tok/s | llama.cpp CUDA |

### Apple Silicon (7B model)

| Format | RAM   | Speed       | Tool            |
| ------ | ----- | ----------- | --------------- |
| MLX    | 16 GB | 30-60 tok/s | MLX             |
| GGUF   | 16 GB | 20-40 tok/s | llama.cpp Metal |

---

## Code Examples

### Format Detection

```rust
use ironvault::formats::ModelFormat;

let format = ModelFormat::from_extension("safetensors");
println!("{}", format.name()); // "Safetensors"
```

### Model Metadata

```rust
use ironvault::formats::{ModelFormat, ModelMetadata};

let metadata = ModelMetadata::new(
    "llama-2-7b".to_string(),
    ModelFormat::Safetensors,
)
.with_framework("PyTorch".to_string())
.with_task("text-generation".to_string())
.with_parameters(7_000_000_000)
.add_custom_field("quantization".to_string(), "none".to_string())
.add_custom_field("context_length".to_string(), "4096".to_string());
```

### Format Converter

```rust
use ironvault::formats::FormatConverter;

let mut converter = FormatConverter::new();

// Register converter
converter.register(
    ModelFormat::PyTorch,
    ModelFormat::Safetensors,
    my_converter_fn
);

// Check support
if converter.can_convert(from, to) {
    let data = converter.convert(&input, from, to)?;
}
```

---

## File Extensions Reference

```
.safetensors  → Safetensors (HuggingFace)
.gguf         → GGUF (Ollama, LM Studio, llama.cpp)
.pt, .pth     → PyTorch weights
.bin          → PyTorch binary or NCNN
.onnx         → ONNX Runtime
.plan         → TensorRT Engine
.mlmodel      → Core ML (iOS/macOS)
.tflite       → TensorFlow Lite (Android)
.npz          → MLX (Apple Silicon) or NumPy
.xml + .bin   → OpenVINO IR
.param + .bin → NCNN
.mnn          → MNN (Alibaba)
.rknn         → RKNN (Rockchip)
.pb           → TensorFlow SavedModel
.h5, .keras   → Keras
```

---

## Platform-Specific Recommendations

### Windows + NVIDIA
1. TensorRT Engine (best performance)
2. GGUF Q4_K_M (CPU fallback)
3. ONNX (compatibility)

### macOS (Intel)
1. ONNX (CPU inference)
2. GGUF (llama.cpp)
3. PyTorch (development)

### macOS (Apple Silicon)
1. MLX (native optimization)
2. Core ML (on-device)
3. GGUF + Metal (llama.cpp)

### Linux + NVIDIA
1. TensorRT Engine
2. Safetensors + vLLM
3. ONNX

### Linux (CPU only)
1. GGUF Q4_K_M
2. OpenVINO (Intel)
3. ONNX

### iOS
1. Core ML (primary)
2. TFLite
3. ONNX Mobile

### Android
1. TFLite (primary)
2. NCNN (performance)
3. MNN (alternative)

---

## Best Practices Summary

✅ **DO:**
- Use Safetensors as interchange format
- Store original weights + converted variants
- Test converted models for accuracy
- Document quantization settings
- Benchmark on target hardware
- Include metadata with models

❌ **DON'T:**
- Use Pickle in production (security risk)
- Convert without validation
- Ignore quantization quality metrics
- Deploy without benchmarking
- Mix up format extensions

---

## Quick Commands

```bash
# Run providers/formats demo
cargo run --example providers_formats_demo --release

# Detect format (when CLI implemented)
iv format detect model.safetensors

# Show format info
iv format info safetensors

# List all formats
iv format list

# Convert (when implemented)
iv convert model.pt --to safetensors
iv convert model.safetensors --to gguf --quant q4_k_m
```

---

## Resources

- **HuggingFace:** https://huggingface.co/docs/safetensors
- **llama.cpp:** https://github.com/ggerganov/llama.cpp
- **ONNX:** https://onnxruntime.ai/
- **TensorRT:** https://developer.nvidia.com/tensorrt
- **MLX:** https://ml-explore.github.io/mlx/
- **Core ML:** https://developer.apple.com/documentation/coreml
- **TFLite:** https://www.tensorflow.org/lite

---

**IronVault (AIMV)** - Universal model storage supporting 23+ formats
