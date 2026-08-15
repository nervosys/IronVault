//! Convert command handler — model format conversion via ConversionPipeline.

use ironvault::conversion::{ConversionOptions, ConversionPipeline};
use ironvault::formats::ModelFormat;
use ironvault::{Result, VaultConfig, VaultError};
use std::path::{Path, PathBuf};

use crate::cli::helpers::{build_vault, prompt_passphrase};

/// List all supported conversions.
pub fn handle_list_conversions() -> Result<()> {
    let pipeline = ConversionPipeline::with_builtins();
    let conversions = pipeline.supported_conversions();

    println!(
        "🔄 Supported format conversions ({} total):\n",
        conversions.len()
    );
    println!("   {:<30} Converter", "Conversion");
    println!("   {:<30} ─────────", "──────────");
    for (src, dst, name) in &conversions {
        println!(
            "   {:<30} {}",
            format!("{} → {}", src.name(), dst.name()),
            name
        );
    }

    // Show some multi-step paths
    println!("\n📊 Multi-step conversion paths (via BFS):");
    let interesting = [
        (ModelFormat::PyTorch, ModelFormat::TensorRT),
        (ModelFormat::PyTorch, ModelFormat::CoreML),
        (ModelFormat::Safetensors, ModelFormat::TensorRT),
    ];
    for (from, to) in &interesting {
        if let Some(path) = pipeline.find_path(from, to) {
            let names: Vec<&str> = path.iter().map(|f| f.name()).collect();
            println!("   {} → {}: {}", from.name(), to.name(), names.join(" → "));
        }
    }

    Ok(())
}

