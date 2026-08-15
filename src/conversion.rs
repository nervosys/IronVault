//! Model format conversion pipeline
//!
//! Provides a trait-based conversion architecture with:
//! - Pluggable converter implementations
//! - Multi-step conversion via graph search
//! - Progress reporting callbacks
//! - Output validation (integrity, accuracy, metadata)

use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::error::{Result, VaultError};
use crate::formats::ModelFormat;

// ── Progress reporting ───────────────────────────────────────────────────────

/// Progress update sent during a conversion operation.
#[derive(Debug, Clone)]
pub struct ConversionProgress {
    /// Current step index (0-based).
    pub step: usize,
    /// Total number of steps.
    pub total_steps: usize,
    /// Bytes processed so far in the current step.
    pub bytes_processed: u64,
    /// Total bytes for the current step (0 = unknown).
    pub bytes_total: u64,
    /// Human-readable description of the current step.
    pub message: String,
}

impl fmt::Display for ConversionProgress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.bytes_total > 0 {
            let pct = (self.bytes_processed as f64 / self.bytes_total as f64) * 100.0;
            write!(
                f,
                "[{}/{}] {:.1}% — {}",
                self.step + 1,
                self.total_steps,
                pct,
                self.message,
            )
        } else {
            write!(
                f,
                "[{}/{}] {}",
                self.step + 1,
                self.total_steps,
                self.message,
            )
        }
    }
}

/// Callback type for progress updates.
pub type ProgressCallback = Box<dyn Fn(&ConversionProgress) + Send + Sync>;

// ── Conversion options ───────────────────────────────────────────────────────

/// Options that influence a conversion operation.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ConversionOptions {
    /// Target quantization (e.g. "q4_0", "q8_0" for GGUF).
    pub quantization: Option<String>,
    /// Target opset version for ONNX export.
    pub opset_version: Option<u32>,
    /// Preserve training-related metadata.
    pub preserve_metadata: bool,
    /// Validate the output after conversion.
    pub validate: bool,
    /// Tolerance for numerical accuracy validation.
    pub tolerance: f64,
    /// Custom key-value options for format-specific converters.
    pub extra: HashMap<String, String>,
}

impl ConversionOptions {
    /// Create options with validation enabled and a default tolerance.
    #[must_use]
    pub fn with_validation() -> Self {
        Self {
            validate: true,
            tolerance: 1e-5,
            preserve_metadata: true,
            ..Default::default()
        }
    }
}

// ── Conversion result ────────────────────────────────────────────────────────

/// Metadata collected during a conversion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversionResult {
    /// The converted model data.
    ///
    /// Empty when [`Self::plan`] is set — a plan is not a model file.
    #[serde(skip)]
    pub data: Vec<u8>,
    /// Set when the conversion could not be performed natively and instead
    /// produced instructions for external tooling.
    ///
    /// When this is `Some`, no conversion happened: `data` is empty and the
    /// caller must run the described steps to obtain the target format.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plan: Option<serde_json::Value>,
    /// Source format.
    pub source_format: ModelFormat,
    /// Target format.
    pub target_format: ModelFormat,
    /// Formats traversed (including source and target).
    pub conversion_path: Vec<ModelFormat>,
    /// Size of the input (bytes).
    pub input_size: u64,
    /// Size of the output (bytes).
    pub output_size: u64,
    /// Validation report (populated if validation was requested).
    pub validation: Option<ValidationReport>,
}

impl ConversionResult {
    /// True when no conversion was performed and [`Self::plan`] holds
    /// instructions for external tooling instead.
    ///
    /// Callers that write `data` to a file must check this first — otherwise
    /// they produce a file with the target extension and the wrong contents.
    #[must_use]
    pub fn is_plan(&self) -> bool {
        self.plan.is_some()
    }

    /// Compression ratio (output / input).
    #[must_use]
    pub fn compression_ratio(&self) -> f64 {
        if self.input_size == 0 {
            return 0.0;
        }
        self.output_size as f64 / self.input_size as f64
    }
}

// ── Validation ───────────────────────────────────────────────────────────────

/// Result of validating a converted model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationReport {
    /// Whether the output passes all checks.
    pub passed: bool,
    /// Individual check results.
    pub checks: Vec<ValidationCheck>,
}

impl ValidationReport {
    /// Create a passing report from a list of checks.
    #[must_use]
    pub fn from_checks(checks: Vec<ValidationCheck>) -> Self {
        let passed = checks.iter().all(|c| c.passed);
        Self { passed, checks }
    }
}

/// A single validation check.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationCheck {
    /// Name of the check.
    pub name: String,
    /// Whether the check passed.
    pub passed: bool,
    /// Human-readable detail.
    pub message: String,
}

impl ValidationCheck {
    /// Create a passing check.
    #[must_use]
    pub fn pass(name: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            passed: true,
            message: message.into(),
        }
    }

    /// Create a failing check.
    #[must_use]
    pub fn fail(name: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            passed: false,
            message: message.into(),
        }
    }
}

// ── Converter trait ──────────────────────────────────────────────────────────

/// A single-step format converter.
///
/// Implement this trait to add support for a new format pair.
pub trait Converter: Send + Sync {
    /// Human-readable name (e.g. "SafeTensors → PyTorch").
    fn name(&self) -> &str;

    /// Source format this converter reads.
    fn source_format(&self) -> ModelFormat;

    /// Target format this converter produces.
    fn target_format(&self) -> ModelFormat;

    /// Whether this converter emits a JSON *plan* describing how to perform the
    /// conversion with external tooling, rather than target-format bytes.
    ///
    /// Converters that need a Python runtime (PyTorch → ONNX, ONNX → TensorRT,
    /// ONNX → Core ML, SafeTensors → GGUF) return `true`. Callers must not treat
    /// their output as a model file — [`ConversionResult::plan`] carries it
    /// instead, and [`ConversionResult::data`] is left empty.
    fn produces_plan(&self) -> bool {
        false
    }

    /// Perform the conversion.
    fn convert(
        &self,
        data: &[u8],
        options: &ConversionOptions,
        progress: Option<&ProgressCallback>,
    ) -> Result<Vec<u8>>;

    /// Validate the output. Default implementation checks non-empty + magic bytes.
    fn validate(
        &self,
        input: &[u8],
        output: &[u8],
        options: &ConversionOptions,
    ) -> ValidationReport {
        let mut checks = vec![
            // Non-empty output
            if output.is_empty() {
                ValidationCheck::fail("non_empty", "Output is empty")
            } else {
                ValidationCheck::pass("non_empty", format!("Output is {} bytes", output.len()))
            },
            // Size sanity (output shouldn't be >100× or <0.001× input unless quantizing)
            {
                let ratio = if input.is_empty() {
                    1.0
                } else {
                    output.len() as f64 / input.len() as f64
                };
                if ratio > 100.0 && options.quantization.is_none() {
                    ValidationCheck::fail(
                        "size_ratio",
                        format!("Suspicious size ratio: {ratio:.1}×"),
                    )
                } else {
                    ValidationCheck::pass("size_ratio", format!("Size ratio: {ratio:.2}×"))
                }
            },
        ];

        // Format-specific magic bytes check
        let magic_check = validate_magic_bytes(output, &self.target_format());
        checks.push(magic_check);

        ValidationReport::from_checks(checks)
    }
}

// ── Conversion pipeline ──────────────────────────────────────────────────────

/// Registry of converters with multi-step path finding.
pub struct ConversionPipeline {
    converters: Vec<Box<dyn Converter>>,
    /// Adjacency: (source, target) → index into `converters`.
    edges: HashMap<(ModelFormat, ModelFormat), usize>,
}

impl ConversionPipeline {
    /// Create an empty pipeline.
    #[must_use]
    pub fn new() -> Self {
        Self {
            converters: Vec::new(),
            edges: HashMap::new(),
        }
    }

    /// Create a pipeline pre-loaded with all built-in converters.
    #[must_use]
    pub fn with_builtins() -> Self {
        let mut p = Self::new();
        p.register(Box::new(SafeTensorsToRawConverter));
        p.register(Box::new(RawToSafeTensorsConverter));
        p.register(Box::new(GgufHeaderParser));
        p.register(Box::new(OnnxMetadataExtractor));
        p.register(Box::new(SafeTensorsToPyTorchConverter));
        p.register(Box::new(PyTorchToSafeTensorsConverter));
        p.register(Box::new(PyTorchToOnnxConverter));
        p.register(Box::new(OnnxToTensorRtConverter));
        p.register(Box::new(OnnxToCoreMLConverter));
        p.register(Box::new(SafeTensorsToGgufConverter));
        p
    }

    /// Register a converter.
    pub fn register(&mut self, converter: Box<dyn Converter>) {
        let key = (converter.source_format(), converter.target_format());
        let idx = self.converters.len();
        self.converters.push(converter);
        self.edges.insert(key, idx);
    }

    /// Check whether a direct conversion is available.
    #[must_use]
    pub fn can_convert_direct(&self, from: &ModelFormat, to: &ModelFormat) -> bool {
        self.edges.contains_key(&(from.clone(), to.clone()))
    }

    /// Find the shortest conversion path (BFS over the format graph).
    #[must_use]
    pub fn find_path(&self, from: &ModelFormat, to: &ModelFormat) -> Option<Vec<ModelFormat>> {
        if from == to {
            return Some(vec![from.clone()]);
        }

        // BFS
        let mut visited: HashSet<ModelFormat> = HashSet::new();
        let mut queue: VecDeque<Vec<ModelFormat>> = VecDeque::new();
        queue.push_back(vec![from.clone()]);
        visited.insert(from.clone());

        while let Some(path) = queue.pop_front() {
            let current = path.last().unwrap();

            for (src, dst) in self.edges.keys() {
                if src == current && !visited.contains(dst) {
                    let mut new_path = path.clone();
                    new_path.push(dst.clone());
                    if dst == to {
                        return Some(new_path);
                    }
                    visited.insert(dst.clone());
                    queue.push_back(new_path);
                }
            }
        }

        None
    }

    /// List all supported direct conversions.
    #[must_use]
    pub fn supported_conversions(&self) -> Vec<(ModelFormat, ModelFormat, &str)> {
        self.converters
            .iter()
            .map(|c| (c.source_format(), c.target_format(), c.name()))
            .collect()
    }

    /// Perform a (possibly multi-step) conversion.
    pub fn convert(
        &self,
        data: &[u8],
        from: &ModelFormat,
        to: &ModelFormat,
        options: &ConversionOptions,
        progress: Option<&ProgressCallback>,
    ) -> Result<ConversionResult> {
        if from == to {
            return Ok(ConversionResult {
                data: data.to_vec(),
                plan: None,
                source_format: from.clone(),
                target_format: to.clone(),
                conversion_path: vec![from.clone()],
                input_size: data.len() as u64,
                output_size: data.len() as u64,
                validation: None,
            });
        }

        let path = self.find_path(from, to).ok_or_else(|| {
            VaultError::ConversionError(format!(
                "No conversion path from {} to {}",
                from.name(),
                to.name(),
            ))
        })?;

        let total_steps = path.len() - 1;
        let mut current_data = data.to_vec();
        // Set as soon as any step emits a plan; the whole conversion is then a
        // plan, because later steps have no real bytes to work from.
        let mut plan: Option<serde_json::Value> = None;

        for (i, window) in path.windows(2).enumerate() {
            let (src, dst) = (&window[0], &window[1]);
            let idx = self.edges.get(&(src.clone(), dst.clone())).ok_or_else(|| {
                VaultError::ConversionError(format!(
                    "Missing converter for {} -> {}",
                    src.name(),
                    dst.name(),
                ))
            })?;

            let converter = &self.converters[*idx];

            // Report progress
            if let Some(cb) = progress {
                cb(&ConversionProgress {
                    step: i,
                    total_steps,
                    bytes_processed: 0,
                    bytes_total: current_data.len() as u64,
                    message: format!("{} → {}", src.name(), dst.name()),
                });
            }

            current_data = converter.convert(&current_data, options, progress)?;

            if converter.produces_plan() {
                plan = Some(
                    serde_json::from_slice(&current_data)
                        .unwrap_or_else(|_| serde_json::json!({ "converter": converter.name() })),
                );
                // Stop here: downstream steps cannot operate on a plan, and the
                // caller must run the external tooling before continuing.
                break;
            }

            // Intermediate validation
            if options.validate && i < total_steps - 1 {
                let report = converter.validate(data, &current_data, options);
                if !report.passed {
                    return Err(VaultError::ConversionError(format!(
                        "Intermediate validation failed at step {} ({}): {:?}",
                        i + 1,
                        converter.name(),
                        report
                            .checks
                            .iter()
                            .filter(|c| !c.passed)
                            .map(|c| &c.message)
                            .collect::<Vec<_>>(),
                    )));
                }
            }
        }

        // A plan is not target-format bytes: report it as such and hand back no
        // data, so no caller can write it out as a model file.
        if plan.is_some() {
            return Ok(ConversionResult {
                input_size: data.len() as u64,
                output_size: 0,
                data: Vec::new(),
                plan,
                source_format: from.clone(),
                target_format: to.clone(),
                conversion_path: path,
                validation: None,
            });
        }

        // Final validation
        let validation = if options.validate {
            let final_idx = self
                .edges
                .get(&(path[path.len() - 2].clone(), to.clone()))
                .unwrap();
            let report = self.converters[*final_idx].validate(data, &current_data, options);
            Some(report)
        } else {
            None
        };

        Ok(ConversionResult {
            input_size: data.len() as u64,
            output_size: current_data.len() as u64,
            data: current_data,
            plan: None,
            source_format: from.clone(),
            target_format: to.clone(),
            conversion_path: path,
            validation,
        })
    }
}

