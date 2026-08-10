//! Integration tests for the format conversion pipeline.

use ironvault::conversion::*;
use ironvault::formats::ModelFormat;

// ── Pipeline construction ────────────────────────────────────────────────────

#[test]
fn test_pipeline_with_builtins_has_converters() {
    let pipeline = ConversionPipeline::with_builtins();
    let conversions = pipeline.supported_conversions();
    assert!(
        conversions.len() >= 10,
        "Expected at least 10 built-in converters, got {}",
        conversions.len()
    );
}

#[test]
fn test_pipeline_register_custom_converter() {
    struct IdentityConverter;
    impl Converter for IdentityConverter {
        fn name(&self) -> &str {
            "Identity"
        }
        fn source_format(&self) -> ModelFormat {
            ModelFormat::Custom("test-a".into())
        }
        fn target_format(&self) -> ModelFormat {
            ModelFormat::Custom("test-b".into())
        }
        fn convert(
            &self,
            data: &[u8],
            _options: &ConversionOptions,
            _progress: Option<&ProgressCallback>,
        ) -> ironvault::Result<Vec<u8>> {
            Ok(data.to_vec())
        }
    }

    let mut pipeline = ConversionPipeline::new();
    pipeline.register(Box::new(IdentityConverter));
    assert!(pipeline.can_convert_direct(
        &ModelFormat::Custom("test-a".into()),
        &ModelFormat::Custom("test-b".into()),
    ));
    assert!(!pipeline.can_convert_direct(
        &ModelFormat::Custom("test-b".into()),
        &ModelFormat::Custom("test-a".into()),
    ));
}

// ── Path finding ─────────────────────────────────────────────────────────────

#[test]
fn test_find_path_identity() {
    let pipeline = ConversionPipeline::with_builtins();
    let path = pipeline.find_path(&ModelFormat::PyTorch, &ModelFormat::PyTorch);
    assert_eq!(path, Some(vec![ModelFormat::PyTorch]));
}

#[test]
fn test_find_path_direct_safetensors_to_pytorch() {
    let pipeline = ConversionPipeline::with_builtins();
    let path = pipeline.find_path(&ModelFormat::Safetensors, &ModelFormat::PyTorch);
    assert_eq!(
        path,
        Some(vec![ModelFormat::Safetensors, ModelFormat::PyTorch])
    );
}

#[test]
fn test_find_path_multi_step_pytorch_to_tensorrt() {
    let pipeline = ConversionPipeline::with_builtins();
    let path = pipeline
        .find_path(&ModelFormat::PyTorch, &ModelFormat::TensorRT)
        .unwrap();
    // PyTorch → ONNX → TensorRT
    assert_eq!(path.len(), 3);
    assert_eq!(path[0], ModelFormat::PyTorch);
    assert_eq!(path[1], ModelFormat::ONNX);
    assert_eq!(path[2], ModelFormat::TensorRT);
}

#[test]
fn test_find_path_multi_step_pytorch_to_coreml() {
    let pipeline = ConversionPipeline::with_builtins();
    let path = pipeline
        .find_path(&ModelFormat::PyTorch, &ModelFormat::CoreML)
        .unwrap();
    // PyTorch → ONNX → CoreML
    assert_eq!(path.len(), 3);
    assert_eq!(path[0], ModelFormat::PyTorch);
    assert_eq!(path[2], ModelFormat::CoreML);
}

#[test]
fn test_find_path_unreachable_format() {
    let pipeline = ConversionPipeline::with_builtins();
    let path = pipeline.find_path(&ModelFormat::PyTorch, &ModelFormat::MXNet);
    assert!(path.is_none());
}

// ── Same-format passthrough ──────────────────────────────────────────────────

#[test]
fn test_convert_same_format_returns_identical_data() {
    let pipeline = ConversionPipeline::with_builtins();
    let data = b"model data bytes";
    let result = pipeline
        .convert(
            data,
            &ModelFormat::ONNX,
            &ModelFormat::ONNX,
            &ConversionOptions::default(),
            None,
        )
        .unwrap();
    assert_eq!(result.data, data);
    assert_eq!(result.input_size, data.len() as u64);
    assert_eq!(result.output_size, data.len() as u64);
    assert_eq!(result.conversion_path, vec![ModelFormat::ONNX]);
    assert!(result.validation.is_none());
}

// ── SafeTensors ↔ Raw roundtrip ──────────────────────────────────────────────