/// Convert a model between formats.
#[allow(clippy::too_many_arguments)]
pub fn handle_convert(
    name: String,
    to_format_str: String,
    output: Option<PathBuf>,
    version: Option<u32>,
    quantization: Option<String>,
    opset: Option<u32>,
    validate: bool,
    plan_only: bool,
    from_dir: Option<PathBuf>,
    config: VaultConfig,
    use_sqlite: bool,
) -> Result<()> {
    println!("🔄 Converting model format");
    println!("   Model: {}", name);
    println!("   Target format: {}", to_format_str);

    // Parse target format
    let to_format = parse_format(&to_format_str)?;

    // A HuggingFace checkpoint is a *directory* — weights plus config.json plus
    // tokenizer.model — and the real converter streams it tensor by tensor. The
    // `Converter` trait takes `&[u8]` and returns `Vec<u8>`, so neither the
    // registry nor the vault path can carry it. This is its own route, and it
    // never opens the vault.
    if let Some(src_dir) = from_dir {
        return convert_from_directory(
            &name,
            &src_dir,
            to_format,
            &to_format_str,
            output,
            quantization.as_deref(),
            plan_only,
        );
    }

    // Open vault and get model
    let mut vault = build_vault(config.clone(), use_sqlite)?;
    let passphrase = prompt_passphrase("Enter vault passphrase: ")?;
    vault.unlock(passphrase)?;

    // Get version to convert
    let version_num = if let Some(v) = version {
        v
    } else {
        vault
            .list_versions(&name)
            .last()
            .map(|mv| mv.version)
            .ok_or_else(|| {
                VaultError::ModelNotFound(format!("Model '{}' not found or has no versions", name))
            })?
    };

    // Get model data and metadata
    let data = vault.get_model(&name, Some(version_num))?;
    let versions = vault.list_versions(&name);
    let model_version = versions
        .iter()
        .find(|v| v.version == version_num)
        .ok_or_else(|| VaultError::VersionNotFound(version_num, name.clone()))?;

    // Version records store `format.name()`, not an extension.
    let from_format = ModelFormat::from_stored(&model_version.format);
    println!("   Source format: {}", from_format.name());
    println!("   Source size: {} bytes", data.len());

    if from_format == to_format {
        println!(
            "\n⚠️  Model is already in {} format — no conversion needed.",
            to_format.name()
        );
        return Ok(());
    }

    // Build conversion options
    let mut options = if validate {
        ConversionOptions::with_validation()
    } else {
        ConversionOptions::default()
    };
    options.quantization = quantization.clone();
    options.opset_version = opset;
    options.preserve_metadata = true;

    // Create pipeline and find path
    let pipeline = ConversionPipeline::with_builtins();

    let path = pipeline.find_path(&from_format, &to_format).ok_or_else(|| {
        VaultError::ConversionError(format!(
            "No conversion path from {} to {}. Run `iv list-conversions` to see available conversions.",
            from_format.name(),
            to_format.name(),
        ))
    })?;

    let path_names: Vec<&str> = path.iter().map(|f| f.name()).collect();
    println!("   Conversion path: {}", path_names.join(" → "));

    if path.len() > 2 {
        println!("   ({} steps)", path.len() - 1);
    }

    // Determine output path
    let output_path = output.unwrap_or_else(|| {
        let extension = to_format.extension();
        PathBuf::from(format!("{}_converted.{}", name, extension))
    });

    if plan_only {
        println!("\n📋 Conversion plan (--plan-only):");
        for window in path.windows(2) {
            let (src, dst) = (&window[0], &window[1]);
            println!("   • {} → {}", src.name(), dst.name());
            if pipeline.can_convert_direct(src, dst) {
                println!("     ✅ Direct converter available");
            }
        }
        if let Some(q) = &quantization {
            println!("   Quantization: {}", q);
        }
        if let Some(o) = opset {
            println!("   ONNX opset: {}", o);
        }
        println!("   Output: {}", output_path.display());
        println!("\n   Remove --plan-only to execute the conversion.");
        return Ok(());
    }

    // Execute conversion with progress
    println!("\n⚙️  Converting...");
    let progress_cb: ironvault::conversion::ProgressCallback = Box::new(|p| {
        println!("   {}", p);
    });

    // Both labels come from `telemetry_name`, which collapses
    // `ModelFormat::Custom` to a literal -- `name()` would return whatever
    // string the user passed to `--to-format`.
    let started = std::time::Instant::now();
    let converted = pipeline.convert(
        &data,
        &from_format,
        &to_format,
        &options,
        Some(&progress_cb),
    );
    ironvault::telemetry::track_conversion(
        from_format.telemetry_name(),
        to_format.telemetry_name(),
        started.elapsed(),
        converted.is_ok(),
    );
    let result = converted?;

    if let Some(plan) = &result.plan {
        // No conversion happened — the pipeline returned instructions instead.
        println!("\n📋 This conversion requires external Python tools.");
        println!("   Converter: {}", plan["converter"]);

        if let Some(requires) = plan["requires"].as_array() {
            let pkgs: Vec<&str> = requires.iter().filter_map(|v| v.as_str()).collect();
            println!("   Required packages: {}", pkgs.join(", "));
            println!("   Install: pip install {}", pkgs.join(" "));
        }

        // Export the source model for manual conversion
        let export_ext = from_format.extension();
        let export_file = format!("{}_v{}.{}", name, version_num, export_ext);
        std::fs::write(&export_file, &data)?;
        println!(
            "\n   Source exported: {} ({} bytes)",
            export_file,
            data.len()
        );

        if let Some(python) = plan["python"].as_str() {
            println!("\n   Python script (set input_path/output_path):");
            for line in python.lines() {
                println!("     {}", line);
            }
        }
        if let Some(shell) = plan["shell"].as_str() {
            println!("\n   Or shell command:");
            println!("     {}", shell);
        }

        // Write the plan next to the requested output, never *as* it — a file
        // named `model.onnx` must never contain a JSON plan.
        let plan_path = output_path.with_extension("plan.json");
        std::fs::write(&plan_path, serde_json::to_vec_pretty(plan)?)?;
        println!("\n   Plan written: {}", plan_path.display());
        println!(
            "   No {} file was produced — run the steps above to create one.",
            to_format.name()
        );

        println!("\n   After conversion, store back:");
        println!(
            "      iv store {} {} --format {}",
            name,
            output_path.display(),
            to_format_str
        );
    } else {
        // Pure-Rust conversion — write the output directly
        std::fs::write(&output_path, &result.data)?;
        println!("\n✅ Conversion complete!");
        println!(
            "   Output: {} ({} bytes)",
            output_path.display(),
            result.output_size
        );
        println!("   Size ratio: {:.2}×", result.compression_ratio());

        if let Some(ref report) = result.validation {
            if report.passed {
                println!("   Validation: ✅ all checks passed");
            } else {
                println!("   Validation: ❌ some checks failed:");
                for check in &report.checks {
                    let icon = if check.passed { "✅" } else { "❌" };
                    println!("     {} {}: {}", icon, check.name, check.message);
                }
            }
        }

        println!(
            "\n   Store back: iv store {} {} --format {}",
            name,
            output_path.display(),
            to_format_str
        );
    }

    Ok(())
}