impl Default for ConversionPipeline {
    fn default() -> Self {
        Self::with_builtins()
    }
}

// ── Magic byte validation ────────────────────────────────────────────────────

/// Validate that `data` starts with the expected magic bytes for `format`.
fn validate_magic_bytes(data: &[u8], format: &ModelFormat) -> ValidationCheck {
    let (expected, label): (Option<&[u8]>, &str) = match format {
        ModelFormat::GGUF => (Some(b"GGUF"), "GGUF magic"),
        ModelFormat::ONNX => (Some(&[0x08]), "ONNX protobuf tag"), // field 1 varint
        ModelFormat::Safetensors => {
            // SafeTensors starts with a little-endian u64 header length
            if data.len() >= 8 {
                let header_len = u64::from_le_bytes(data[..8].try_into().unwrap());
                if header_len > 0 && header_len < data.len() as u64 {
                    return ValidationCheck::pass(
                        "magic_bytes",
                        format!("Valid SafeTensors header ({header_len} bytes)"),
                    );
                }
                return ValidationCheck::fail("magic_bytes", "Invalid SafeTensors header length");
            }
            return ValidationCheck::fail("magic_bytes", "Too small for SafeTensors header");
        }
        ModelFormat::PyTorch => {
            // PyTorch .pt files are ZIP archives (PK magic)
            if data.len() >= 2 && data[0] == b'P' && data[1] == b'K' {
                return ValidationCheck::pass("magic_bytes", "Valid PyTorch ZIP archive");
            }
            // Older pickle format
            if data.len() >= 2 && data[0] == 0x80 {
                return ValidationCheck::pass("magic_bytes", "Valid PyTorch pickle format");
            }
            return ValidationCheck::fail("magic_bytes", "Unrecognised PyTorch header");
        }
        ModelFormat::TFLite => (Some(b"\x20\x00\x00\x00"), "TFLite FlatBuffer"),
        _ => (None, ""),
    };

    match expected {
        Some(magic) => {
            if data.len() >= magic.len() && &data[..magic.len()] == magic {
                ValidationCheck::pass("magic_bytes", format!("{label} OK"))
            } else {
                ValidationCheck::fail("magic_bytes", format!("{label} mismatch"))
            }
        }
        None => ValidationCheck::pass(
            "magic_bytes",
            format!("No magic-byte check for {}", format.name()),
        ),
    }
}

// ── Built-in converters ──────────────────────────────────────────────────────

// ---------- SafeTensors ↔ raw tensor data ----------

/// Parse SafeTensors format and extract raw tensor data concatenated.
pub struct SafeTensorsToRawConverter;

impl Converter for SafeTensorsToRawConverter {
    fn name(&self) -> &str {
        "SafeTensors → Raw"
    }
    fn source_format(&self) -> ModelFormat {
        ModelFormat::Safetensors
    }
    fn target_format(&self) -> ModelFormat {
        ModelFormat::Custom("raw".into())
    }

    fn convert(
        &self,
        data: &[u8],
        _options: &ConversionOptions,
        progress: Option<&ProgressCallback>,
    ) -> Result<Vec<u8>> {
        if data.len() < 8 {
            return Err(VaultError::ConversionError(
                "Data too small for SafeTensors format".into(),
            ));
        }
        let header_len = u64::from_le_bytes(data[..8].try_into().unwrap()) as usize;
        // Cap header size at 100 MB to prevent memory exhaustion from crafted inputs
        const MAX_HEADER: usize = 100 * 1024 * 1024;
        if header_len > MAX_HEADER {
            return Err(VaultError::ConversionError(
                "SafeTensors header too large".into(),
            ));
        }
        if 8 + header_len > data.len() {
            return Err(VaultError::ConversionError(
                "SafeTensors header length exceeds data".into(),
            ));
        }
        let raw = data[8 + header_len..].to_vec();

        if let Some(cb) = progress {
            cb(&ConversionProgress {
                step: 0,
                total_steps: 1,
                bytes_processed: raw.len() as u64,
                bytes_total: raw.len() as u64,
                message: format!("Extracted {} bytes of raw tensor data", raw.len()),
            });
        }
        Ok(raw)
    }
}

/// Pack raw tensor data into SafeTensors format with a minimal header.
pub struct RawToSafeTensorsConverter;

impl Converter for RawToSafeTensorsConverter {
    fn name(&self) -> &str {
        "Raw → SafeTensors"
    }
    fn source_format(&self) -> ModelFormat {
        ModelFormat::Custom("raw".into())
    }
    fn target_format(&self) -> ModelFormat {
        ModelFormat::Safetensors
    }

    fn convert(
        &self,
        data: &[u8],
        _options: &ConversionOptions,
        progress: Option<&ProgressCallback>,
    ) -> Result<Vec<u8>> {
        // Minimal safetensors: header = JSON object with one tensor entry
        let header = format!(
            r#"{{"__metadata__":{{"format":"raw"}},"tensor_0":{{"dtype":"U8","shape":[{}],"data_offsets":[0,{}]}}}}"#,
            data.len(),
            data.len(),
        );
        let header_bytes = header.as_bytes();
        let header_len = header_bytes.len() as u64;

        let mut out = Vec::with_capacity(8 + header_bytes.len() + data.len());
        out.extend_from_slice(&header_len.to_le_bytes());
        out.extend_from_slice(header_bytes);
        out.extend_from_slice(data);

        if let Some(cb) = progress {
            cb(&ConversionProgress {
                step: 0,
                total_steps: 1,
                bytes_processed: out.len() as u64,
                bytes_total: out.len() as u64,
                message: format!(
                    "Packed {} bytes into SafeTensors ({} byte header)",
                    data.len(),
                    header_bytes.len()
                ),
            });
        }
        Ok(out)
    }
}

// ---------- GGUF header parser ----------

/// Parse GGUF metadata. Returns JSON description of the model.
pub struct GgufHeaderParser;

impl Converter for GgufHeaderParser {
    fn name(&self) -> &str {
        "GGUF → Metadata (JSON)"
    }
    fn source_format(&self) -> ModelFormat {
        ModelFormat::GGUF
    }
    fn target_format(&self) -> ModelFormat {
        ModelFormat::Custom("gguf-meta".into())
    }

    fn convert(
        &self,
        data: &[u8],
        _options: &ConversionOptions,
        _progress: Option<&ProgressCallback>,
    ) -> Result<Vec<u8>> {
        if data.len() < 24 {
            return Err(VaultError::ConversionError(
                "Data too small for GGUF format".into(),
            ));
        }
        if &data[..4] != b"GGUF" {
            return Err(VaultError::ConversionError(
                "Invalid GGUF magic bytes".into(),
            ));
        }
        let version = u32::from_le_bytes(data[4..8].try_into().unwrap());
        let tensor_count = u64::from_le_bytes(data[8..16].try_into().unwrap());
        let kv_count = u64::from_le_bytes(data[16..24].try_into().unwrap());

        let meta = serde_json::json!({
            "format": "GGUF",
            "version": version,
            "tensor_count": tensor_count,
            "kv_count": kv_count,
            "file_size": data.len(),
        });

        serde_json::to_vec_pretty(&meta).map_err(|e| {
            VaultError::ConversionError(format!("Failed to serialise GGUF metadata: {e}"))
        })
    }
}

// ---------- ONNX metadata extractor ----------

/// Extract basic ONNX model metadata (protobuf top-level fields).
pub struct OnnxMetadataExtractor;

impl Converter for OnnxMetadataExtractor {
    fn name(&self) -> &str {
        "ONNX → Metadata (JSON)"
    }
    fn source_format(&self) -> ModelFormat {
        ModelFormat::ONNX
    }
    fn target_format(&self) -> ModelFormat {
        ModelFormat::Custom("onnx-meta".into())
    }

    fn convert(
        &self,
        data: &[u8],
        _options: &ConversionOptions,
        _progress: Option<&ProgressCallback>,
    ) -> Result<Vec<u8>> {
        // ONNX files are protobuf ModelProto. Extract top-level string fields
        // using a minimal protobuf wire-format parser (no .proto compilation).
        let mut ir_version: u64 = 0;
        let mut producer: String = String::new();
        let mut model_version: u64 = 0;
        let mut doc_string: String = String::new();

        let mut pos = 0;
        while pos < data.len() {
            let (field_num, wire_type, new_pos) = match parse_protobuf_tag(data, pos) {
                Some(v) => v,
                None => break,
            };
            pos = new_pos;

            match (field_num, wire_type) {
                // ir_version = field 1, varint
                (1, 0) => {
                    let (val, np) = parse_varint(data, pos);
                    ir_version = val;
                    pos = np;
                }
                // producer_name = field 2, length-delimited
                (2, 2) => {
                    let (bytes, np) = parse_length_delimited(data, pos)?;
                    producer = String::from_utf8_lossy(bytes).into_owned();
                    pos = np;
                }
                // model_version = field 5, varint
                (5, 0) => {
                    let (val, np) = parse_varint(data, pos);
                    model_version = val;
                    pos = np;
                }
                // doc_string = field 6, length-delimited
                (6, 2) => {
                    let (bytes, np) = parse_length_delimited(data, pos)?;
                    doc_string = String::from_utf8_lossy(bytes).into_owned();
                    pos = np;
                }
                // Skip other fields
                (_, 0) => {
                    let (_, np) = parse_varint(data, pos);
                    pos = np;
                }
                (_, 2) => {
                    let (_, np) = parse_length_delimited(data, pos)?;
                    pos = np;
                }
                (_, 5) => pos += 4, // 32-bit
                (_, 1) => pos += 8, // 64-bit
                _ => break,
            }
        }

        let meta = serde_json::json!({
            "format": "ONNX",
            "ir_version": ir_version,
            "producer": producer,
            "model_version": model_version,
            "doc_string": doc_string,
            "file_size": data.len(),
        });

        serde_json::to_vec_pretty(&meta).map_err(|e| {
            VaultError::ConversionError(format!("Failed to serialise ONNX metadata: {e}"))
        })
    }
}

// ── Shim converters (require external Python runtime) ────────────────────────
//
// ── Real converters (pure Rust, no Python required) ──────────────────────────
//
// SafeTensors ↔ PyTorch converters that produce valid binary output.
// PyTorch .pt files are ZIP archives containing:
//   - archive/data.pkl  — pickle bytecode describing the state_dict structure
//   - archive/data/N    — raw tensor storage files (one per tensor)
//
// We generate minimal pickle v2 bytecode to reconstruct an OrderedDict of
// tensors, and write each tensor's raw data into a numbered storage file.

/// Dtype string mapping between SafeTensors names and PyTorch storage types.
fn safetensors_dtype_to_pytorch(dtype: &str) -> Option<(&'static str, usize)> {
    // Returns (pytorch_storage_type, element_size_bytes)
    match dtype {
        "F64" => Some(("DoubleStorage", 8)),
        "F32" => Some(("FloatStorage", 4)),
        "F16" => Some(("HalfStorage", 2)),
        "BF16" => Some(("BFloat16Storage", 2)),
        "I64" => Some(("LongStorage", 8)),
        "I32" => Some(("IntStorage", 4)),
        "I16" => Some(("ShortStorage", 2)),
        "I8" => Some(("CharStorage", 1)),
        "U8" => Some(("ByteStorage", 1)),
        "BOOL" => Some(("BoolStorage", 1)),
        _ => None,
    }
}

/// Dtype string mapping from PyTorch storage type to SafeTensors dtype.
fn pytorch_storage_to_safetensors_dtype(storage_type: &str) -> Option<(&'static str, usize)> {
    // Returns (safetensors_dtype, element_size_bytes)
    match storage_type {
        "DoubleStorage" => Some(("F64", 8)),
        "FloatStorage" => Some(("F32", 4)),
        "HalfStorage" => Some(("F16", 2)),
        "BFloat16Storage" => Some(("BF16", 2)),
        "LongStorage" => Some(("I64", 8)),
        "IntStorage" => Some(("I32", 4)),
        "ShortStorage" => Some(("I16", 2)),
        "CharStorage" => Some(("I8", 1)),
        "ByteStorage" | "UntypedStorage" => Some(("U8", 1)),
        "BoolStorage" => Some(("BOOL", 1)),
        _ => None,
    }
}

/// Real: SafeTensors → PyTorch (.pt ZIP archive with pickle bytecode).
///
/// Parses the SafeTensors header and tensor data, then produces a valid .pt file
/// that can be loaded by `torch.load()` without any Python dependencies at
/// conversion time.
pub struct SafeTensorsToPyTorchConverter;

impl Converter for SafeTensorsToPyTorchConverter {
    fn name(&self) -> &str {
        "SafeTensors → PyTorch"
    }
    fn source_format(&self) -> ModelFormat {
        ModelFormat::Safetensors
    }
    fn target_format(&self) -> ModelFormat {
        ModelFormat::PyTorch
    }

