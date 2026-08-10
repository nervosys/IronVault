//! Comprehensive demonstration of model providers and format conversions
//!
//! This example showcases:
//! - All supported model formats (23+ formats)
//! - Format detection and conversion capabilities
//! - Model provider ecosystem (HuggingFace, Ollama, LM Studio, etc.)
//! - Format conversion paths and use cases
//! - Best practices for different deployment targets

use ironvault::formats::{FormatConverter, ModelFormat, ModelMetadata};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    print_header();

    // Step 1: Show all supported formats
    demonstrate_model_formats();

    // Step 2: Show model providers and their formats
    demonstrate_model_providers();

    // Step 3: Show format conversion paths
    demonstrate_format_conversions();

    // Step 4: Show deployment targets
    demonstrate_deployment_targets();

    // Step 5: Show format use cases
    demonstrate_use_cases();

    // Step 6: Show metadata handling
    demonstrate_metadata();

    // Step 7: Show conversion registry
    demonstrate_converter_registry();

    print_footer();

    Ok(())
}

fn print_header() {
    println!("\n{}", "=".repeat(70));
    println!("  IronVault (AIMV) - Model Providers & Formats Demo");
    println!("{}\n", "=".repeat(70));
}

fn print_separator(title: &str) {
    println!("\n{}", "─".repeat(70));
    println!("  {}", title);
    println!("{}\n", "─".repeat(70));
}

fn print_footer() {
    println!("\n{}", "=".repeat(70));
    println!("  AIMV Model Providers & Formats Demo Complete!");
    println!("{}\n", "=".repeat(70));

    println!("Key Takeaways:");
    println!("  [+] 23+ model formats supported");
    println!("  [+] Coverage across LLMs, computer vision, mobile, edge");
    println!("  [+] Flexible format conversion system");
    println!("  [+] Provider-agnostic storage");
    println!("  [+] Optimized for different deployment targets");
    println!();
}

fn demonstrate_model_formats() {
    print_separator("Step 1: Supported Model Formats");

    println!("🤖 LLM-Centric Formats:");
    println!();

    let llm_formats = vec![
        (
            ModelFormat::Safetensors,
            "HuggingFace default for Transformers",
            "safetensors",
        ),
        (
            ModelFormat::GGUF,
            "Quantized LLM format (llama.cpp, LM Studio, Ollama)",
            "gguf",
        ),
        (
            ModelFormat::PyTorch,
            "Classic state_dict files",
            "pt, pth, bin",
        ),
        (ModelFormat::TensorRT, "NVIDIA compiled engines", "plan"),
        (ModelFormat::ONNX, "Interchange/serving format", "onnx"),
        (ModelFormat::MLX, "Apple Silicon optimized format", "npz"),
        (
            ModelFormat::CoreML,
            "iOS/macOS on-device inference",
            "mlmodel",
        ),
        (ModelFormat::TorchScript, "PyTorch serialization", "pt"),
        (ModelFormat::TFLite, "Mobile/edge deployment", "tflite"),
    ];

    for (format, description, extensions) in llm_formats {
        println!("  📦 {:<18} → {}", format.name(), description);
        println!("     Extensions: {}", extensions);
        println!();
    }

    println!("🧠 General Deep Learning Formats:");
    println!();

    let dl_formats = vec![
        (ModelFormat::TensorFlow, "TensorFlow serving format", "pb"),
        (ModelFormat::Keras, "Keras model format", "h5, keras"),
        (
            ModelFormat::OpenVINO,
            "Intel optimization format",
            "xml + bin",
        ),
        (ModelFormat::TVM, "Compiled artifacts", "so"),
        (ModelFormat::NCNN, "Mobile-optimized format", "param + bin"),
        (ModelFormat::MNN, "Mobile Neural Network format", "mnn"),
        (ModelFormat::RKNN, "Rockchip NPU format", "rknn"),
    ];

    for (format, description, extensions) in dl_formats {
        println!("  📦 {:<18} → {}", format.name(), description);
        println!("     Extensions: {}", extensions);
        println!();
    }

    println!("🏛️  Legacy Formats:");
    println!();

    let legacy_formats = vec![
        (ModelFormat::Caffe, "Legacy Caffe format", "caffemodel"),
        (ModelFormat::MXNet, "Apache MXNet format", "params"),
        (ModelFormat::Darknet, "YOLO/Darknet format", "weights"),
    ];

    for (format, description, extensions) in legacy_formats {
        println!("  📦 {:<18} → {}", format.name(), description);
        println!("     Extensions: {}", extensions);
        println!();
    }

    println!("💾 Data Formats:");
    println!();

    let data_formats = vec![
        (ModelFormat::HDF5, "Hierarchical data format", "h5, hdf5"),
        (ModelFormat::Pickle, "Python pickle format", "pkl"),
        (ModelFormat::NumPy, "NumPy array format", "npy, npz"),
    ];

    for (format, description, extensions) in data_formats {
        println!("  📦 {:<18} → {}", format.name(), description);
        println!("     Extensions: {}", extensions);
        println!();
    }

    // Test format detection
    println!("🔍 Format Detection Examples:");
    println!();

    let test_files = vec![
        "model.safetensors",
        "llama-7b-q4.gguf",
        "bert-base.pt",
        "yolov8.onnx",
        "gpt2.tflite",
        "stable-diffusion.mlmodel",
        "resnet50.plan",
    ];

    for file in test_files {
        let ext = file.split('.').next_back().unwrap();
        let format = ModelFormat::from_extension(ext);
        println!("  {} → {}", file, format.name());
    }
    println!();
}