#[test]
fn test_safetensors_raw_roundtrip_via_pipeline() {
    let pipeline = ConversionPipeline::with_builtins();
    let original = b"raw tensor data for roundtrip testing 0123456789";

    // raw → safetensors
    let st_result = pipeline
        .convert(
            original,
            &ModelFormat::Custom("raw".into()),
            &ModelFormat::Safetensors,
            &ConversionOptions::default(),
            None,
        )
        .unwrap();
    assert!(st_result.data.len() > original.len()); // header overhead

    // safetensors → raw
    let raw_result = pipeline
        .convert(
            &st_result.data,
            &ModelFormat::Safetensors,
            &ModelFormat::Custom("raw".into()),
            &ConversionOptions::default(),
            None,
        )
        .unwrap();
    assert_eq!(raw_result.data, original);
}

// ── SafeTensors validation ───────────────────────────────────────────────────

#[test]
fn test_safetensors_validation_passes() {
    let pipeline = ConversionPipeline::with_builtins();
    let data = b"test data for validation";

    let result = pipeline
        .convert(
            data,
            &ModelFormat::Custom("raw".into()),
            &ModelFormat::Safetensors,
            &ConversionOptions::with_validation(),
            None,
        )
        .unwrap();

    let report = result.validation.unwrap();
    assert!(report.passed);
    assert!(!report.checks.is_empty());
}

// ── GGUF parser ──────────────────────────────────────────────────────────────

#[test]
fn test_gguf_parser_extracts_metadata() {
    let mut data = Vec::new();
    data.extend_from_slice(b"GGUF");
    data.extend_from_slice(&3u32.to_le_bytes()); // version 3
    data.extend_from_slice(&100u64.to_le_bytes()); // tensor count
    data.extend_from_slice(&15u64.to_le_bytes()); // kv count
    data.extend_from_slice(&[0u8; 64]); // padding

    let pipeline = ConversionPipeline::with_builtins();
    let result = pipeline
        .convert(
            &data,
            &ModelFormat::GGUF,
            &ModelFormat::Custom("gguf-meta".into()),
            &ConversionOptions::default(),
            None,
        )
        .unwrap();

    let meta: serde_json::Value = serde_json::from_slice(&result.data).unwrap();
    assert_eq!(meta["format"], "GGUF");
    assert_eq!(meta["version"], 3);
    assert_eq!(meta["tensor_count"], 100);
    assert_eq!(meta["kv_count"], 15);
}

#[test]
fn test_gguf_parser_rejects_invalid() {
    let pipeline = ConversionPipeline::with_builtins();
    let err = pipeline
        .convert(
            b"NOT_A_GGUF_FILE_AT_ALL_XXXX",
            &ModelFormat::GGUF,
            &ModelFormat::Custom("gguf-meta".into()),
            &ConversionOptions::default(),
            None,
        )
        .unwrap_err();
    assert!(format!("{err}").contains("GGUF magic"));
}

// ── Real converter: SafeTensors → PyTorch ────────────────────────────────────

#[test]
fn test_safetensors_to_pytorch_produces_zip() {
    // Build a minimal safetensors file
    let header = r#"{"t":{"dtype":"U8","shape":[4],"data_offsets":[0,4]}}"#;
    let header_bytes = header.as_bytes();
    let mut data = Vec::new();
    data.extend_from_slice(&(header_bytes.len() as u64).to_le_bytes());
    data.extend_from_slice(header_bytes);
    data.extend_from_slice(b"test");

    let pipeline = ConversionPipeline::with_builtins();
    let result = pipeline
        .convert(
            &data,
            &ModelFormat::Safetensors,
            &ModelFormat::PyTorch,
            &ConversionOptions::default(),
            None,
        )
        .unwrap();

    // Should produce a valid ZIP archive (PyTorch .pt format)
    assert!(result.data.len() > 4);
    assert_eq!(&result.data[0..2], b"PK"); // ZIP magic bytes
}

#[test]
fn test_shim_pytorch_to_onnx_custom_opset() {
    let opts = ConversionOptions {
        opset_version: Some(15),
        ..ConversionOptions::default()
    };

    let pipeline = ConversionPipeline::with_builtins();
    let result = pipeline
        .convert(b"", &ModelFormat::PyTorch, &ModelFormat::ONNX, &opts, None)
        .unwrap();

    assert!(
        result.is_plan(),
        "conversion needing external tooling must be reported as a plan"
    );
    assert!(
        result.data.is_empty(),
        "a plan must not masquerade as model data"
    );
    let plan = result.plan.clone().unwrap();
    assert_eq!(plan["converter"], "pytorch_to_onnx");
    assert_eq!(plan["opset_version"], 15);
}