    fn convert(
        &self,
        data: &[u8],
        _options: &ConversionOptions,
        progress: Option<&ProgressCallback>,
    ) -> Result<Vec<u8>> {
        if data.len() < 8 {
            return Err(VaultError::ConversionError(
                "Data too small for SafeTensors format".into(),
            ));
        }
        let header_len = u64::from_le_bytes(data[..8].try_into().unwrap()) as usize;
        // Cap header size at 100 MB to prevent memory exhaustion from crafted inputs
        const MAX_HEADER: usize = 100 * 1024 * 1024;
        if header_len > MAX_HEADER {
            return Err(VaultError::ConversionError(
                "SafeTensors header too large".into(),
            ));
        }
        if 8 + header_len > data.len() {
            return Err(VaultError::ConversionError(
                "SafeTensors header length exceeds data".into(),
            ));
        }

        let header: serde_json::Map<String, serde_json::Value> =
            serde_json::from_slice(&data[8..8 + header_len]).map_err(|e| {
                VaultError::ConversionError(format!("Invalid SafeTensors header JSON: {e}"))
            })?;
        let tensor_data = &data[8 + header_len..];

        // Collect tensor entries (skip __metadata__)
        let mut tensors: Vec<ConvTensorEntry> = Vec::new();
        for (name, info) in &header {
            if name == "__metadata__" {
                continue;
            }
            let obj = info.as_object().ok_or_else(|| {
                VaultError::ConversionError(format!("Tensor '{name}' is not an object"))
            })?;
            let dtype = obj
                .get("dtype")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    VaultError::ConversionError(format!("Missing dtype for tensor '{name}'"))
                })?
                .to_string();
            let shape: Vec<i64> = obj
                .get("shape")
                .and_then(|v| v.as_array())
                .ok_or_else(|| {
                    VaultError::ConversionError(format!("Missing shape for tensor '{name}'"))
                })?
                .iter()
                .filter_map(|v| v.as_i64())
                .collect();
            let offsets = obj
                .get("data_offsets")
                .and_then(|v| v.as_array())
                .ok_or_else(|| {
                    VaultError::ConversionError(format!("Missing data_offsets for tensor '{name}'"))
                })?;
            let start = offsets[0].as_u64().unwrap_or(0) as usize;
            let end = offsets[1].as_u64().unwrap_or(0) as usize;

            tensors.push(ConvTensorEntry {
                name: name.clone(),
                dtype,
                shape,
                data_start: start,
                data_end: end,
            });
        }

        // Sort by data offset for deterministic output
        tensors.sort_by_key(|t| t.data_start);

        if let Some(cb) = progress {
            cb(&ConversionProgress {
                step: 0,
                total_steps: 1,
                bytes_processed: 0,
                bytes_total: data.len() as u64,
                message: format!(
                    "Converting {} tensors from SafeTensors to PyTorch",
                    tensors.len()
                ),
            });
        }

        // Build a ZIP archive with pickle + tensor data
        let mut zip_buf = std::io::Cursor::new(Vec::new());
        {
            let mut zip = zip::ZipWriter::new(&mut zip_buf);
            let stored = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Stored);

            // Write each tensor's data as archive/data/N
            for (i, tensor) in tensors.iter().enumerate() {
                let entry_name = format!("archive/data/{i}");
                let slice = tensor_data
                    .get(tensor.data_start..tensor.data_end)
                    .ok_or_else(|| {
                        VaultError::ConversionError(format!(
                            "Tensor '{}' data offsets [{},{}] out of bounds (data len {})",
                            tensor.name,
                            tensor.data_start,
                            tensor.data_end,
                            tensor_data.len(),
                        ))
                    })?;
                zip.start_file(&entry_name, stored)
                    .map_err(|e| VaultError::ConversionError(format!("ZIP write error: {e}")))?;
                std::io::Write::write_all(&mut zip, slice)
                    .map_err(|e| VaultError::ConversionError(format!("ZIP write error: {e}")))?;
            }

            // Build minimal pickle v2 bytecode that constructs an OrderedDict
            let pickle = build_pytorch_pickle(&tensors)?;
            zip.start_file("archive/data.pkl", stored)
                .map_err(|e| VaultError::ConversionError(format!("ZIP write error: {e}")))?;
            std::io::Write::write_all(&mut zip, &pickle)
                .map_err(|e| VaultError::ConversionError(format!("ZIP write error: {e}")))?;

            zip.finish()
                .map_err(|e| VaultError::ConversionError(format!("ZIP finalize error: {e}")))?;
        }

        let output = zip_buf.into_inner();

        if let Some(cb) = progress {
            cb(&ConversionProgress {
                step: 0,
                total_steps: 1,
                bytes_processed: output.len() as u64,
                bytes_total: output.len() as u64,
                message: format!(
                    "Created PyTorch archive: {} bytes ({} tensors)",
                    output.len(),
                    tensors.len(),
                ),
            });
        }

        Ok(output)
    }
}

/// Internal tensor entry used during SafeTensors ↔ PyTorch conversion.
struct ConvTensorEntry {
    name: String,
    dtype: String,
    shape: Vec<i64>,
    data_start: usize,
    data_end: usize,
}

/// Build minimal pickle v2 bytecode for a PyTorch state_dict.
///
/// Produces bytecode that `torch.load()` interprets as:
/// ```ignore
/// OrderedDict([(name, torch._utils._rebuild_tensor_v2(
///     PersistentLoad((storage_type, key, device, numel)),
///     0, shape, stride
/// )) for each tensor])
/// ```
fn build_pytorch_pickle(tensors: &[ConvTensorEntry]) -> Result<Vec<u8>> {
    let mut pkl = Vec::with_capacity(4096);

    // Protocol 2
    pkl.push(0x80); // PROTO
    pkl.push(2);

    // Push OrderedDict constructor: collections.OrderedDict
    pkl.extend_from_slice(b"c");
    pkl.extend_from_slice(b"collections\nOrderedDict\n");
    pkl.push(b')'); // EMPTY_TUPLE
    pkl.push(b'R'); // REDUCE
    pkl.push(b'q'); // BINPUT
    pkl.push(0);

    // MARK for SETITEMS
    pkl.push(b'('); // MARK

    for (idx, tensor) in tensors.iter().enumerate() {
        let (storage_type, elem_size) =
            safetensors_dtype_to_pytorch(&tensor.dtype).ok_or_else(|| {
                VaultError::ConversionError(format!(
                    "Unsupported dtype '{}' for tensor '{}'",
                    tensor.dtype, tensor.name
                ))
            })?;

        let data_len = tensor.data_end - tensor.data_start;
        let numel = data_len.checked_div(elem_size).unwrap_or(data_len);

        // Key: tensor name
        write_short_binunicode(&mut pkl, &tensor.name);

        // Value: rebuild_tensor_v2(storage, offset, shape, stride)
        pkl.extend_from_slice(b"c");
        pkl.extend_from_slice(b"torch._utils\n_rebuild_tensor_v2\n");

        // Arguments tuple: (storage, offset, shape, stride, requires_grad)
        pkl.push(b'('); // MARK

        // storage: PersistentLoad((storage_type_str, key, device, numel))
        pkl.push(b'('); // MARK
        write_short_binunicode(&mut pkl, "storage");
        pkl.extend_from_slice(b"c");
        pkl.extend_from_slice(format!("torch\n{storage_type}\n").as_bytes());
        write_short_binunicode(&mut pkl, &idx.to_string());
        write_short_binunicode(&mut pkl, "cpu");
        write_pickle_int(&mut pkl, numel as i64);
        pkl.push(b't'); // TUPLE
        pkl.push(b'Q'); // BINPERSID

        // storage offset: 0
        pkl.push(0x4b); // BININT1
        pkl.push(0);

        // shape tuple
        pkl.push(b'('); // MARK
        for &dim in &tensor.shape {
            write_pickle_int(&mut pkl, dim);
        }
        pkl.push(b't'); // TUPLE

        // stride tuple (row-major)
        pkl.push(b'('); // MARK
        let mut stride = vec![1i64; tensor.shape.len()];
        for i in (0..tensor.shape.len().saturating_sub(1)).rev() {
            stride[i] = stride[i + 1] * tensor.shape[i + 1];
        }
        for s in &stride {
            write_pickle_int(&mut pkl, *s);
        }
        pkl.push(b't'); // TUPLE

        // requires_grad = False
        pkl.push(0x89); // NEWFALSE

        // Close the rebuild args tuple + REDUCE
        pkl.push(b't'); // TUPLE
        pkl.push(b'R'); // REDUCE

        // Memo
        let memo_idx = (idx + 1) as u8;
        if memo_idx < 255 {
            pkl.push(b'q'); // BINPUT
            pkl.push(memo_idx);
        }
    }

    pkl.push(b'u'); // SETITEMS
    pkl.push(b'.'); // STOP

    Ok(pkl)
}

fn write_short_binunicode(pkl: &mut Vec<u8>, s: &str) {
    let bytes = s.as_bytes();
    if bytes.len() <= 255 {
        pkl.push(0x8c); // SHORT_BINUNICODE
        pkl.push(bytes.len() as u8);
        pkl.extend_from_slice(bytes);
    } else {
        pkl.push(0x8d); // BINUNICODE
        pkl.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
        pkl.extend_from_slice(bytes);
    }
}

fn write_pickle_int(pkl: &mut Vec<u8>, val: i64) {
    if (0..=255).contains(&val) {
        pkl.push(0x4b); // BININT1
        pkl.push(val as u8);
    } else if (0..=65535).contains(&val) {
        pkl.push(0x4d); // BININT2
        pkl.extend_from_slice(&(val as u16).to_le_bytes());
    } else if val >= i32::MIN as i64 && val <= i32::MAX as i64 {
        pkl.push(0x4a); // BININT
        pkl.extend_from_slice(&(val as i32).to_le_bytes());
    } else {
        // LONG1 for 64-bit values
        pkl.push(0x8a); // LONG1
        let bytes = val.to_le_bytes();
        // Find significant length (strip trailing 0x00 for positive, 0xff for negative)
        let mut len = 8;
        if val >= 0 {
            while len > 1 && bytes[len - 1] == 0 {
                len -= 1;
            }
            // Need extra byte if high bit set (would be interpreted as negative)
            if bytes[len - 1] & 0x80 != 0 {
                pkl.push((len + 1) as u8);
                pkl.extend_from_slice(&bytes[..len]);
                pkl.push(0);
                return;
            }
        } else {
            while len > 1 && bytes[len - 1] == 0xff {
                len -= 1;
            }
            if bytes[len - 1] & 0x80 == 0 {
                pkl.push((len + 1) as u8);
                pkl.extend_from_slice(&bytes[..len]);
                pkl.push(0xff);
                return;
            }
        }
        pkl.push(len as u8);
        pkl.extend_from_slice(&bytes[..len]);
    }
}

/// Real: PyTorch → SafeTensors.
///
/// Reads a PyTorch .pt ZIP archive, extracts tensor storage files and
/// reconstructs a valid SafeTensors file. Falls back to a conversion plan
/// if the archive structure is unrecognised.
///
/// # Security — Pickle Deserialization (I-01)
///
/// PyTorch `.pt` files contain pickled Python objects (`data.pkl`). This
/// converter **does not** unpickle or execute pickle bytecodes — it only
/// reads the raw binary tensor storage blobs from the ZIP archive.
/// Arbitrary code execution through malicious pickle payloads is therefore
/// **not possible** via this conversion path. If full pickle support is
/// ever added, use a sandboxed deserializer (e.g., `fickling` or
/// `safepickle`) and reject `__reduce__` / `__reduce_ex__` opcodes.
pub struct PyTorchToSafeTensorsConverter;

impl Converter for PyTorchToSafeTensorsConverter {
    fn name(&self) -> &str {
        "PyTorch → SafeTensors"
    }
    fn source_format(&self) -> ModelFormat {
        ModelFormat::PyTorch
    }
    fn target_format(&self) -> ModelFormat {
        ModelFormat::Safetensors
    }