fn demonstrate_model_providers() {
    print_separator("Step 2: Model Provider Ecosystem");

    println!("🤗 HuggingFace Hub");
    println!("   Primary Format: Safetensors (.safetensors)");
    println!("   Legacy: PyTorch (.bin, .pt)");
    println!("   Examples:");
    println!("   • meta-llama/Llama-2-7b-hf → safetensors");
    println!("   • bert-base-uncased → safetensors");
    println!("   • stable-diffusion-v1-5 → safetensors");
    println!("   Benefits: Fast loading, security (no pickle), memory mapping");
    println!();

    println!("🦙 Ollama");
    println!("   Primary Format: GGUF (.gguf)");
    println!("   Examples:");
    println!("   • llama2:7b → gguf (Q4_0 quantization)");
    println!("   • mistral:7b → gguf (Q4_K_M quantization)");
    println!("   • codellama:13b → gguf");
    println!("   Benefits: Quantization, CPU inference, low memory");
    println!();

    println!("🎙️  LM Studio");
    println!("   Primary Format: GGUF (.gguf)");
    println!("   Examples:");
    println!("   • TheBloke/Llama-2-7B-GGUF → Q4_K_M, Q5_K_M, Q8_0");
    println!("   • TheBloke/Mistral-7B-GGUF → various quantizations");
    println!("   Benefits: Desktop GUI, quantization options, local inference");
    println!();

    println!("🚀 llama.cpp");
    println!("   Primary Format: GGUF (.gguf)");
    println!("   Legacy: GGML (.bin)");
    println!("   Examples:");
    println!("   • ggml-model-q4_0.gguf → 4-bit quantization");
    println!("   • ggml-model-q8_0.gguf → 8-bit quantization");
    println!("   Benefits: C++ native, fast CPU inference, quantization");
    println!();

    println!("🖼️  Stable Diffusion / ComfyUI");
    println!("   Primary Format: Safetensors (.safetensors)");
    println!("   Legacy: PyTorch (.ckpt, .pt)");
    println!("   Examples:");
    println!("   • sd-v1-5.safetensors → Stable Diffusion 1.5");
    println!("   • sd-xl-base.safetensors → SDXL base model");
    println!("   • controlnet.safetensors → ControlNet weights");
    println!("   Benefits: Safe loading, fast, widely supported");
    println!();

    println!("⚡ NVIDIA TensorRT");
    println!("   Primary Format: TensorRT Engine (.plan)");
    println!("   Examples:");
    println!("   • llm.plan → Optimized LLM engine");
    println!("   • resnet50-fp16.plan → Computer vision engine");
    println!("   Benefits: Maximum GPU performance, optimized kernels");
    println!();

    println!("🍎 Apple MLX");
    println!("   Primary Format: MLX (.npz)");
    println!("   Examples:");
    println!("   • llama-7b-mlx.npz → Apple Silicon optimized");
    println!("   • mistral-7b-mlx.npz → M1/M2/M3 optimized");
    println!("   Benefits: Unified memory, Apple Silicon acceleration");
    println!();

    println!("📱 Mobile Deployment");
    println!("   iOS/macOS: Core ML (.mlmodel)");
    println!("   Android: TensorFlow Lite (.tflite)");
    println!("   Cross-platform: ONNX Runtime (.onnx)");
    println!("   Examples:");
    println!("   • mobilenet-v3.tflite → Android on-device inference");
    println!("   • yolov5-coreml.mlmodel → iOS object detection");
    println!("   • bert-base.onnx → Cross-platform NLP");
    println!();

    println!("🔧 Edge & Embedded");
    println!("   NCNN: Mobile-optimized (.param + .bin)");
    println!("   MNN: Alibaba mobile framework (.mnn)");
    println!("   OpenVINO: Intel optimization (.xml + .bin)");
    println!("   RKNN: Rockchip NPU (.rknn)");
    println!("   Examples:");
    println!("   • yolov5-ncnn.param → Mobile object detection");
    println!("   • resnet50-openvino.xml → Intel CPU/GPU/VPU");
    println!();
}