#[test]
fn test_shim_safetensors_to_gguf_quantization() {
    let opts = ConversionOptions {
        quantization: Some("q4_k_m".into()),
        ..ConversionOptions::default()
    };

    let pipeline = ConversionPipeline::with_builtins();
    let result = pipeline
        .convert(
            b"",
            &ModelFormat::Safetensors,
            &ModelFormat::GGUF,
            &opts,
            None,
        )
        .unwrap();

    assert!(
        result.is_plan(),
        "conversion needing external tooling must be reported as a plan"
    );
    assert!(
        result.data.is_empty(),
        "a plan must not masquerade as model data"
    );
    let plan = result.plan.clone().unwrap();
    assert_eq!(plan["converter"], "safetensors_to_gguf");
    assert_eq!(plan["quantization"], "q4_k_m");
}

#[test]
fn test_shim_onnx_to_tensorrt() {
    let pipeline = ConversionPipeline::with_builtins();
    let result = pipeline
        .convert(
            b"",
            &ModelFormat::ONNX,
            &ModelFormat::TensorRT,
            &ConversionOptions::default(),
            None,
        )
        .unwrap();

    assert!(
        result.is_plan(),
        "conversion needing external tooling must be reported as a plan"
    );
    assert!(
        result.data.is_empty(),
        "a plan must not masquerade as model data"
    );
    let plan = result.plan.clone().unwrap();
    assert_eq!(plan["converter"], "onnx_to_tensorrt");
    assert!(plan["requires"]
        .as_array()
        .unwrap()
        .contains(&"tensorrt".into()));
}

#[test]
fn test_shim_onnx_to_coreml() {
    let pipeline = ConversionPipeline::with_builtins();
    let result = pipeline
        .convert(
            b"",
            &ModelFormat::ONNX,
            &ModelFormat::CoreML,
            &ConversionOptions::default(),
            None,
        )
        .unwrap();

    assert!(
        result.is_plan(),
        "conversion needing external tooling must be reported as a plan"
    );
    assert!(
        result.data.is_empty(),
        "a plan must not masquerade as model data"
    );
    let plan = result.plan.clone().unwrap();
    assert_eq!(plan["converter"], "onnx_to_coreml");
}

// ── Multi-step shim conversion ───────────────────────────────────────────────

#[test]
fn test_multi_step_pytorch_to_tensorrt_via_onnx() {
    let pipeline = ConversionPipeline::with_builtins();
    // This traverses PyTorch → ONNX → TensorRT (2 steps)
    let result = pipeline
        .convert(
            b"",
            &ModelFormat::PyTorch,
            &ModelFormat::TensorRT,
            &ConversionOptions::default(),
            None,
        )
        .unwrap();

    // Planning stops at the first step that needs external tooling: there are no
    // real ONNX bytes for the second step to consume, so the returned plan is the
    // PyTorch → ONNX one, not a plan derived from another plan.
    assert!(result.is_plan());
    assert!(result.data.is_empty());
    let plan = result.plan.clone().unwrap();
    assert_eq!(plan["converter"], "pytorch_to_onnx");
    assert_eq!(result.conversion_path.len(), 3);
}

// ── Progress callback ────────────────────────────────────────────────────────

#[test]
fn test_progress_callback_fires() {
    let pipeline = ConversionPipeline::with_builtins();
    let messages = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let msgs = messages.clone();
    let cb: ProgressCallback = Box::new(move |p| {
        msgs.lock().unwrap().push(p.message.clone());
    });

    let _ = pipeline.convert(
        b"test data",
        &ModelFormat::Custom("raw".into()),
        &ModelFormat::Safetensors,
        &ConversionOptions::default(),
        Some(&cb),
    );

    let log = messages.lock().unwrap();
    assert!(!log.is_empty(), "Progress callback should have been called");
}

// ── Validation structures ────────────────────────────────────────────────────

#[test]
fn test_validation_check_pass_and_fail() {
    let pass = ValidationCheck::pass("test", "looks good");
    assert!(pass.passed);
    assert_eq!(pass.name, "test");

    let fail = ValidationCheck::fail("test", "something wrong");
    assert!(!fail.passed);
}