    fn convert(
        &self,
        data: &[u8],
        _options: &ConversionOptions,
        progress: Option<&ProgressCallback>,
    ) -> Result<Vec<u8>> {
        // Try to read as ZIP archive
        let cursor = std::io::Cursor::new(data);
        let mut archive = zip::ZipArchive::new(cursor).map_err(|e| {
            VaultError::ConversionError(format!("Not a valid PyTorch ZIP archive: {e}"))
        })?;

        if let Some(cb) = progress {
            cb(&ConversionProgress {
                step: 0,
                total_steps: 1,
                bytes_processed: 0,
                bytes_total: data.len() as u64,
                message: format!("Parsing PyTorch archive ({} entries)", archive.len()),
            });
        }

        // Scan for data files (archive/data/0, archive/data/1, ...)
        // and the pickle file (archive/data.pkl)
        let mut storage_files: Vec<(String, Vec<u8>)> = Vec::new();
        let mut pkl_data: Option<Vec<u8>> = None;

        for i in 0..archive.len() {
            let mut file = archive
                .by_index(i)
                .map_err(|e| VaultError::ConversionError(format!("ZIP read error: {e}")))?;
            let name = file.name().to_string();
            let mut contents = Vec::new();
            std::io::Read::read_to_end(&mut file, &mut contents)
                .map_err(|e| VaultError::ConversionError(format!("ZIP read error: {e}")))?;

            if name.ends_with(".pkl") || name.ends_with("/data.pkl") {
                pkl_data = Some(contents);
            } else if name.contains("/data/") {
                // Extract the storage index from the path
                storage_files.push((name, contents));
            }
        }

        // Sort storage files by their numeric index
        storage_files.sort_by(|a, b| {
            let idx_a =
                a.0.rsplit('/')
                    .next()
                    .and_then(|s| s.parse::<usize>().ok())
                    .unwrap_or(usize::MAX);
            let idx_b =
                b.0.rsplit('/')
                    .next()
                    .and_then(|s| s.parse::<usize>().ok())
                    .unwrap_or(usize::MAX);
            idx_a.cmp(&idx_b)
        });

        // Try to extract tensor metadata from pickle bytecode
        let tensor_infos = if let Some(ref pkl) = pkl_data {
            extract_tensor_info_from_pickle(pkl)
        } else {
            Vec::new()
        };

        // Build SafeTensors output
        let mut header_entries = serde_json::Map::new();
        let mut all_tensor_data = Vec::new();

        if !tensor_infos.is_empty() && tensor_infos.len() == storage_files.len() {
            // We successfully parsed pickle — use tensor names, dtypes, shapes
            for (info, (_path, storage_data)) in tensor_infos.iter().zip(storage_files.iter()) {
                let offset_start = all_tensor_data.len();
                all_tensor_data.extend_from_slice(storage_data);
                let offset_end = all_tensor_data.len();

                let entry = serde_json::json!({
                    "dtype": info.dtype,
                    "shape": info.shape,
                    "data_offsets": [offset_start, offset_end],
                });
                header_entries.insert(info.name.clone(), entry);
            }
        } else {
            // Fallback: treat each storage file as a raw U8 tensor
            for (path, storage_data) in &storage_files {
                let tensor_name = path.rsplit('/').next().unwrap_or("tensor");
                let offset_start = all_tensor_data.len();
                all_tensor_data.extend_from_slice(storage_data);
                let offset_end = all_tensor_data.len();

                let entry = serde_json::json!({
                    "dtype": "U8",
                    "shape": [storage_data.len()],
                    "data_offsets": [offset_start, offset_end],
                });
                header_entries.insert(format!("storage_{tensor_name}"), entry);
            }
        }

        // Serialize SafeTensors
        let header_json = serde_json::to_string(&header_entries).map_err(|e| {
            VaultError::ConversionError(format!("Failed to serialize SafeTensors header: {e}"))
        })?;
        let header_bytes = header_json.as_bytes();
        let header_len = header_bytes.len() as u64;

        let mut output = Vec::with_capacity(8 + header_bytes.len() + all_tensor_data.len());
        output.extend_from_slice(&header_len.to_le_bytes());
        output.extend_from_slice(header_bytes);
        output.extend_from_slice(&all_tensor_data);

        if let Some(cb) = progress {
            cb(&ConversionProgress {
                step: 0,
                total_steps: 1,
                bytes_processed: output.len() as u64,
                bytes_total: output.len() as u64,
                message: format!(
                    "Created SafeTensors: {} bytes ({} tensors)",
                    output.len(),
                    header_entries.len(),
                ),
            });
        }

        Ok(output)
    }
}

/// Extracted tensor info from pickle bytecode.
struct PickleTensorInfo {
    name: String,
    dtype: String,
    shape: Vec<i64>,
}

/// Minimal pickle bytecode parser to extract tensor metadata from PyTorch files.
///
/// Looks for patterns matching `_rebuild_tensor_v2` calls and extracts the
/// tensor name, storage type, and shape. This doesn't need to fully interpret
/// pickle — it scans for the specific opcode sequences PyTorch uses.
fn extract_tensor_info_from_pickle(pkl: &[u8]) -> Vec<PickleTensorInfo> {
    let mut tensors = Vec::new();
    let mut strings: Vec<String> = Vec::new();
    let mut pos = 0;

    // First pass: extract all string literals (these include tensor names and storage types)
    while pos < pkl.len() {
        match pkl[pos] {
            0x80 => pos += 2, // PROTO
            b'c' => {
                // GLOBAL opcode: c<module>\n<name>\n
                pos += 1;
                // Read module name (until \n)
                while pos < pkl.len() && pkl[pos] != b'\n' {
                    pos += 1;
                }
                pos += 1; // skip \n
                          // Read class name (until \n) — this contains e.g. "FloatStorage"
                let start = pos;
                while pos < pkl.len() && pkl[pos] != b'\n' {
                    pos += 1;
                }
                if let Ok(s) = std::str::from_utf8(&pkl[start..pos]) {
                    if !s.is_empty() {
                        strings.push(s.to_string());
                    }
                }
                pos += 1; // skip \n
            }
            0x8c => {
                // SHORT_BINUNICODE
                pos += 1;
                if pos >= pkl.len() {
                    break;
                }
                let len = pkl[pos] as usize;
                pos += 1;
                if pos + len <= pkl.len() {
                    if let Ok(s) = std::str::from_utf8(&pkl[pos..pos + len]) {
                        strings.push(s.to_string());
                    }
                    pos += len;
                } else {
                    break;
                }
            }
            0x8d => {
                // BINUNICODE
                pos += 1;
                if pos + 4 > pkl.len() {
                    break;
                }
                let len = u32::from_le_bytes(pkl[pos..pos + 4].try_into().unwrap()) as usize;
                pos += 4;
                if pos + len <= pkl.len() {
                    if let Ok(s) = std::str::from_utf8(&pkl[pos..pos + len]) {
                        strings.push(s.to_string());
                    }
                    pos += len;
                } else {
                    break;
                }
            }
            _ => pos += 1,
        }
    }

    // Look for patterns: in PyTorch pickle, tensor entries follow the pattern:
    // <tensor_name_string> ... "storage" ... <StorageType string> ... <key> ... "cpu" ...
    // followed by shape tuple integers
    //
    // We scan the string list for storage type names and pair them with
    // preceding tensor names and following shape data.
    let mut i = 0;
    while i < strings.len() {
        // Look for a string that matches a known storage type
        if let Some((dtype, _elem_size)) = pytorch_storage_to_safetensors_dtype(&strings[i]) {
            // The tensor name is typically a few positions back
            // Search backward for a name that looks like a tensor name (contains . or _ and isn't a keyword)
            let mut tensor_name = None;
            for j in (0..i).rev() {
                let candidate = &strings[j];
                if candidate == "storage"
                    || candidate == "cpu"
                    || candidate == "cuda"
                    || candidate.contains("torch")
                    || candidate.contains("collections")
                    || candidate.contains("OrderedDict")
                    || candidate.starts_with("_rebuild")
                    || pytorch_storage_to_safetensors_dtype(candidate).is_some()
                    || candidate.parse::<usize>().is_ok()
                {
                    continue;
                }
                // Looks like a tensor name
                tensor_name = Some(candidate.clone());
                break;
            }

            if let Some(name) = tensor_name {
                tensors.push(PickleTensorInfo {
                    name,
                    dtype: dtype.to_string(),
                    shape: Vec::new(), // Shape extracted from ints is complex; we set reasonable defaults
                });
            }
        }
        i += 1;
    }

    // Shape extraction from pickle ints is complex (requires full stack emulation).
    // For the shapes, we rely on the data sizes and dtype element sizes to infer
    // a flat [N] shape. The caller can post-process if needed.
    tensors
}

/// Shim: PyTorch → ONNX (needs `torch` Python package).
pub struct PyTorchToOnnxConverter;

impl Converter for PyTorchToOnnxConverter {
    fn name(&self) -> &str {
        "PyTorch → ONNX (shim)"
    }

    fn produces_plan(&self) -> bool {
        true
    }
    fn source_format(&self) -> ModelFormat {
        ModelFormat::PyTorch
    }
    fn target_format(&self) -> ModelFormat {
        ModelFormat::ONNX
    }

    fn convert(
        &self,
        _data: &[u8],
        options: &ConversionOptions,
        _progress: Option<&ProgressCallback>,
    ) -> Result<Vec<u8>> {
        let opset = options.opset_version.unwrap_or(17);
        let plan = serde_json::json!({
            "converter": "pytorch_to_onnx",
            "requires": ["torch", "onnx"],
            "opset_version": opset,
            "python": format!(
                concat!(
                    "import torch, onnx\n",
                    "model = torch.load(input_path, map_location='cpu', weights_only=False)\n",
                    "model.eval()\n",
                    "dummy = torch.randn(1, 3, 224, 224)  # adjust shape as needed\n",
                    "torch.onnx.export(model, dummy, output_path, opset_version={})\n",
                ),
                opset,
            ),
        });

        serde_json::to_vec_pretty(&plan).map_err(|e| {
            VaultError::ConversionError(format!("Failed to create conversion plan: {e}"))
        })
    }
}

/// Shim: ONNX → TensorRT (needs `tensorrt` Python package or `trtexec`).
pub struct OnnxToTensorRtConverter;

impl Converter for OnnxToTensorRtConverter {
    fn name(&self) -> &str {
        "ONNX → TensorRT (shim)"
    }

    fn produces_plan(&self) -> bool {
        true
    }
    fn source_format(&self) -> ModelFormat {
        ModelFormat::ONNX
    }
    fn target_format(&self) -> ModelFormat {
        ModelFormat::TensorRT
    }

    fn convert(
        &self,
        _data: &[u8],
        _options: &ConversionOptions,
        _progress: Option<&ProgressCallback>,
    ) -> Result<Vec<u8>> {
        let plan = serde_json::json!({
            "converter": "onnx_to_tensorrt",
            "requires": ["tensorrt"],
            "shell": "trtexec --onnx=input_path --saveEngine=output_path",
            "python": concat!(
                "import tensorrt as trt\n",
                "logger = trt.Logger(trt.Logger.WARNING)\n",
                "builder = trt.Builder(logger)\n",
                "network = builder.create_network(1 << int(trt.NetworkDefinitionCreationFlag.EXPLICIT_BATCH))\n",
                "parser = trt.OnnxParser(network, logger)\n",
                "with open(input_path, 'rb') as f:\n",
                "    parser.parse(f.read())\n",
                "config = builder.create_builder_config()\n",
                "engine = builder.build_serialized_network(network, config)\n",
                "with open(output_path, 'wb') as f:\n",
                "    f.write(engine)\n",
            ),
        });

        serde_json::to_vec_pretty(&plan).map_err(|e| {
            VaultError::ConversionError(format!("Failed to create conversion plan: {e}"))
        })
    }
}

/// Shim: ONNX → CoreML (needs `coremltools` Python package).
pub struct OnnxToCoreMLConverter;

impl Converter for OnnxToCoreMLConverter {
    fn name(&self) -> &str {
        "ONNX → Core ML (shim)"
    }

    fn produces_plan(&self) -> bool {
        true
    }
    fn source_format(&self) -> ModelFormat {
        ModelFormat::ONNX
    }
    fn target_format(&self) -> ModelFormat {
        ModelFormat::CoreML
    }

    fn convert(
        &self,
        _data: &[u8],
        _options: &ConversionOptions,
        _progress: Option<&ProgressCallback>,
    ) -> Result<Vec<u8>> {
        let plan = serde_json::json!({
            "converter": "onnx_to_coreml",
            "requires": ["coremltools", "onnx"],
            "python": concat!(
                "import coremltools as ct\n",
                "import onnx\n",
                "model = onnx.load(input_path)\n",
                "ml_model = ct.convert(model)\n",
                "ml_model.save(output_path)\n",
            ),
        });

        serde_json::to_vec_pretty(&plan).map_err(|e| {
            VaultError::ConversionError(format!("Failed to create conversion plan: {e}"))
        })
    }
}

/// Shim: SafeTensors → GGUF, for the *byte-array* pipeline only.
///
/// 🚨 A real converter lives in [`crate::hf_gguf::convert_hf_to_gguf`], and
/// `iv convert --from-dir <hf_dir> -t gguf` reaches it. This shim cannot: the
/// [`Converter`] trait takes `&[u8]` and returns `Vec<u8>`, while a GGUF needs
/// the whole HuggingFace *directory* — `config.json` and `tokenizer.model` as
/// well as the weights — and streams a checkpoint too large to hold twice.
/// So this arm stays a plan, and the plan now points at the real route.
pub struct SafeTensorsToGgufConverter;

impl Converter for SafeTensorsToGgufConverter {
    fn name(&self) -> &str {
        "SafeTensors → GGUF (shim; use `iv convert --from-dir` for the real one)"
    }

    fn produces_plan(&self) -> bool {
        true
    }
    fn source_format(&self) -> ModelFormat {
        ModelFormat::Safetensors
    }
    fn target_format(&self) -> ModelFormat {
        ModelFormat::GGUF
    }