fn demonstrate_format_conversions() {
    print_separator("Step 3: Format Conversion Paths");

    println!("🔄 Common Conversion Scenarios:");
    println!();

    let conversions = vec![
        (
            "HuggingFace → Ollama",
            "Safetensors → GGUF",
            "llama.cpp quantization tools",
        ),
        (
            "HuggingFace → TensorRT",
            "Safetensors → TensorRT Engine",
            "trtllm-build, TensorRT-LLM",
        ),
        (
            "HuggingFace → ONNX",
            "Safetensors/PyTorch → ONNX",
            "transformers.onnx.export()",
        ),
        (
            "HuggingFace → Core ML",
            "PyTorch → Core ML",
            "coremltools.convert()",
        ),
        ("HuggingFace → TFLite", "PyTorch → TFLite", "ai_edge_torch"),
        (
            "PyTorch → Safetensors",
            "PyTorch .bin → Safetensors",
            "safetensors.torch.save_file()",
        ),
        ("ONNX → TensorRT", "ONNX → TensorRT Engine", "trtexec"),
        (
            "PyTorch → TorchScript",
            "PyTorch → TorchScript",
            "torch.jit.trace()",
        ),
        (
            "TensorFlow → TFLite",
            "SavedModel → TFLite",
            "TFLiteConverter",
        ),
        ("PyTorch → MLX", "PyTorch → MLX", "mlx.convert()"),
    ];

    for (scenario, conversion, tool) in conversions {
        println!("  📊 {}", scenario);
        println!(
            "     {} → {}",
            conversion.split(" → ").next().unwrap(),
            conversion.split(" → ").last().unwrap()
        );
        println!("     Tool: {}", tool);
        println!();
    }

    println!("🎯 Conversion Workflow Examples:");
    println!();

    println!("  1️⃣  Training → Production LLM Serving:");
    println!("     PyTorch (.pt) → Safetensors → GGUF (Q4_K_M) → Ollama");
    println!("     or");
    println!("     PyTorch (.pt) → ONNX → TensorRT Engine → vLLM");
    println!();

    println!("  2️⃣  Research Model → Mobile App:");
    println!("     HuggingFace (Safetensors) → ONNX → TFLite/Core ML");
    println!("     Optimization: Quantization (INT8), pruning");
    println!();

    println!("  3️⃣  Image Model → Edge Device:");
    println!("     PyTorch (.pt) → ONNX → OpenVINO IR → Intel NUC");
    println!("     or");
    println!("     PyTorch (.pt) → NCNN → Mobile/Embedded");
    println!();

    println!("  4️⃣  LLM → Apple Silicon:");
    println!("     HuggingFace → PyTorch → MLX → M1/M2/M3 Mac");
    println!("     Benefits: Unified memory, GPU acceleration");
    println!();
}