#[test]
fn test_validation_report_mixed() {
    let checks = vec![
        ValidationCheck::pass("a", "ok"),
        ValidationCheck::pass("b", "ok"),
        ValidationCheck::fail("c", "bad"),
    ];
    let report = ValidationReport::from_checks(checks);
    assert!(!report.passed); // one failure means overall failure
    assert_eq!(report.checks.len(), 3);
}

#[test]
fn test_validation_report_all_pass() {
    let checks = vec![
        ValidationCheck::pass("a", "ok"),
        ValidationCheck::pass("b", "ok"),
    ];
    let report = ValidationReport::from_checks(checks);
    assert!(report.passed);
}

// ── ConversionOptions ────────────────────────────────────────────────────────

#[test]
fn test_conversion_options_default() {
    let opts = ConversionOptions::default();
    assert!(!opts.validate);
    assert!(!opts.preserve_metadata);
    assert!(opts.quantization.is_none());
    assert!(opts.opset_version.is_none());
    assert!(opts.extra.is_empty());
}

#[test]
fn test_conversion_options_with_validation() {
    let opts = ConversionOptions::with_validation();
    assert!(opts.validate);
    assert!(opts.preserve_metadata);
    assert!((opts.tolerance - 1e-5).abs() < f64::EPSILON);
}

// ── ConversionResult ─────────────────────────────────────────────────────────

#[test]
fn test_conversion_result_compression_ratio() {
    let result = ConversionResult {
        data: vec![],
        plan: None,
        source_format: ModelFormat::PyTorch,
        target_format: ModelFormat::Safetensors,
        conversion_path: vec![],
        input_size: 2000,
        output_size: 1000,
        validation: None,
    };
    assert!((result.compression_ratio() - 0.5).abs() < f64::EPSILON);
}

#[test]
fn test_conversion_result_zero_input() {
    let result = ConversionResult {
        data: vec![],
        plan: None,
        source_format: ModelFormat::PyTorch,
        target_format: ModelFormat::Safetensors,
        conversion_path: vec![],
        input_size: 0,
        output_size: 100,
        validation: None,
    };
    assert!((result.compression_ratio() - 0.0).abs() < f64::EPSILON);
}

// ── ConversionProgress ───────────────────────────────────────────────────────

#[test]
fn test_progress_display_with_percentage() {
    let p = ConversionProgress {
        step: 0,
        total_steps: 2,
        bytes_processed: 750,
        bytes_total: 1000,
        message: "Processing".into(),
    };
    let s = format!("{p}");
    assert!(s.contains("75.0%"));
    assert!(s.contains("[1/2]"));
    assert!(s.contains("Processing"));
}

#[test]
fn test_progress_display_without_total() {
    let p = ConversionProgress {
        step: 1,
        total_steps: 3,
        bytes_processed: 500,
        bytes_total: 0,
        message: "Working".into(),
    };
    let s = format!("{p}");
    assert!(s.contains("[2/3]"));
    assert!(s.contains("Working"));
    assert!(!s.contains('%'));
}

// ── Error paths ──────────────────────────────────────────────────────────────

#[test]
fn test_no_conversion_path_error() {
    let pipeline = ConversionPipeline::new(); // empty
    let err = pipeline
        .convert(
            b"data",
            &ModelFormat::PyTorch,
            &ModelFormat::CoreML,
            &ConversionOptions::default(),
            None,
        )
        .unwrap_err();
    assert!(format!("{err}").contains("No conversion path"));
}

#[test]
fn test_safetensors_too_small_error() {
    let pipeline = ConversionPipeline::with_builtins();
    let err = pipeline
        .convert(
            b"sml",
            &ModelFormat::Safetensors,
            &ModelFormat::Custom("raw".into()),
            &ConversionOptions::default(),
            None,
        )
        .unwrap_err();
    assert!(format!("{err}").contains("too small"));
}

#[test]
fn test_safetensors_header_overflow_error() {
    // header_len says 9999 but data is only 20 bytes
    let mut data = Vec::new();
    data.extend_from_slice(&9999u64.to_le_bytes());
    data.extend_from_slice(&[0u8; 12]);

    let pipeline = ConversionPipeline::with_builtins();
    let err = pipeline
        .convert(
            &data,
            &ModelFormat::Safetensors,
            &ModelFormat::Custom("raw".into()),
            &ConversionOptions::default(),
            None,
        )
        .unwrap_err();
    assert!(format!("{err}").contains("exceeds"));
}