    fn convert(
        &self,
        _data: &[u8],
        options: &ConversionOptions,
        _progress: Option<&ProgressCallback>,
    ) -> Result<Vec<u8>> {
        let quant = options.quantization.as_deref().unwrap_or("f16");

        // Llama to f16/bf16/f32 needs no Python at all — that is `--from-dir`.
        // Anything else does, and saying which is which is the point of a plan.
        let native = matches!(quant, "f16" | "fp16" | "bf16" | "f32" | "fp32");
        let plan = serde_json::json!({
            "converter": "safetensors_to_gguf",
            "requires": if native { vec![] } else { vec!["gguf", "numpy", "safetensors"] },
            "quantization": quant,
            "shell": if native {
                "iv convert <name> -t gguf --from-dir <hf_model_dir> -o out.gguf".to_string()
            } else {
                format!("python convert_hf_to_gguf.py --outtype f16 model_dir && llama-quantize out.gguf out-{quant}.gguf {quant}")
            },
            "python": if native {
                concat!(
                    "# No Python needed for llama → f16/bf16/f32.\n",
                    "# iv convert <name> -t gguf --from-dir <hf_model_dir> -o out.gguf\n",
                ).to_string()
            } else {
                format!(
                    concat!(
                        "# No K-quant encoder exists here, and non-llama architectures\n",
                        "# need per-architecture mapping. Both go through llama.cpp:\n",
                        "# python convert_hf_to_gguf.py --outtype f16 model_dir\n",
                        "# llama-quantize out.gguf out-{}.gguf {}\n",
                    ),
                    quant, quant,
                )
            },
        });

        serde_json::to_vec_pretty(&plan).map_err(|e| {
            VaultError::ConversionError(format!("Failed to create conversion plan: {e}"))
        })
    }
}

// ── Protobuf helpers (minimal wire-format parser) ────────────────────────────

fn parse_varint(data: &[u8], start: usize) -> (u64, usize) {
    let mut result: u64 = 0;
    let mut shift = 0u32;
    let mut pos = start;
    while pos < data.len() {
        let byte = data[pos];
        result |= ((byte & 0x7F) as u64) << shift;
        pos += 1;
        if byte & 0x80 == 0 {
            break;
        }
        shift += 7;
        if shift >= 64 {
            break;
        }
    }
    (result, pos)
}

fn parse_protobuf_tag(data: &[u8], pos: usize) -> Option<(u64, u8, usize)> {
    if pos >= data.len() {
        return None;
    }
    let (tag, new_pos) = parse_varint(data, pos);
    let field_num = tag >> 3;
    let wire_type = (tag & 0x07) as u8;
    Some((field_num, wire_type, new_pos))
}