fn demonstrate_deployment_targets() {
    print_separator("Step 4: Deployment Targets & Format Selection");

    let targets = vec![
        (
            "🖥️  Desktop/Server (NVIDIA GPU)",
            vec![
                "TensorRT Engine (.plan) - Best performance",
                "GGUF (.gguf) - CPU fallback with quantization",
                "Safetensors (.safetensors) - Original weights",
                "ONNX (.onnx) - Cross-platform compatibility",
            ],
        ),
        (
            "🍎 Apple Silicon (M1/M2/M3)",
            vec![
                "MLX (.npz) - Native Apple Silicon optimization",
                "Core ML (.mlmodel) - On-device inference",
                "GGUF (.gguf) - llama.cpp with Metal acceleration",
                "ONNX (.onnx) - ONNX Runtime CoreML",
            ],
        ),
        (
            "📱 Mobile (iOS)",
            vec![
                "Core ML (.mlmodel) - Primary choice",
                "TensorFlow Lite (.tflite) - TFLite on iOS",
                "ONNX Mobile (.onnx) - ONNX Runtime Mobile",
            ],
        ),
        (
            "📱 Mobile (Android)",
            vec![
                "TensorFlow Lite (.tflite) - Primary choice",
                "NNAPI (.onnx) - Android Neural Networks API",
                "NCNN (.param) - Tencent mobile framework",
                "MNN (.mnn) - Alibaba mobile framework",
            ],
        ),
        (
            "🎮 Edge Devices",
            vec![
                "OpenVINO (.xml) - Intel CPU/GPU/VPU/NCS2",
                "TensorRT (.plan) - NVIDIA Jetson",
                "RKNN (.rknn) - Rockchip NPU (RK3588)",
                "NCNN (.param) - ARM mobile processors",
            ],
        ),
        (
            "☁️  Cloud Inference",
            vec![
                "ONNX (.onnx) - Triton Inference Server",
                "TensorRT (.plan) - NVIDIA Triton",
                "Safetensors (.safetensors) - HuggingFace TGI",
                "TorchScript (.pt) - TorchServe",
            ],
        ),
        (
            "🧪 Research/Development",
            vec![
                "PyTorch (.pt) - Training and experimentation",
                "Safetensors (.safetensors) - Model sharing",
                "ONNX (.onnx) - Format evaluation",
                "Pickle (.pkl) - Prototyping (not production!)",
            ],
        ),
    ];

    for (target, formats) in targets {
        println!("{}", target);
        for format in formats {
            println!("  • {}", format);
        }
        println!();
    }
}

fn demonstrate_use_cases() {
    print_separator("Step 5: Format Selection by Use Case");

    println!("🎯 LLM Inference:");
    println!();
    println!("  Local CPU (Low Memory):");
    println!("  → GGUF with Q4_K_M quantization");
    println!("  → Tools: Ollama, LM Studio, llama.cpp");
    println!();
    println!("  Local CPU (High Quality):");
    println!("  → GGUF with Q8_0 quantization");
    println!("  → Safetensors (if enough RAM)");
    println!();
    println!("  GPU Inference (NVIDIA):");
    println!("  → TensorRT Engine (best performance)");
    println!("  → Safetensors with vLLM/TGI");
    println!();
    println!("  GPU Inference (Apple Silicon):");
    println!("  → MLX format");
    println!("  → GGUF with Metal acceleration");
    println!();

    println!("🖼️  Computer Vision:");
    println!();
    println!("  Server Inference:");
    println!("  → TensorRT Engine (NVIDIA)");
    println!("  → ONNX Runtime (cross-platform)");
    println!();
    println!("  Mobile Deployment:");
    println!("  → Core ML (iOS)");
    println!("  → TensorFlow Lite (Android)");
    println!();
    println!("  Edge Devices:");
    println!("  → OpenVINO (Intel)");
    println!("  → NCNN/MNN (ARM)");
    println!();

    println!("🗣️  Speech/Audio:");
    println!();
    println!("  Real-time Processing:");
    println!("  → ONNX Runtime (low latency)");
    println!("  → TensorRT (GPU acceleration)");
    println!();
    println!("  Mobile Apps:");
    println!("  → Core ML (iOS)");
    println!("  → TensorFlow Lite (Android)");
    println!();

    println!("🔄 Model Distribution:");
    println!();
    println!("  Open Source Sharing:");
    println!("  → Safetensors (HuggingFace Hub standard)");
    println!("  → GGUF (for quantized variants)");
    println!();
    println!("  Production Deployment:");
    println!("  → TensorRT Engine (compiled, optimized)");
    println!("  → ONNX (cross-framework compatibility)");
    println!();
}