/// safetensors directory → GGUF, via the real streaming converter.
fn convert_from_directory(
    name: &str,
    src_dir: &Path,
    to_format: ModelFormat,
    to_format_str: &str,
    output: Option<PathBuf>,
    quantization: Option<&str>,
    plan_only: bool,
) -> Result<()> {
    if to_format != ModelFormat::GGUF {
        return Err(VaultError::InvalidInput(format!(
            "--from-dir converts a HuggingFace checkpoint to GGUF; got --to-format {}. \
             Drop --from-dir to convert a vaulted model to {}.",
            to_format_str,
            to_format.name(),
        )));
    }
    if !src_dir.is_dir() {
        return Err(VaultError::InvalidInput(format!(
            "--from-dir {} is not a directory. It should be a HuggingFace checkpoint \
             directory containing config.json, tokenizer.model and the safetensors \
             shards.",
            src_dir.display(),
        )));
    }

    let out_type = parse_out_type(quantization)?;
    let output_path = output.unwrap_or_else(|| PathBuf::from(format!("{name}.gguf")));

    println!("   Source: {} (HuggingFace directory)", src_dir.display());
    println!("   Output tensor type: {out_type:?}");
    println!("   Output: {}", output_path.display());

    if plan_only {
        println!("\n📋 --plan-only: nothing was written.");
        println!("   Remove --plan-only to execute the conversion.");
        return Ok(());
    }

    println!("\n⚙️  Converting (streaming; peak memory is the largest tensor)...");
    let started = std::time::Instant::now();
    let result = ironvault::hf_gguf::convert_hf_to_gguf(
        src_dir,
        &output_path,
        &ironvault::hf_gguf::HfToGgufOptions { out_type },
    );
    ironvault::telemetry::track_conversion(
        ModelFormat::Safetensors.telemetry_name(),
        to_format.telemetry_name(),
        started.elapsed(),
        result.is_ok(),
    );
    let summary = result?;

    println!("\n✅ Conversion complete!");
    println!("   Tensors: {}", summary.tensors);
    println!(
        "   Tensor bytes: {} ({:.2} GiB)",
        summary.tensor_bytes,
        summary.tensor_bytes as f64 / (1024.0 * 1024.0 * 1024.0),
    );
    println!("   Vocabulary: {} entries", summary.vocab);
    println!("   Elapsed: {:.1}s", started.elapsed().as_secs_f64());

    // 🔑 A GGUF that loads is not a GGUF that is right: a shuffled RoPE
    // permutation, a wrong tokenizer model, or a missing merge table each
    // produce a file that parses and generates fluent, wrong text.
    println!("\n   Verify before trusting it: run a prompt and compare the token");
    println!("   ids against the source tokenizer. Structural checks cannot");
    println!("   substitute — every one of those failures still parses.");
    println!(
        "\n   Store it: iv store {} {} --format {}",
        name,
        output_path.display(),
        to_format_str,
    );
    Ok(())
}

/// Map `--quantization` onto an output tensor type.
///
/// Only the widths this project can actually *write* are accepted. A K-quant is
/// refused by name rather than silently downgraded to F16 — a file that is not
/// the type you asked for is worse than an error, because it works.
fn parse_out_type(quantization: Option<&str>) -> Result<gguf_quant::GGMLQuantizationType> {
    use gguf_quant::GGMLQuantizationType as Q;
    match quantization.unwrap_or("f16").to_lowercase().as_str() {
        "f16" | "fp16" | "half" => Ok(Q::F16),
        "bf16" | "bfloat16" => Ok(Q::BF16),
        "f32" | "fp32" | "float32" => Ok(Q::F32),
        other => Err(VaultError::InvalidInput(format!(
            "cannot write {other}: this converter emits f16, bf16 or f32 only. \
             No K-quant encoder exists in this project — writing one means porting \
             llama.cpp's scale search, and a subtly wrong search yields a model that \
             generates fluent but degraded text. Convert to f16 here, then run \
             `llama-quantize out.gguf out-{other}.gguf {other}`."
        ))),
    }
}

fn parse_format(s: &str) -> Result<ModelFormat> {
    match s.to_lowercase().as_str() {
        "safetensors" => Ok(ModelFormat::Safetensors),
        "gguf" => Ok(ModelFormat::GGUF),
        "pytorch" | "pt" | "torch" => Ok(ModelFormat::PyTorch),
        "onnx" => Ok(ModelFormat::ONNX),
        "tensorrt" | "trt" => Ok(ModelFormat::TensorRT),
        "tflite" | "tensorflow-lite" => Ok(ModelFormat::TFLite),
        "coreml" | "mlmodel" => Ok(ModelFormat::CoreML),
        "mlx" => Ok(ModelFormat::MLX),
        "torchscript" => Ok(ModelFormat::TorchScript),
        "openvino" => Ok(ModelFormat::OpenVINO),
        "ncnn" => Ok(ModelFormat::NCNN),
        "mnn" => Ok(ModelFormat::MNN),
        _ => Err(VaultError::InvalidInput(format!(
            "Unsupported format: '{}'. Use: safetensors, gguf, pytorch, onnx, tensorrt, tflite, coreml, mlx, torchscript, openvino, ncnn, mnn",
            s
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gguf_quant::GGMLQuantizationType as Q;

    #[test]
    fn out_type_defaults_to_f16() {
        assert_eq!(parse_out_type(None).unwrap(), Q::F16);
    }

    #[test]
    fn out_type_accepts_the_widths_we_can_write() {
        for (s, want) in [
            ("f16", Q::F16),
            ("FP16", Q::F16),
            ("bf16", Q::BF16),
            ("f32", Q::F32),
        ] {
            assert_eq!(parse_out_type(Some(s)).unwrap(), want, "{s}");
        }
    }

    #[test]
    fn a_k_quant_is_refused_by_name_not_downgraded() {
        // Silently emitting F16 for `-q q4_k_m` would produce a 4× larger file
        // that works — the failure mode this project keeps paying for.
        let err = parse_out_type(Some("q4_k_m")).unwrap_err().to_string();
        assert!(err.contains("q4_k_m"), "must name what it got: {err}");
        assert!(err.contains("llama-quantize"), "must say what to do: {err}");
    }
}