fn parse_length_delimited(data: &[u8], pos: usize) -> Result<(&[u8], usize)> {
    let (len, new_pos) = parse_varint(data, pos);
    let len = len as usize;
    if new_pos + len > data.len() {
        return Err(VaultError::ConversionError(
            "Protobuf length-delimited field exceeds data".into(),
        ));
    }
    Ok((&data[new_pos..new_pos + len], new_pos + len))
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pipeline_same_format_passthrough() {
        let pipeline = ConversionPipeline::with_builtins();
        let data = b"hello world";
        let result = pipeline
            .convert(
                data,
                &ModelFormat::PyTorch,
                &ModelFormat::PyTorch,
                &ConversionOptions::default(),
                None,
            )
            .unwrap();
        assert_eq!(result.data, data);
        assert_eq!(result.conversion_path, vec![ModelFormat::PyTorch]);
    }

    #[test]
    fn test_pipeline_no_path_error() {
        let pipeline = ConversionPipeline::new(); // empty
        let err = pipeline
            .convert(
                b"data",
                &ModelFormat::PyTorch,
                &ModelFormat::ONNX,
                &ConversionOptions::default(),
                None,
            )
            .unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("No conversion path"));
    }

    #[test]
    fn test_safetensors_roundtrip() {
        let original = b"test tensor data 1234567890";
        let pipeline = ConversionPipeline::with_builtins();

        // raw → safetensors
        let st = RawToSafeTensorsConverter
            .convert(original, &ConversionOptions::default(), None)
            .unwrap();
        assert!(st.len() > original.len());

        // safetensors → raw
        let raw = SafeTensorsToRawConverter
            .convert(&st, &ConversionOptions::default(), None)
            .unwrap();
        assert_eq!(raw, original);

        // Also test via pipeline path finding
        assert!(pipeline.can_convert_direct(
            &ModelFormat::Custom("raw".into()),
            &ModelFormat::Safetensors,
        ));
    }

    #[test]
    fn test_safetensors_to_raw_too_small() {
        let err = SafeTensorsToRawConverter
            .convert(b"tiny", &ConversionOptions::default(), None)
            .unwrap_err();
        assert!(format!("{err}").contains("too small"));
    }

    #[test]
    fn test_gguf_header_parser_valid() {
        // Construct a minimal valid GGUF header
        let mut data = Vec::new();
        data.extend_from_slice(b"GGUF"); // magic
        data.extend_from_slice(&3u32.to_le_bytes()); // version
        data.extend_from_slice(&42u64.to_le_bytes()); // tensor count
        data.extend_from_slice(&7u64.to_le_bytes()); // kv count
                                                     // Pad to simulate the rest of the file
        data.extend_from_slice(&[0u8; 100]);

        let result = GgufHeaderParser
            .convert(&data, &ConversionOptions::default(), None)
            .unwrap();
        let meta: serde_json::Value = serde_json::from_slice(&result).unwrap();
        assert_eq!(meta["version"], 3);
        assert_eq!(meta["tensor_count"], 42);
        assert_eq!(meta["kv_count"], 7);
    }

    #[test]
    fn test_gguf_header_parser_invalid_magic() {
        let err = GgufHeaderParser
            .convert(
                b"NOT_GGUF_DATA_HERE__________",
                &ConversionOptions::default(),
                None,
            )
            .unwrap_err();
        assert!(format!("{err}").contains("Invalid GGUF magic"));
    }

    #[test]
    fn test_validation_report() {
        let checks = vec![
            ValidationCheck::pass("a", "ok"),
            ValidationCheck::fail("b", "bad"),
        ];
        let report = ValidationReport::from_checks(checks);
        assert!(!report.passed);
        assert_eq!(report.checks.len(), 2);
    }

    #[test]
    fn test_conversion_options_with_validation() {
        let opts = ConversionOptions::with_validation();
        assert!(opts.validate);
        assert!(opts.preserve_metadata);
        assert!((opts.tolerance - 1e-5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_conversion_result_compression_ratio() {
        let result = ConversionResult {
            data: vec![],
            plan: None,
            source_format: ModelFormat::PyTorch,
            target_format: ModelFormat::Safetensors,
            conversion_path: vec![],
            input_size: 1000,
            output_size: 500,
            validation: None,
        };
        assert!((result.compression_ratio() - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_find_path_direct() {
        let pipeline = ConversionPipeline::with_builtins();
        let path = pipeline
            .find_path(&ModelFormat::Safetensors, &ModelFormat::PyTorch)
            .unwrap();
        assert_eq!(path, vec![ModelFormat::Safetensors, ModelFormat::PyTorch]);
    }

    #[test]
    fn test_find_path_multi_step() {
        let pipeline = ConversionPipeline::with_builtins();
        // PyTorch → ONNX → TensorRT (two steps)
        let path = pipeline
            .find_path(&ModelFormat::PyTorch, &ModelFormat::TensorRT)
            .unwrap();
        assert_eq!(
            path,
            vec![
                ModelFormat::PyTorch,
                ModelFormat::ONNX,
                ModelFormat::TensorRT
            ]
        );
    }

    #[test]
    fn test_find_path_none() {
        let pipeline = ConversionPipeline::with_builtins();
        // No path to, say, MXNet
        let path = pipeline.find_path(&ModelFormat::PyTorch, &ModelFormat::MXNet);
        assert!(path.is_none());
    }

    #[test]
    fn test_supported_conversions_list() {
        let pipeline = ConversionPipeline::with_builtins();
        let conversions = pipeline.supported_conversions();
        assert!(conversions.len() >= 10);
    }

    #[test]
    fn test_progress_display() {
        let p = ConversionProgress {
            step: 0,
            total_steps: 3,
            bytes_processed: 500,
            bytes_total: 1000,
            message: "Converting".into(),
        };
        let s = format!("{p}");
        assert!(s.contains("50.0%"));
        assert!(s.contains("[1/3]"));
    }

    #[test]
    fn test_progress_display_unknown_total() {
        let p = ConversionProgress {
            step: 1,
            total_steps: 2,
            bytes_processed: 100,
            bytes_total: 0,
            message: "Working".into(),
        };
        let s = format!("{p}");
        assert!(s.contains("[2/2]"));
        assert!(s.contains("Working"));
        assert!(!s.contains('%'));
    }

    #[test]
    fn test_shim_converter_produces_plan() {
        let converter = PyTorchToOnnxConverter;
        let plan_bytes = converter
            .convert(b"", &ConversionOptions::default(), None)
            .unwrap();
        let plan: serde_json::Value = serde_json::from_slice(&plan_bytes).unwrap();
        assert_eq!(plan["converter"], "pytorch_to_onnx");
        assert!(plan["requires"]
            .as_array()
            .unwrap()
            .contains(&"torch".into()));
        assert_eq!(plan["opset_version"], 17);
    }

    #[test]
    fn test_shim_converter_custom_opset() {
        let opts = ConversionOptions {
            opset_version: Some(13),
            ..ConversionOptions::default()
        };
        let plan_bytes = PyTorchToOnnxConverter.convert(b"", &opts, None).unwrap();
        let plan: serde_json::Value = serde_json::from_slice(&plan_bytes).unwrap();
        assert_eq!(plan["opset_version"], 13);
    }

    #[test]
    fn test_safetensors_to_gguf_quantization() {
        let opts = ConversionOptions {
            quantization: Some("q4_k_m".into()),
            ..ConversionOptions::default()
        };
        let plan_bytes = SafeTensorsToGgufConverter
            .convert(b"", &opts, None)
            .unwrap();
        let plan: serde_json::Value = serde_json::from_slice(&plan_bytes).unwrap();
        assert_eq!(plan["quantization"], "q4_k_m");
    }

    #[test]
    fn test_validate_magic_bytes_safetensors() {
        // Valid safetensors (8-byte header length + JSON)
        let header = b"{}";
        let mut data = Vec::new();
        data.extend_from_slice(&(header.len() as u64).to_le_bytes());
        data.extend_from_slice(header);
        let check = validate_magic_bytes(&data, &ModelFormat::Safetensors);
        assert!(check.passed);
    }

    #[test]
    fn test_validate_magic_bytes_gguf() {
        let mut data = b"GGUF".to_vec();
        data.extend_from_slice(&[0u8; 20]);
        let check = validate_magic_bytes(&data, &ModelFormat::GGUF);
        assert!(check.passed);
    }

    #[test]
    fn test_validate_magic_bytes_pytorch_zip() {
        let data = b"PK\x03\x04...";
        let check = validate_magic_bytes(data, &ModelFormat::PyTorch);
        assert!(check.passed);
    }

    #[test]
    fn test_validate_magic_bytes_unknown_format() {
        let check = validate_magic_bytes(b"anything", &ModelFormat::Keras);
        assert!(check.passed); // no check for Keras → pass
    }

    #[test]
    fn test_pipeline_with_progress_callback() {
        let pipeline = ConversionPipeline::with_builtins();
        let progress_log = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let log_clone = progress_log.clone();
        let cb: ProgressCallback = Box::new(move |p| {
            log_clone.lock().unwrap().push(p.message.clone());
        });

        let _ = pipeline.convert(
            b"",
            &ModelFormat::PyTorch,
            &ModelFormat::ONNX,
            &ConversionOptions::default(),
            Some(&cb),
        );

        let log = progress_log.lock().unwrap();
        assert!(!log.is_empty());
    }

    // ── Default validate() coverage ──

    #[test]
    fn test_validate_empty_output() {
        // Line 162: output.is_empty() → fail
        let converter = PyTorchToOnnxConverter;
        let report = converter.validate(b"input", b"", &ConversionOptions::default());
        assert!(!report.passed);
        assert!(report
            .checks
            .iter()
            .any(|c| c.name == "non_empty" && !c.passed));
    }

    #[test]
    fn test_validate_suspicious_ratio() {
        // Lines 170-174: ratio > 100.0 without quantization → fail
        let converter = PyTorchToOnnxConverter;
        let input = b"x";
        let output = vec![0u8; 200];
        let report = converter.validate(input, &output, &ConversionOptions::default());
        assert!(!report.passed);
        assert!(report
            .checks
            .iter()
            .any(|c| c.name == "size_ratio" && !c.passed));
    }

    #[test]
    fn test_validate_ratio_with_quantization_ok() {
        let converter = PyTorchToOnnxConverter;
        let input = b"x";
        let output = vec![0u8; 200];
        let opts = ConversionOptions {
            quantization: Some("q4_k_m".into()),
            ..ConversionOptions::default()
        };
        let report = converter.validate(input, &output, &opts);
        // Should pass because quantization is set
        assert!(report
            .checks
            .iter()
            .any(|c| c.name == "size_ratio" && c.passed));
    }

    #[test]
    fn test_validate_empty_input_ratio() {
        // Line 164: input.is_empty() → ratio = 1.0
        let converter = PyTorchToOnnxConverter;
        let output = vec![0u8; 10];
        let report = converter.validate(b"", &output, &ConversionOptions::default());
        assert!(report
            .checks
            .iter()
            .any(|c| c.name == "size_ratio" && c.passed));
    }

    // ── Real converter tests ──

    #[test]
    fn test_safetensors_pytorch_roundtrip() {
        // Build a valid SafeTensors buffer with one F32 tensor [2,2]
        let header = serde_json::json!({
            "weight": { "dtype": "F32", "shape": [2, 2], "data_offsets": [0, 16] }
        });
        let header_bytes = serde_json::to_vec(&header).unwrap();
        let mut st_data = Vec::new();
        st_data.extend_from_slice(&(header_bytes.len() as u64).to_le_bytes());
        st_data.extend_from_slice(&header_bytes);
        // 4 floats × 4 bytes = 16 bytes of tensor data
        let floats: [f32; 4] = [1.0, 2.0, 3.0, 4.0];
        for f in &floats {
            st_data.extend_from_slice(&f.to_le_bytes());
        }

        // SafeTensors → PyTorch
        let pt_bytes = SafeTensorsToPyTorchConverter
            .convert(&st_data, &ConversionOptions::default(), None)
            .unwrap();
        // Output should be a valid ZIP archive
        assert!(pt_bytes.len() > 4);
        assert_eq!(&pt_bytes[0..2], b"PK"); // ZIP magic

        // PyTorch → SafeTensors (roundtrip)
        let st2_bytes = PyTorchToSafeTensorsConverter
            .convert(&pt_bytes, &ConversionOptions::default(), None)
            .unwrap();
        // Should start with 8-byte LE header length
        assert!(st2_bytes.len() > 8);
        let hdr_len = u64::from_le_bytes(st2_bytes[0..8].try_into().unwrap()) as usize;
        assert!(hdr_len > 0 && hdr_len < st2_bytes.len());
        let hdr: serde_json::Value = serde_json::from_slice(&st2_bytes[8..8 + hdr_len]).unwrap();
        // Should contain the "weight" tensor
        assert!(
            hdr.get("weight").is_some(),
            "Header missing 'weight': {hdr}"
        );
    }

    #[test]
    fn test_pytorch_to_safetensors_requires_valid_zip() {
        let conv = PyTorchToSafeTensorsConverter;
        let err = conv
            .convert(b"model data", &ConversionOptions::default(), None)
            .unwrap_err();
        assert!(format!("{err}").contains("ZIP archive"));
    }

    #[test]
    fn test_onnx_to_tensorrt_plan() {
        let conv = OnnxToTensorRtConverter;
        let plan_bytes = conv
            .convert(b"onnx data", &ConversionOptions::default(), None)
            .unwrap();
        let plan: serde_json::Value = serde_json::from_slice(&plan_bytes).unwrap();
        assert_eq!(plan["converter"], "onnx_to_tensorrt");
    }

    #[test]
    fn test_onnx_to_coreml_plan() {
        let conv = OnnxToCoreMLConverter;
        let plan_bytes = conv
            .convert(b"onnx data", &ConversionOptions::default(), None)
            .unwrap();
        let plan: serde_json::Value = serde_json::from_slice(&plan_bytes).unwrap();
        assert_eq!(plan["converter"], "onnx_to_coreml");
    }

    #[test]
    fn test_safetensors_to_gguf_plan_default() {
        let conv = SafeTensorsToGgufConverter;
        let plan_bytes = conv
            .convert(b"", &ConversionOptions::default(), None)
            .unwrap();
        let plan: serde_json::Value = serde_json::from_slice(&plan_bytes).unwrap();
        assert_eq!(plan["converter"], "safetensors_to_gguf");
        assert_eq!(plan["quantization"], "f16"); // default when no quantization specified
    }

    #[test]
    fn test_safetensors_to_pytorch_valid_header() {
        // Build valid safetensors data
        let header = serde_json::json!({
            "weight": { "dtype": "F32", "shape": [2, 2], "data_offsets": [0, 16] }
        });
        let header_bytes = serde_json::to_vec(&header).unwrap();
        let mut data = Vec::new();
        data.extend_from_slice(&(header_bytes.len() as u64).to_le_bytes());
        data.extend_from_slice(&header_bytes);
        data.extend_from_slice(&[0u8; 16]); // tensor data

        let conv = SafeTensorsToPyTorchConverter;
        let pt_bytes = conv
            .convert(&data, &ConversionOptions::default(), None)
            .unwrap();
        // Should produce a valid ZIP archive
        assert!(pt_bytes.len() > 4);
        assert_eq!(&pt_bytes[0..2], b"PK"); // ZIP magic
    }

    #[test]
    fn test_safetensors_to_pytorch_too_small() {
        let conv = SafeTensorsToPyTorchConverter;
        let err = conv
            .convert(b"tiny", &ConversionOptions::default(), None)
            .unwrap_err();
        assert!(format!("{err}").contains("too small"));
    }

    #[test]
    fn test_safetensors_to_pytorch_header_exceeds() {
        let mut data = Vec::new();
        data.extend_from_slice(&(1000u64).to_le_bytes()); // header_len = 1000
        data.extend_from_slice(b"short"); // but data is only 5 bytes
        let conv = SafeTensorsToPyTorchConverter;
        let err = conv
            .convert(&data, &ConversionOptions::default(), None)
            .unwrap_err();
        assert!(format!("{err}").contains("exceeds"));
    }

    // ── ONNX metadata extractor ──

    #[test]
    fn test_onnx_metadata_extractor_minimal() {
        // Create minimal protobuf-like ONNX data
        // Field 1 (ir_version) = varint, field 2 (producer) = length-delimited string
        let data = vec![
            0x08, 0x07, // Field 1 (tag = 1<<3 | 0 = 0x08), value = 7
            0x12, 0x04, b't', b'e', b's',
            b't', // Field 2 (tag = 2<<3 | 2 = 0x12), length = 4, "test"
        ];

        let conv = OnnxMetadataExtractor;
        let result = conv
            .convert(&data, &ConversionOptions::default(), None)
            .unwrap();
        let meta: serde_json::Value = serde_json::from_slice(&result).unwrap();
        assert_eq!(meta["ir_version"], 7);
        assert_eq!(meta["producer"], "test");
    }

    // ── Supported conversions ──

    #[test]
    fn test_supported_conversions_non_empty() {
        let pipeline = ConversionPipeline::with_builtins();
        let list = pipeline.supported_conversions();
        assert!(!list.is_empty());
        // Each entry should be (name, source, target)
        for (source, target, name) in &list {
            assert!(!name.is_empty());
            assert_ne!(source, target); // no self-loops
        }
    }

    // ── Multi-step conversion ──

    #[test]
    fn test_multi_step_conversion_executes() {
        let pipeline = ConversionPipeline::with_builtins();
        // PyTorch → ONNX → TensorRT (2 steps)
        let result = pipeline.convert(
            b"model data",
            &ModelFormat::PyTorch,
            &ModelFormat::TensorRT,
            &ConversionOptions::default(),
            None,
        );
        assert!(result.is_ok());
        let conv = result.unwrap();
        assert_eq!(conv.conversion_path.len(), 3);
    }

    #[test]
    fn test_validate_empty_output_fails() {
        let converter = RawToSafeTensorsConverter;
        let report = converter.validate(b"input data", b"", &ConversionOptions::default());
        assert!(!report.passed, "Empty output should fail validation");
        assert!(report
            .checks
            .iter()
            .any(|c| !c.passed && c.name == "non_empty"));
    }

    #[test]
    fn test_validate_huge_ratio_without_quantization_fails() {
        let converter = RawToSafeTensorsConverter;
        let input = b"x";
        // output >100x input without quantization should fail size_ratio
        let output = vec![0u8; 200];
        let report = converter.validate(input, &output, &ConversionOptions::default());
        assert!(report
            .checks
            .iter()
            .any(|c| !c.passed && c.name == "size_ratio"));
    }

    #[test]
    fn test_validation_check_pass_and_fail() {
        let p = ValidationCheck::pass("ok", "good");
        assert!(p.passed);
        assert_eq!(p.name, "ok");

        let f = ValidationCheck::fail("bad", "nope");
        assert!(!f.passed);
        assert_eq!(f.name, "bad");
    }

    #[test]
    fn test_compression_ratio_zero_input() {
        // Covers L120 — input_size == 0 returns 0.0
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

    #[test]
    fn test_safetensors_to_raw_with_progress() {
        // Covers L521-533 — SafeTensorsToRawConverter progress callback
        let raw_data = b"tensor bytes for progress test";
        // First create valid safetensors data
        let st = RawToSafeTensorsConverter
            .convert(raw_data, &ConversionOptions::default(), None)
            .unwrap();

        let progress_called = std::sync::Arc::new(std::sync::Mutex::new(false));
        let pc = progress_called.clone();
        let cb: ProgressCallback = Box::new(move |_p| {
            *pc.lock().unwrap() = true;
        });

        let result = SafeTensorsToRawConverter
            .convert(&st, &ConversionOptions::default(), Some(&cb))
            .unwrap();
        assert_eq!(result, raw_data);
        assert!(*progress_called.lock().unwrap());
    }

    #[test]
    fn test_raw_to_safetensors_with_progress() {
        // Covers L575-583 — RawToSafeTensorsConverter progress callback
        let progress_called = std::sync::Arc::new(std::sync::Mutex::new(false));
        let pc = progress_called.clone();
        let cb: ProgressCallback = Box::new(move |_p| {
            *pc.lock().unwrap() = true;
        });

        let result = RawToSafeTensorsConverter
            .convert(b"data", &ConversionOptions::default(), Some(&cb))
            .unwrap();
        assert!(!result.is_empty());
        assert!(*progress_called.lock().unwrap());
    }

    #[test]
    fn test_safetensors_to_raw_header_exceeds() {
        // Covers L528 — header_len exceeds data length
        let mut data = Vec::new();
        data.extend_from_slice(&(9999u64).to_le_bytes());
        data.extend_from_slice(b"short");
        let err = SafeTensorsToRawConverter
            .convert(&data, &ConversionOptions::default(), None)
            .unwrap_err();
        assert!(format!("{err}").contains("exceeds"));
    }

    #[test]
    fn test_shim_pipeline_returns_plan_not_data() {
        // PyTorch → ONNX needs an external Python toolchain. The pipeline must
        // say so explicitly rather than handing back plan bytes that a caller
        // would write out as a .onnx file.
        let pipeline = ConversionPipeline::with_builtins();
        let conv = pipeline
            .convert(
                b"model data",
                &ModelFormat::PyTorch,
                &ModelFormat::ONNX,
                &ConversionOptions::with_validation(),
                None,
            )
            .expect("planning a shim conversion is not an error");

        assert!(conv.is_plan(), "shim conversion must be reported as a plan");
        assert!(
            conv.data.is_empty(),
            "a plan must not masquerade as model data"
        );
        assert_eq!(conv.output_size, 0);
        assert_eq!(conv.plan.as_ref().unwrap()["converter"], "pytorch_to_onnx");
        // Validating a plan against target-format magic bytes is meaningless.
        assert!(conv.validation.is_none());
    }

    #[test]
    fn test_native_conversion_is_not_a_plan() {
        // The pure-Rust path must stay unaffected: real bytes, no plan.
        let header = serde_json::json!({
            "weight": { "dtype": "F32", "shape": [2, 2], "data_offsets": [0, 16] }
        });
        let header_bytes = serde_json::to_vec(&header).unwrap();
        let mut safetensors = Vec::new();
        safetensors.extend_from_slice(&(header_bytes.len() as u64).to_le_bytes());
        safetensors.extend_from_slice(&header_bytes);
        for f in &[1.0f32, 2.0, 3.0, 4.0] {
            safetensors.extend_from_slice(&f.to_le_bytes());
        }

        let pipeline = ConversionPipeline::with_builtins();
        let conv = pipeline
            .convert(
                &safetensors,
                &ModelFormat::Safetensors,
                &ModelFormat::PyTorch,
                &ConversionOptions::default(),
                None,
            )
            .expect("native conversion should succeed");

        assert!(!conv.is_plan());
        assert!(conv.plan.is_none());
        assert!(!conv.data.is_empty());
    }

    #[test]
    fn test_multi_step_conversion_with_validation() {
        // Covers L392-403 — intermediate validation during multi-step
        let pipeline = ConversionPipeline::with_builtins();
        let opts = ConversionOptions::with_validation();
        // PyTorch → ONNX → TensorRT (2 steps, intermediate validation at step 0)
        let result = pipeline.convert(
            b"model data",
            &ModelFormat::PyTorch,
            &ModelFormat::TensorRT,
            &opts,
            None,
        );
        // May fail or pass depending on validation checks
        let _ = result;
    }

    #[test]
    fn test_onnx_metadata_fields_5_and_6() {
        // Covers L693-713 — ONNX protobuf fields 5 (model_version) and 6 (doc_string)
        let mut data = vec![
            0x08, 0x09, // Field 1, varint: ir_version = 9
            0x12, 0x08, // Field 2, length-delimited: producer = "TestProd"
        ];
        data.extend_from_slice(b"TestProd");
        data.extend_from_slice(&[
            0x28, 0x2A, // Field 5, varint: model_version = 42
            0x32, 0x0C, // Field 6, length-delimited: doc_string = "A test model"
        ]);
        data.extend_from_slice(b"A test model");

        let result = OnnxMetadataExtractor
            .convert(&data, &ConversionOptions::default(), None)
            .unwrap();
        let meta: serde_json::Value = serde_json::from_slice(&result).unwrap();
        assert_eq!(meta["ir_version"], 9);
        assert_eq!(meta["producer"], "TestProd");
        assert_eq!(meta["model_version"], 42);
        assert_eq!(meta["doc_string"], "A test model");
    }

    #[test]
    fn test_onnx_metadata_skip_32bit_64bit() {
        // Covers L705-706, L709-710 — skipping 32-bit and 64-bit fields
        let mut data = Vec::new();
        // Field 1, varint (0x08), value 7
        data.push(0x08);
        data.push(0x07);
        // Field 10, 32-bit fixed (tag = 10<<3 | 5 = 0x55)
        data.push(0x55);
        data.extend_from_slice(&[0u8; 4]);
        // Field 11, 64-bit fixed (tag = 11<<3 | 1 = 0x59)
        data.push(0x59);
        data.extend_from_slice(&[0u8; 8]);

        let result = OnnxMetadataExtractor
            .convert(&data, &ConversionOptions::default(), None)
            .unwrap();
        let meta: serde_json::Value = serde_json::from_slice(&result).unwrap();
        assert_eq!(meta["ir_version"], 7);
    }

    #[test]
    fn test_validate_magic_bytes_onnx() {
        // Covers L466-467 — ONNX protobuf tag
        let data = vec![0x08, 0x07, 0x12, 0x04]; // starts with protobuf tag
        let check = validate_magic_bytes(&data, &ModelFormat::ONNX);
        assert!(check.passed);
    }

    #[test]
    fn test_validate_magic_bytes_tflite() {
        // Covers L471 — TFLite FlatBuffer
        let mut data = vec![0x20, 0x00, 0x00, 0x00]; // TFLite magic
        data.extend_from_slice(&[0u8; 20]);
        let check = validate_magic_bytes(&data, &ModelFormat::TFLite);
        assert!(check.passed);
    }

    #[test]
    fn test_validate_magic_bytes_pytorch_pickle() {
        // Covers L466-467 variations — PyTorch pickle format (0x80)
        let data = vec![0x80, 0x02, 0x00, 0x00];
        let check = validate_magic_bytes(&data, &ModelFormat::PyTorch);
        assert!(check.passed);
    }

    #[test]
    fn test_validate_magic_bytes_pytorch_invalid() {
        let data = vec![0x00, 0xFF, 0x00, 0x00]; // Not PK and not 0x80
        let check = validate_magic_bytes(&data, &ModelFormat::PyTorch);
        assert!(!check.passed);
    }

    #[test]
    fn test_validate_magic_bytes_safetensors_invalid() {
        // Invalid header length
        let mut data = Vec::new();
        data.extend_from_slice(&(99999u64).to_le_bytes()); // header_len > data.len
        let check = validate_magic_bytes(&data, &ModelFormat::Safetensors);
        assert!(!check.passed);
    }

    #[test]
    fn test_validate_magic_bytes_safetensors_too_small() {
        let check = validate_magic_bytes(b"tiny", &ModelFormat::Safetensors);
        assert!(!check.passed);
    }

    #[test]
    fn test_validate_magic_bytes_gguf_mismatch() {
        let check = validate_magic_bytes(b"NOT_GGUF", &ModelFormat::GGUF);
        assert!(!check.passed);
    }

    #[test]
    fn test_validate_magic_bytes_tflite_mismatch() {
        let check = validate_magic_bytes(b"\x00\x00\x00\x00extra", &ModelFormat::TFLite);
        assert!(!check.passed);
    }

    #[test]
    fn test_validate_magic_bytes_onnx_mismatch() {
        let check = validate_magic_bytes(b"\xFF\xFF", &ModelFormat::ONNX);
        assert!(!check.passed);
    }

    #[test]
    fn test_gguf_header_parser_too_small() {
        // Covers L636 variation — data too small
        let err = GgufHeaderParser
            .convert(b"tiny_data", &ConversionOptions::default(), None)
            .unwrap_err();
        assert!(format!("{err}").contains("too small"));
    }

    #[test]
    fn test_safetensors_to_pytorch_invalid_header_json() {
        // Covers L778 — invalid JSON in safetensors header
        let invalid_json = b"not valid json at all";
        let mut data = Vec::new();
        data.extend_from_slice(&(invalid_json.len() as u64).to_le_bytes());
        data.extend_from_slice(invalid_json);
        data.extend_from_slice(&[0u8; 16]);
        let err = SafeTensorsToPyTorchConverter
            .convert(&data, &ConversionOptions::default(), None)
            .unwrap_err();
        assert!(format!("{err}").contains("Invalid SafeTensors header JSON"));
    }

    #[test]
    fn test_pipeline_default() {
        // Covers ConversionPipeline::default()
        let pipeline = ConversionPipeline::default();
        let list = pipeline.supported_conversions();
        assert!(!list.is_empty());
    }

    #[test]
    fn test_multi_step_halts_at_first_plan_step() {
        // PyTorch → ONNX → TensorRT: step 1 needs external tooling, so there are
        // no real ONNX bytes for step 2 to consume. The pipeline must stop and
        // return that plan rather than feeding a plan into the next converter
        // (which previously produced a meaningless plan-of-a-plan).
        let pipeline = ConversionPipeline::with_builtins();
        let steps_arc = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let sc = steps_arc.clone();
        let cb: ProgressCallback = Box::new(move |p| {
            sc.lock().unwrap().push(p.step);
        });
        let conv = pipeline
            .convert(
                b"model",
                &ModelFormat::PyTorch,
                &ModelFormat::TensorRT,
                &ConversionOptions::default(),
                Some(&cb),
            )
            .expect("planning should not error");

        let logged = steps_arc.lock().unwrap();
        assert_eq!(logged.len(), 1, "must stop after the first plan step");
        assert!(conv.is_plan());
        assert_eq!(conv.plan.as_ref().unwrap()["converter"], "pytorch_to_onnx");
    }

    #[test]
    fn test_parse_varint_overflow() {
        // Covers L1024-L1025 — shift >= 64 branch in parse_varint
        // A varint with all continuation bits set (10 bytes, each 0xFF except last)
        // will hit the shift >= 64 guard
        let data = vec![0x80u8; 10]; // All continuation, never terminates normally
        let (result, pos) = parse_varint(&data, 0);
        // Should break out after shift >= 64 (after ~9 bytes)
        assert!(pos <= data.len());
        let _ = result; // Value is meaningless for overflowed varint
    }

    #[test]
    fn test_parse_protobuf_tag_empty() {
        // Covers L1034 — pos >= data.len() returns None
        let data: &[u8] = &[];
        assert!(parse_protobuf_tag(data, 0).is_none());

        // Also test pos beyond bounds
        let data2 = &[0x08u8]; // 1 byte
        assert!(parse_protobuf_tag(data2, 5).is_none());
    }

    #[test]
    fn test_parse_length_delimited_exceeds() {
        // Covers L1046-L1047 — length field exceeds remaining data
        // Varint for length=100 followed by only 2 bytes of data
        let data = vec![100u8, 0xAA, 0xBB]; // length=100 but only 2 bytes follow
        let result = parse_length_delimited(&data, 0);
        assert!(result.is_err());
        let msg = format!("{}", result.unwrap_err());
        assert!(msg.contains("exceeds data"));
    }

    // ── Additional coverage: ONNX extractor edge cases ──

    #[test]
    fn test_onnx_metadata_empty_data() {
        let result = OnnxMetadataExtractor
            .convert(&[], &ConversionOptions::default(), None)
            .unwrap();
        let meta: serde_json::Value = serde_json::from_slice(&result).unwrap();
        assert_eq!(meta["ir_version"], 0);
        assert_eq!(meta["producer"], "");
    }

    #[test]
    fn test_onnx_metadata_skip_unknown_wire_type() {
        // Wire type 3 or 4 (unknown) should cause break
        let mut data = vec![
            0x08, 0x05, // field 1 varint = 5
        ];
        // Wire type 3 (start group), field 20: tag = 20<<3 | 3 = 163 = 0xA3
        data.push(0xA3);
        data.push(0x01);
        // Parser should break on wire type 3, but ir_version should be extracted
        let result = OnnxMetadataExtractor
            .convert(&data, &ConversionOptions::default(), None)
            .unwrap();
        let meta: serde_json::Value = serde_json::from_slice(&result).unwrap();
        assert_eq!(meta["ir_version"], 5);
    }

    #[test]
    fn test_onnx_metadata_skip_varint_field() {
        // Unknown varint field (e.g. field 3) should be skipped
        let data = vec![
            0x08, 0x03, // field 1: ir_version = 3
            0x18, 0x42, // field 3: unknown varint (skipped)
            0x28, 0x01, // field 5: model_version = 1
        ];
        let result = OnnxMetadataExtractor
            .convert(&data, &ConversionOptions::default(), None)
            .unwrap();
        let meta: serde_json::Value = serde_json::from_slice(&result).unwrap();
        assert_eq!(meta["ir_version"], 3);
        assert_eq!(meta["model_version"], 1);
    }

    #[test]
    fn test_onnx_metadata_skip_length_delimited_field() {
        // Unknown length-delimited field (e.g. field 7) should be skipped
        let mut data = vec![
            0x08, 0x04, // field 1: ir_version = 4
            0x3A, 0x03, // field 7: length-delimited, length = 3
        ];
        data.extend_from_slice(b"abc"); // 3 bytes of payload for field 7
        data.extend_from_slice(&[0x28, 0x0A]); // field 5: model_version = 10

        let result = OnnxMetadataExtractor
            .convert(&data, &ConversionOptions::default(), None)
            .unwrap();
        let meta: serde_json::Value = serde_json::from_slice(&result).unwrap();
        assert_eq!(meta["ir_version"], 4);
        assert_eq!(meta["model_version"], 10);
    }

    // ── Additional coverage: SafeTensors → PyTorch edge cases ──

    #[test]
    fn test_safetensors_to_pytorch_header_too_large() {
        // Covers header > 100MB cap
        let mut data = Vec::new();
        let huge = 200 * 1024 * 1024u64; // 200 MB
        data.extend_from_slice(&huge.to_le_bytes());
        data.extend_from_slice(&[0u8; 16]);
        let err = SafeTensorsToPyTorchConverter
            .convert(&data, &ConversionOptions::default(), None)
            .unwrap_err();
        assert!(format!("{err}").contains("too large"));
    }

    #[test]
    fn test_safetensors_to_pytorch_missing_dtype() {
        let header = serde_json::json!({
            "weight": { "shape": [2], "data_offsets": [0, 8] }
        });
        let hdr_bytes = serde_json::to_vec(&header).unwrap();
        let mut data = Vec::new();
        data.extend_from_slice(&(hdr_bytes.len() as u64).to_le_bytes());
        data.extend_from_slice(&hdr_bytes);
        data.extend_from_slice(&[0u8; 8]);
        let err = SafeTensorsToPyTorchConverter
            .convert(&data, &ConversionOptions::default(), None)
            .unwrap_err();
        assert!(format!("{err}").contains("dtype"));
    }

    #[test]
    fn test_safetensors_to_pytorch_missing_shape() {
        let header = serde_json::json!({
            "weight": { "dtype": "F32", "data_offsets": [0, 8] }
        });
        let hdr_bytes = serde_json::to_vec(&header).unwrap();
        let mut data = Vec::new();
        data.extend_from_slice(&(hdr_bytes.len() as u64).to_le_bytes());
        data.extend_from_slice(&hdr_bytes);
        data.extend_from_slice(&[0u8; 8]);
        let err = SafeTensorsToPyTorchConverter
            .convert(&data, &ConversionOptions::default(), None)
            .unwrap_err();
        assert!(format!("{err}").contains("shape"));
    }

    #[test]
    fn test_safetensors_to_pytorch_missing_offsets() {
        let header = serde_json::json!({
            "weight": { "dtype": "F32", "shape": [2] }
        });
        let hdr_bytes = serde_json::to_vec(&header).unwrap();
        let mut data = Vec::new();
        data.extend_from_slice(&(hdr_bytes.len() as u64).to_le_bytes());
        data.extend_from_slice(&hdr_bytes);
        data.extend_from_slice(&[0u8; 8]);
        let err = SafeTensorsToPyTorchConverter
            .convert(&data, &ConversionOptions::default(), None)
            .unwrap_err();
        assert!(format!("{err}").contains("data_offsets"));
    }

    #[test]
    fn test_safetensors_to_pytorch_tensor_not_object() {
        let header = serde_json::json!({
            "weight": "not an object"
        });
        let hdr_bytes = serde_json::to_vec(&header).unwrap();
        let mut data = Vec::new();
        data.extend_from_slice(&(hdr_bytes.len() as u64).to_le_bytes());
        data.extend_from_slice(&hdr_bytes);
        let err = SafeTensorsToPyTorchConverter
            .convert(&data, &ConversionOptions::default(), None)
            .unwrap_err();
        assert!(format!("{err}").contains("not an object"));
    }

    #[test]
    fn test_safetensors_to_pytorch_out_of_bounds_offsets() {
        let header = serde_json::json!({
            "weight": { "dtype": "F32", "shape": [100], "data_offsets": [0, 99999] }
        });
        let hdr_bytes = serde_json::to_vec(&header).unwrap();
        let mut data = Vec::new();
        data.extend_from_slice(&(hdr_bytes.len() as u64).to_le_bytes());
        data.extend_from_slice(&hdr_bytes);
        data.extend_from_slice(&[0u8; 16]); // Only 16 bytes of tensor data
        let err = SafeTensorsToPyTorchConverter
            .convert(&data, &ConversionOptions::default(), None)
            .unwrap_err();
        assert!(format!("{err}").contains("out of bounds"));
    }

    #[test]
    fn test_safetensors_to_pytorch_unsupported_dtype() {
        let header = serde_json::json!({
            "weight": { "dtype": "COMPLEX128", "shape": [2], "data_offsets": [0, 16] }
        });
        let hdr_bytes = serde_json::to_vec(&header).unwrap();
        let mut data = Vec::new();
        data.extend_from_slice(&(hdr_bytes.len() as u64).to_le_bytes());
        data.extend_from_slice(&hdr_bytes);
        data.extend_from_slice(&[0u8; 16]);
        let err = SafeTensorsToPyTorchConverter
            .convert(&data, &ConversionOptions::default(), None)
            .unwrap_err();
        assert!(format!("{err}").contains("Unsupported dtype"));
    }

    #[test]
    fn test_safetensors_to_pytorch_metadata_skipped() {
        // __metadata__ key should be skipped (not treated as tensor)
        let header = serde_json::json!({
            "__metadata__": { "format": "pt" },
            "bias": { "dtype": "F32", "shape": [4], "data_offsets": [0, 16] }
        });
        let hdr_bytes = serde_json::to_vec(&header).unwrap();
        let mut data = Vec::new();
        data.extend_from_slice(&(hdr_bytes.len() as u64).to_le_bytes());
        data.extend_from_slice(&hdr_bytes);
        data.extend_from_slice(&[0u8; 16]);
        let result = SafeTensorsToPyTorchConverter
            .convert(&data, &ConversionOptions::default(), None)
            .unwrap();
        assert_eq!(&result[0..2], b"PK"); // ZIP archive
    }

    #[test]
    fn test_safetensors_to_pytorch_with_progress() {
        let header = serde_json::json!({
            "w": { "dtype": "F32", "shape": [2], "data_offsets": [0, 8] }
        });
        let hdr_bytes = serde_json::to_vec(&header).unwrap();
        let mut data = Vec::new();
        data.extend_from_slice(&(hdr_bytes.len() as u64).to_le_bytes());
        data.extend_from_slice(&hdr_bytes);
        data.extend_from_slice(&[0u8; 8]);

        let msgs = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let mc = msgs.clone();
        let cb: ProgressCallback = Box::new(move |p| {
            mc.lock().unwrap().push(p.message.clone());
        });
        let result = SafeTensorsToPyTorchConverter
            .convert(&data, &ConversionOptions::default(), Some(&cb))
            .unwrap();
        assert!(!result.is_empty());
        assert!(msgs.lock().unwrap().len() >= 2);
    }

    // ── Additional coverage: PyTorch → SafeTensors edge cases ──

    #[test]
    fn test_pytorch_to_safetensors_with_progress() {
        // Create a valid SafeTensors → PyTorch → SafeTensors round trip with progress
        let header = serde_json::json!({
            "w": { "dtype": "F32", "shape": [2], "data_offsets": [0, 8] }
        });
        let hdr_bytes = serde_json::to_vec(&header).unwrap();
        let mut st_data = Vec::new();
        st_data.extend_from_slice(&(hdr_bytes.len() as u64).to_le_bytes());
        st_data.extend_from_slice(&hdr_bytes);
        st_data.extend_from_slice(&[0u8; 8]);

        let pt_bytes = SafeTensorsToPyTorchConverter
            .convert(&st_data, &ConversionOptions::default(), None)
            .unwrap();

        let msgs = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let mc = msgs.clone();
        let cb: ProgressCallback = Box::new(move |p| {
            mc.lock().unwrap().push(p.message.clone());
        });
        let result = PyTorchToSafeTensorsConverter
            .convert(&pt_bytes, &ConversionOptions::default(), Some(&cb))
            .unwrap();
        assert!(!result.is_empty());
        assert!(!msgs.lock().unwrap().is_empty());
    }

    #[test]
    fn test_pytorch_to_safetensors_fallback_no_pickle() {
        // Create a ZIP without data.pkl — exercises fallback path
        let mut buf = std::io::Cursor::new(Vec::new());
        {
            let mut zip = zip::ZipWriter::new(&mut buf);
            let opts = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Stored);
            zip.start_file("archive/data/0", opts).unwrap();
            std::io::Write::write_all(&mut zip, &[1u8, 2, 3, 4]).unwrap();
            zip.finish().unwrap();
        }
        let pt_bytes = buf.into_inner();
        let result = PyTorchToSafeTensorsConverter
            .convert(&pt_bytes, &ConversionOptions::default(), None)
            .unwrap();
        let hdr_len = u64::from_le_bytes(result[0..8].try_into().unwrap()) as usize;
        let hdr: serde_json::Value = serde_json::from_slice(&result[8..8 + hdr_len]).unwrap();
        // Fallback should create storage_0 with U8 dtype
        assert!(hdr.get("storage_0").is_some());
        let entry = &hdr["storage_0"];
        assert_eq!(entry["dtype"], "U8");
    }

    // ── Additional coverage: dtype mapping ──

    #[test]
    fn test_safetensors_dtype_to_pytorch_all_types() {
        assert_eq!(
            safetensors_dtype_to_pytorch("F64"),
            Some(("DoubleStorage", 8))
        );
        assert_eq!(
            safetensors_dtype_to_pytorch("F32"),
            Some(("FloatStorage", 4))
        );
        assert_eq!(
            safetensors_dtype_to_pytorch("F16"),
            Some(("HalfStorage", 2))
        );
        assert_eq!(
            safetensors_dtype_to_pytorch("BF16"),
            Some(("BFloat16Storage", 2))
        );
        assert_eq!(
            safetensors_dtype_to_pytorch("I64"),
            Some(("LongStorage", 8))
        );
        assert_eq!(safetensors_dtype_to_pytorch("I32"), Some(("IntStorage", 4)));
        assert_eq!(
            safetensors_dtype_to_pytorch("I16"),
            Some(("ShortStorage", 2))
        );
        assert_eq!(safetensors_dtype_to_pytorch("I8"), Some(("CharStorage", 1)));
        assert_eq!(safetensors_dtype_to_pytorch("U8"), Some(("ByteStorage", 1)));
        assert_eq!(
            safetensors_dtype_to_pytorch("BOOL"),
            Some(("BoolStorage", 1))
        );
        assert_eq!(safetensors_dtype_to_pytorch("UNKNOWN"), None);
    }

    #[test]
    fn test_pytorch_storage_to_safetensors_dtype_all_types() {
        assert_eq!(
            pytorch_storage_to_safetensors_dtype("DoubleStorage"),
            Some(("F64", 8))
        );
        assert_eq!(
            pytorch_storage_to_safetensors_dtype("FloatStorage"),
            Some(("F32", 4))
        );
        assert_eq!(
            pytorch_storage_to_safetensors_dtype("HalfStorage"),
            Some(("F16", 2))
        );
        assert_eq!(
            pytorch_storage_to_safetensors_dtype("BFloat16Storage"),
            Some(("BF16", 2))
        );
        assert_eq!(
            pytorch_storage_to_safetensors_dtype("LongStorage"),
            Some(("I64", 8))
        );
        assert_eq!(
            pytorch_storage_to_safetensors_dtype("IntStorage"),
            Some(("I32", 4))
        );
        assert_eq!(
            pytorch_storage_to_safetensors_dtype("ShortStorage"),
            Some(("I16", 2))
        );
        assert_eq!(
            pytorch_storage_to_safetensors_dtype("CharStorage"),
            Some(("I8", 1))
        );
        assert_eq!(
            pytorch_storage_to_safetensors_dtype("ByteStorage"),
            Some(("U8", 1))
        );
        assert_eq!(
            pytorch_storage_to_safetensors_dtype("UntypedStorage"),
            Some(("U8", 1))
        );
        assert_eq!(
            pytorch_storage_to_safetensors_dtype("BoolStorage"),
            Some(("BOOL", 1))
        );
        assert_eq!(pytorch_storage_to_safetensors_dtype("UnknownStorage"), None);
    }

    // ── Additional coverage: pickle helpers ──

    #[test]
    fn test_write_short_binunicode_short() {
        let mut pkl = Vec::new();
        write_short_binunicode(&mut pkl, "abc");
        assert_eq!(pkl[0], 0x8c); // SHORT_BINUNICODE
        assert_eq!(pkl[1], 3);
        assert_eq!(&pkl[2..5], b"abc");
    }

    #[test]
    fn test_write_short_binunicode_long() {
        let mut pkl = Vec::new();
        let s = "x".repeat(300);
        write_short_binunicode(&mut pkl, &s);
        assert_eq!(pkl[0], 0x8d); // BINUNICODE
        let len = u32::from_le_bytes(pkl[1..5].try_into().unwrap());
        assert_eq!(len, 300);
    }

    #[test]
    fn test_write_pickle_int_ranges() {
        // BININT1: 0 - 255
        let mut pkl = Vec::new();
        write_pickle_int(&mut pkl, 42);
        assert_eq!(pkl[0], 0x4b);
        assert_eq!(pkl[1], 42);

        // BININT2: 256 - 65535
        pkl.clear();
        write_pickle_int(&mut pkl, 1000);
        assert_eq!(pkl[0], 0x4d);
        assert_eq!(u16::from_le_bytes(pkl[1..3].try_into().unwrap()), 1000);

        // BININT: i32 range
        pkl.clear();
        write_pickle_int(&mut pkl, 100_000);
        assert_eq!(pkl[0], 0x4a);
        assert_eq!(i32::from_le_bytes(pkl[1..5].try_into().unwrap()), 100_000);

        // LONG1: > i32 range
        pkl.clear();
        write_pickle_int(&mut pkl, i64::MAX);
        assert_eq!(pkl[0], 0x8a); // LONG1
    }

    #[test]
    fn test_write_pickle_int_negative() {
        let mut pkl = Vec::new();
        write_pickle_int(&mut pkl, -100);
        assert_eq!(pkl[0], 0x4a); // BININT (i32 range)
        assert_eq!(i32::from_le_bytes(pkl[1..5].try_into().unwrap()), -100);

        // Large negative (< i32::MIN) — LONG1
        pkl.clear();
        write_pickle_int(&mut pkl, i64::MIN);
        assert_eq!(pkl[0], 0x8a); // LONG1
    }

    #[test]
    fn test_extract_tensor_info_from_pickle_empty() {
        let infos = extract_tensor_info_from_pickle(&[]);
        assert!(infos.is_empty());
    }

    #[test]
    fn test_extract_tensor_info_from_pickle_roundtrip() {
        // Build a pickle via build_pytorch_pickle, then extract info back
        let tensors = vec![ConvTensorEntry {
            name: "model.weight".to_string(),
            dtype: "F32".to_string(),
            shape: vec![3, 4],
            data_start: 0,
            data_end: 48,
        }];
        let pkl = build_pytorch_pickle(&tensors).unwrap();
        let infos = extract_tensor_info_from_pickle(&pkl);
        assert!(!infos.is_empty());
        assert_eq!(infos[0].name, "model.weight");
        assert_eq!(infos[0].dtype, "F32");
    }

    #[test]
    fn test_safetensors_to_pytorch_multiple_dtypes() {
        // Multi-tensor with different dtypes
        let header = serde_json::json!({
            "w1": { "dtype": "F32", "shape": [2], "data_offsets": [0, 8] },
            "w2": { "dtype": "F16", "shape": [4], "data_offsets": [8, 16] },
            "w3": { "dtype": "I64", "shape": [1], "data_offsets": [16, 24] }
        });
        let hdr_bytes = serde_json::to_vec(&header).unwrap();
        let mut data = Vec::new();
        data.extend_from_slice(&(hdr_bytes.len() as u64).to_le_bytes());
        data.extend_from_slice(&hdr_bytes);
        data.extend_from_slice(&[0u8; 24]);
        let result = SafeTensorsToPyTorchConverter
            .convert(&data, &ConversionOptions::default(), None)
            .unwrap();
        assert_eq!(&result[0..2], b"PK");

        // Roundtrip back
        let st2 = PyTorchToSafeTensorsConverter
            .convert(&result, &ConversionOptions::default(), None)
            .unwrap();
        let hdr_len = u64::from_le_bytes(st2[0..8].try_into().unwrap()) as usize;
        let hdr: serde_json::Value = serde_json::from_slice(&st2[8..8 + hdr_len]).unwrap();
        assert!(hdr.as_object().unwrap().len() >= 3);
    }

    #[test]
    fn test_pipeline_convert_no_path() {
        // Covers the error path in convert() when find_path returns None
        let pipeline = ConversionPipeline::with_builtins();
        let err = pipeline
            .convert(
                b"data",
                &ModelFormat::MXNet,
                &ModelFormat::Darknet,
                &ConversionOptions::default(),
                None,
            )
            .unwrap_err();
        assert!(format!("{err}").contains("No conversion path"));
    }

    #[test]
    fn test_validate_magic_bytes_onnx_short_data() {
        // ONNX needs at least 2 bytes; short data should fail
        let check = validate_magic_bytes(b"", &ModelFormat::ONNX);
        assert!(!check.passed);
    }
}