fn demonstrate_metadata() {
    print_separator("Step 6: Model Metadata Management");

    println!("📋 Creating Model Metadata:");
    println!();

    // Example 1: LLM metadata
    let llm_metadata = ModelMetadata::new("llama-2-7b-chat".to_string(), ModelFormat::Safetensors)
        .with_description("Llama 2 7B Chat model fine-tuned for dialogue".to_string())
        .with_framework("PyTorch".to_string())
        .with_task("text-generation".to_string())
        .with_architecture("LlamaForCausalLM".to_string())
        .with_parameters(7_000_000_000)
        .add_custom_field(
            "license".to_string(),
            "Llama 2 Community License".to_string(),
        )
        .add_custom_field("context_length".to_string(), "4096".to_string())
        .add_custom_field("quantization".to_string(), "none".to_string());

    println!("  🤖 LLM Model:");
    println!("     Name: {}", llm_metadata.name);
    println!("     Format: {}", llm_metadata.format);
    println!("     Task: {}", llm_metadata.task.as_ref().unwrap());
    println!(
        "     Architecture: {}",
        llm_metadata.architecture.as_ref().unwrap()
    );
    println!(
        "     Parameters: {:.1}B",
        llm_metadata.parameters.unwrap() as f64 / 1e9
    );
    println!("     Custom Fields:");
    for (key, value) in &llm_metadata.custom_fields {
        println!("       {} = {}", key, value);
    }
    println!();

    // Example 2: Vision model metadata
    let vision_metadata = ModelMetadata::new("yolov8n".to_string(), ModelFormat::ONNX)
        .with_description("YOLOv8 Nano for object detection".to_string())
        .with_framework("Ultralytics".to_string())
        .with_task("object-detection".to_string())
        .with_architecture("YOLOv8".to_string())
        .with_parameters(3_200_000)
        .add_custom_field("input_size".to_string(), "640x640".to_string())
        .add_custom_field("classes".to_string(), "80".to_string())
        .add_custom_field("fps_v100".to_string(), "280".to_string());

    println!("  👁️  Computer Vision Model:");
    println!("     Name: {}", vision_metadata.name);
    println!("     Format: {}", vision_metadata.format);
    println!("     Task: {}", vision_metadata.task.as_ref().unwrap());
    println!(
        "     Parameters: {:.1}M",
        vision_metadata.parameters.unwrap() as f64 / 1e6
    );
    println!("     Custom Fields:");
    for (key, value) in &vision_metadata.custom_fields {
        println!("       {} = {}", key, value);
    }
    println!();

    // Example 3: Quantized model metadata
    let quantized_metadata = ModelMetadata::new("mistral-7b-q4".to_string(), ModelFormat::GGUF)
        .with_description("Mistral 7B quantized to Q4_K_M".to_string())
        .with_framework("llama.cpp".to_string())
        .with_task("text-generation".to_string())
        .with_architecture("Mistral".to_string())
        .with_parameters(7_200_000_000)
        .add_custom_field("quantization".to_string(), "Q4_K_M".to_string())
        .add_custom_field("bits_per_weight".to_string(), "4.5".to_string())
        .add_custom_field("file_size".to_string(), "4.1GB".to_string())
        .add_custom_field("perplexity".to_string(), "5.82".to_string());

    println!("  🔬 Quantized Model:");
    println!("     Name: {}", quantized_metadata.name);
    println!("     Format: {}", quantized_metadata.format);
    println!("     Task: {}", quantized_metadata.task.as_ref().unwrap());
    println!(
        "     Parameters: {:.1}B",
        quantized_metadata.parameters.unwrap() as f64 / 1e9
    );
    println!("     Custom Fields:");
    for (key, value) in &quantized_metadata.custom_fields {
        println!("       {} = {}", key, value);
    }
    println!();
}

