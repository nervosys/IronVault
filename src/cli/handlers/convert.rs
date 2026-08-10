//! Convert command handler — model format conversion via ConversionPipeline.

use ironvault::conversion::{ConversionOptions, ConversionPipeline};
use ironvault::formats::ModelFormat;
use ironvault::{Result, VaultConfig, VaultError};
use std::path::PathBuf;

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
    config: VaultConfig,
    use_sqlite: bool,
) -> Result<()> {
    println!("🔄 Converting model format");
    println!("   Model: {}", name);
    println!("   Target format: {}", to_format_str);

    // Parse target format
    let to_format = parse_format(&to_format_str)?;

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