fn demonstrate_converter_registry() {
    print_separator("Step 7: Format Converter Registry");

    println!("🔧 Converter System:");
    println!();

    let mut converter = FormatConverter::new();

    // Example conversion registration (these would be real implementations)
    println!("  Registering format converters...");
    println!();

    // Register some example converters
    let conversion_pairs = vec![
        (
            ModelFormat::PyTorch,
            ModelFormat::Safetensors,
            "PyTorch → Safetensors",
        ),
        (
            ModelFormat::Safetensors,
            ModelFormat::GGUF,
            "Safetensors → GGUF (quantization)",
        ),
        (
            ModelFormat::PyTorch,
            ModelFormat::ONNX,
            "PyTorch → ONNX (export)",
        ),
        (
            ModelFormat::ONNX,
            ModelFormat::TensorRT,
            "ONNX → TensorRT (compilation)",
        ),
        (
            ModelFormat::PyTorch,
            ModelFormat::TorchScript,
            "PyTorch → TorchScript (tracing)",
        ),
        (
            ModelFormat::TensorFlow,
            ModelFormat::TFLite,
            "TensorFlow → TFLite (mobile)",
        ),
        (
            ModelFormat::PyTorch,
            ModelFormat::CoreML,
            "PyTorch → Core ML (Apple)",
        ),
        (
            ModelFormat::PyTorch,
            ModelFormat::MLX,
            "PyTorch → MLX (Apple Silicon)",
        ),
        (
            ModelFormat::ONNX,
            ModelFormat::OpenVINO,
            "ONNX → OpenVINO (Intel)",
        ),
        (
            ModelFormat::PyTorch,
            ModelFormat::NCNN,
            "PyTorch → NCNN (mobile)",
        ),
    ];

    // Dummy converter function for demonstration
    fn dummy_converter(_data: &[u8]) -> ironvault::error::Result<Vec<u8>> {
        Ok(vec![])
    }

    for (from, to, description) in &conversion_pairs {
        converter.register(from.clone(), to.clone(), dummy_converter);
        println!("  ✓ Registered: {}", description);
    }
    println!();

    println!("📊 Conversion Support Matrix:");
    println!();

    // Test which conversions are supported
    let test_formats = vec![
        ModelFormat::PyTorch,
        ModelFormat::Safetensors,
        ModelFormat::ONNX,
        ModelFormat::GGUF,
        ModelFormat::TensorRT,
    ];

    println!("  From \\ To │ PyTorch │ Safetensors │ ONNX │ GGUF │ TensorRT");
    println!("  ──────────┼─────────┼─────────────┼──────┼──────┼──────────");

    for from in &test_formats {
        print!("  {:<9} │", from.name());
        for to in &test_formats {
            if from == to {
                print!("    -    │");
            } else if converter.can_convert(from.clone(), to.clone()) {
                print!("    ✓    │");
            } else {
                print!("    ✗    │");
            }
        }
        println!();
    }
    println!();

    println!("🎯 Conversion Workflow:");
    println!();
    println!("  1. Detect source format: ModelFormat::from_extension(ext)");
    println!("  2. Check conversion support: converter.can_convert(from, to)");
    println!("  3. Load model data: read model file");
    println!("  4. Convert: converter.convert(data, from, to)");
    println!("  5. Save converted model: write to new file");
    println!();

    println!("💡 Best Practices:");
    println!();
    println!("  • Use Safetensors as interchange format (safe, fast)");
    println!("  • Keep original weights in version control");
    println!("  • Test converted models for accuracy degradation");
    println!("  • Document quantization settings and perplexity");
    println!("  • Benchmark inference speed for target hardware");
    println!("  • Store metadata with each converted model");
    println!();

    println!("🔍 Format Recommendations by Priority:");
    println!();

    let recommendations = vec![
        (
            "Production LLM Serving",
            vec!["TensorRT", "GGUF Q4_K_M", "Safetensors", "ONNX"],
        ),
        (
            "Model Development",
            vec!["PyTorch", "Safetensors", "ONNX", "TorchScript"],
        ),
        (
            "Mobile Deployment",
            vec!["Core ML", "TFLite", "ONNX Mobile", "NCNN"],
        ),
        ("Edge Devices", vec!["OpenVINO", "TensorRT", "NCNN", "RKNN"]),
        (
            "Cross-Platform",
            vec!["ONNX", "Safetensors", "GGUF", "TFLite"],
        ),
        (
            "Apple Ecosystem",
            vec!["MLX", "Core ML", "GGUF+Metal", "ONNX"],
        ),
    ];

    for (use_case, formats) in recommendations {
        println!("  {}: {}", use_case, formats.join(" > "));
    }
    println!();
}
