//! Native Python bindings via PyO3
//!
//! Provides zero-copy access to the Rust vault, crypto, format detection,
//! and model card APIs — replacing the legacy CLI-wrapper Python package.

use pyo3::exceptions::{PyIOError, PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict};
use std::collections::HashMap;

use crate::config::VaultConfig;
use crate::error::VaultError;
use crate::formats::{ModelFormat, ModelMetadata};
use crate::model_card::{Evaluation, IntendedUse, Metric, ModelCard, ModelDetails, TrainingData};
use crate::vault::{ModelStream, Vault};
use crate::version::ModelVersion;

// ── helpers ──────────────────────────────────────────────────────────────────

/// Convert `VaultError` → Python exception.
fn to_py_err(e: VaultError) -> PyErr {
    match &e {
        VaultError::AuthenticationFailed => PyRuntimeError::new_err(e.to_string()),
        VaultError::SecurityViolation(_) => PyRuntimeError::new_err(e.to_string()),
        VaultError::IoError(_) => PyIOError::new_err(e.to_string()),
        VaultError::ModelNotFound(_) => PyValueError::new_err(e.to_string()),
        VaultError::VersionNotFound(_, _) => PyValueError::new_err(e.to_string()),
        VaultError::NotFound(_) => PyValueError::new_err(e.to_string()),
        VaultError::UnsupportedFormat(_) => PyValueError::new_err(e.to_string()),
        _ => PyRuntimeError::new_err(e.to_string()),
    }
}

fn parse_format(s: &str) -> PyResult<ModelFormat> {
    match s.to_lowercase().as_str() {
        "safetensors" => Ok(ModelFormat::Safetensors),
        "gguf" => Ok(ModelFormat::GGUF),
        "pytorch" | "pt" | "pth" => Ok(ModelFormat::PyTorch),
        "tensorrt" | "plan" => Ok(ModelFormat::TensorRT),
        "onnx" => Ok(ModelFormat::ONNX),
        "mlx" => Ok(ModelFormat::MLX),
        "coreml" | "mlmodel" => Ok(ModelFormat::CoreML),
        "torchscript" => Ok(ModelFormat::TorchScript),
        "tflite" => Ok(ModelFormat::TFLite),
        "tensorflow" | "tf" | "pb" => Ok(ModelFormat::TensorFlow),
        "keras" | "h5" => Ok(ModelFormat::Keras),
        "openvino" => Ok(ModelFormat::OpenVINO),
        "tvm" => Ok(ModelFormat::TVM),
        "ncnn" => Ok(ModelFormat::NCNN),
        "mnn" => Ok(ModelFormat::MNN),
        "rknn" => Ok(ModelFormat::RKNN),
        "caffe" => Ok(ModelFormat::Caffe),
        "mxnet" => Ok(ModelFormat::MXNet),
        "darknet" => Ok(ModelFormat::Darknet),
        "hdf5" => Ok(ModelFormat::HDF5),
        "pickle" | "pkl" => Ok(ModelFormat::Pickle),
        "numpy" | "npy" | "npz" => Ok(ModelFormat::NumPy),
        other => Ok(ModelFormat::Custom(other.to_string())),
    }
}

// ── PyModelFormat ────────────────────────────────────────────────────────────

/// AI model format identifier.
///
/// Use `detect("model.safetensors")` to auto-detect from filename.
// Opt in to the FromPyObject derive: before pyo3 0.29 it was automatic for
// Clone pyclasses, and callers extract these types by value.
#[pyclass(name = "ModelFormat", from_py_object)]
#[derive(Clone)]
struct PyModelFormat {
    inner: ModelFormat,
}

#[pymethods]
impl PyModelFormat {
    // -- constructors --------------------------------------------------------

    #[new]
    #[pyo3(signature = (name))]
    fn new(name: &str) -> PyResult<Self> {
        Ok(Self {
            inner: parse_format(name)?,
        })
    }

    /// Detect format from a filename/path.
    #[staticmethod]
    fn detect(filename: &str) -> PyResult<Self> {
        let ext = filename
            .rsplit('.')
            .next()
            .ok_or_else(|| PyValueError::new_err("No file extension"))?;
        Ok(Self {
            inner: ModelFormat::from_extension(ext),
        })
    }

    // -- properties ----------------------------------------------------------

    /// Human-readable format name.
    #[getter]
    fn name(&self) -> &str {
        self.inner.name()
    }

    /// Canonical file extension.
    #[getter]
    fn extension(&self) -> &str {
        self.inner.extension()
    }

    fn __repr__(&self) -> String {
        format!("ModelFormat('{}')", self.inner.name())
    }

    fn __str__(&self) -> String {
        self.inner.name().to_string()
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

// ── PyModelMetadata ──────────────────────────────────────────────────────────

/// Metadata attached to a stored model.
// Opt in to the FromPyObject derive: before pyo3 0.29 it was automatic for
// Clone pyclasses, and callers extract these types by value.
#[pyclass(name = "ModelMetadata", from_py_object)]
#[derive(Clone)]
struct PyModelMetadata {
    inner: ModelMetadata,
}

#[pymethods]
impl PyModelMetadata {
    #[new]
    #[pyo3(signature = (name, format, *, description=None, framework=None, task=None, architecture=None, parameters=None))]
    fn new(
        name: &str,
        format: &str,
        description: Option<String>,
        framework: Option<String>,
        task: Option<String>,
        architecture: Option<String>,
        parameters: Option<u64>,
    ) -> PyResult<Self> {
        let fmt = parse_format(format)?;
        let mut md = ModelMetadata::new(name.to_string(), fmt);
        if let Some(d) = description {
            md = md.with_description(d);
        }
        if let Some(f) = framework {
            md = md.with_framework(f);
        }
        if let Some(t) = task {
            md = md.with_task(t);
        }
        if let Some(a) = architecture {
            md = md.with_architecture(a);
        }
        if let Some(p) = parameters {
            md = md.with_parameters(p);
        }
        Ok(Self { inner: md })
    }

    #[getter]
    fn name(&self) -> &str {
        &self.inner.name
    }

    #[getter]
    fn format(&self) -> PyModelFormat {
        PyModelFormat {
            inner: self.inner.format.clone(),
        }
    }

    #[getter]
    fn description(&self) -> Option<&str> {
        self.inner.description.as_deref()
    }

    #[getter]
    fn framework(&self) -> Option<&str> {
        self.inner.framework.as_deref()
    }

    #[getter]
    fn task(&self) -> Option<&str> {
        self.inner.task.as_deref()
    }

    #[getter]
    fn architecture(&self) -> Option<&str> {
        self.inner.architecture.as_deref()
    }

    #[getter]
    fn parameters(&self) -> Option<u64> {
        self.inner.parameters
    }

    /// Add a custom key/value field.
    fn add_custom_field(&mut self, key: String, value: String) {
        self.inner.custom_fields.insert(key, value);
    }

    fn __repr__(&self) -> String {
        format!(
            "ModelMetadata(name='{}', format='{}')",
            self.inner.name,
            self.inner.format.name()
        )
    }
}

// ── PyModelVersion ───────────────────────────────────────────────────────────

/// Read-only snapshot of a model version.
// Opt in to the FromPyObject derive: before pyo3 0.29 it was automatic for
// Clone pyclasses, and callers extract these types by value.
#[pyclass(name = "ModelVersion", from_py_object)]
#[derive(Clone)]
struct PyModelVersion {
    inner: ModelVersion,
}

#[pymethods]
impl PyModelVersion {
    #[getter]
    fn version(&self) -> u32 {
        self.inner.version
    }

    #[getter]
    fn checkpoint_id(&self) -> &str {
        &self.inner.checkpoint_id
    }

    #[getter]
    fn timestamp(&self) -> String {
        self.inner.timestamp.to_rfc3339()
    }

    #[getter]
    fn parent_version(&self) -> Option<u32> {
        self.inner.parent_version
    }

    #[getter]
    fn format(&self) -> &str {
        &self.inner.format
    }

    #[getter]
    fn size_bytes(&self) -> u64 {
        self.inner.size_bytes
    }

    #[getter]
    fn compressed_size_bytes(&self) -> u64 {
        self.inner.compressed_size_bytes
    }

    #[getter]
    fn checksum_sha256(&self) -> &str {
        &self.inner.checksum_sha256
    }

    #[getter]
    fn metadata(&self) -> HashMap<String, String> {
        self.inner.metadata.clone()
    }

    fn __repr__(&self) -> String {
        format!(
            "ModelVersion(version={}, format='{}', size={})",
            self.inner.version, self.inner.format, self.inner.size_bytes
        )
    }
}

// ── PyVaultConfig ────────────────────────────────────────────────────────────

/// Vault configuration — XDG-compliant paths and crypto settings.
// Opt in to the FromPyObject derive: before pyo3 0.29 it was automatic for
// Clone pyclasses, and callers extract these types by value.
#[pyclass(name = "VaultConfig", from_py_object)]
#[derive(Clone)]
struct PyVaultConfig {
    inner: VaultConfig,
}

#[pymethods]
impl PyVaultConfig {
    /// Create a new VaultConfig.
    ///
    /// If `vault_dir` is given it overrides the default XDG location.
    #[new]
    #[pyo3(signature = (vault_dir=None))]
    fn new(vault_dir: Option<String>) -> PyResult<Self> {
        let cfg = if let Some(dir) = vault_dir {
            let path = std::path::PathBuf::from(&dir)
                .canonicalize()
                .map_err(|e| PyIOError::new_err(format!("Invalid vault directory: {e}")))?;
            let dirs = crate::config::DirectoryPaths {
                config_dir: path.join("config"),
                data_dir: path.clone(),
                cache_dir: path.join("cache"),
                vault_dir: path.join("vaults"),
                log_dir: path.join("logs"),
                backends_dir: path.join("backends"),
                utilities_dir: path.join("utilities"),
                databases_dir: path.join("databases"),
            };
            VaultConfig::with_dirs(dirs).map_err(to_py_err)?
        } else {
            VaultConfig::new().map_err(to_py_err)?
        };
        Ok(Self { inner: cfg })
    }

    #[getter]
    fn vault_path(&self) -> String {
        self.inner
            .get_vault_path(None)
            .to_string_lossy()
            .to_string()
    }

    fn __repr__(&self) -> String {
        format!(
            "VaultConfig(vault_path='{}')",
            self.inner.get_vault_path(None).display()
        )
    }
}

// ── PyVault ──────────────────────────────────────────────────────────────────

/// The main vault — create, unlock, store, retrieve, and manage AI models.
///
/// Example::
///
///     from ironvault import Vault, VaultConfig, ModelMetadata
///
///     vault = Vault()
///     vault.unlock(b"my-passphrase")
///     ver = vault.store_model("my-model", model_bytes,
///               ModelMetadata("my-model", "safetensors"))
///     data = vault.get_model("my-model")
///     vault.lock()
#[pyclass(name = "Vault")]
struct PyVault {
    inner: Vault,
}

#[pymethods]
impl PyVault {
    /// Create or open a vault.
    #[new]
    #[pyo3(signature = (config=None))]
    fn new(config: Option<&PyVaultConfig>) -> PyResult<Self> {
        let cfg = config.map(|c| c.inner.clone());
        let vault = Vault::new(cfg).map_err(to_py_err)?;
        Ok(Self { inner: vault })
    }

    /// Unlock the vault with a passphrase (`bytes`).
    fn unlock(&mut self, passphrase: &[u8]) -> PyResult<()> {
        self.inner.unlock(passphrase.to_vec()).map_err(to_py_err)
    }

    /// Lock the vault (zeroizes keys in memory).
    fn lock(&mut self) {
        self.inner.lock();
    }

    /// Whether the vault is currently unlocked.
    #[getter]
    fn is_unlocked(&self) -> bool {
        self.inner.is_unlocked()
    }

    /// Store a model and return the `ModelVersion`.
    ///
    /// Args:
    ///     name: Model name.
    ///     data: Raw model bytes.
    ///     metadata: `ModelMetadata` instance.
    ///     parent_version: Optional parent version number for lineage tracking.
    ///
    /// Returns:
    ///     `ModelVersion` — the newly created version.
    #[pyo3(signature = (name, data, metadata, parent_version=None))]
    fn store_model(
        &mut self,
        name: &str,
        data: &[u8],
        metadata: &PyModelMetadata,
        parent_version: Option<u32>,
    ) -> PyResult<PyModelVersion> {
        let ver = self
            .inner
            .store_model(name, data.to_vec(), metadata.inner.clone(), parent_version)
            .map_err(to_py_err)?;
        Ok(PyModelVersion { inner: ver })
    }

    /// Retrieve model data as `bytes`.
    ///
    /// Args:
    ///     name: Model name.
    ///     version: Optional version number (latest if omitted).
    #[pyo3(signature = (name, version=None))]
    fn get_model<'py>(
        &self,
        py: Python<'py>,
        name: &str,
        version: Option<u32>,
    ) -> PyResult<Bound<'py, PyBytes>> {
        let data = self.inner.get_model(name, version).map_err(to_py_err)?;
        Ok(PyBytes::new(py, &data))
    }

    /// List all model names in the vault.
    fn list_models(&self) -> Vec<String> {
        self.inner.list_models()
    }

    /// List all versions for a given model.
    fn list_versions(&self, name: &str) -> Vec<PyModelVersion> {
        self.inner
            .list_versions(name)
            .into_iter()
            .map(|v| PyModelVersion { inner: v.clone() })
            .collect()
    }

    /// Get the full version lineage for a model version.
    fn get_lineage(&self, name: &str, version: u32) -> Vec<PyModelVersion> {
        self.inner
            .get_lineage(name, version)
            .into_iter()
            .map(|v| PyModelVersion { inner: v.clone() })
            .collect()
    }

    /// Delete a specific model version. Returns True if it existed.
    fn delete_version(&mut self, name: &str, version: u32) -> PyResult<bool> {
        self.inner.delete_version(name, version).map_err(to_py_err)
    }

    /// Get vault statistics.
    fn get_stats<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let stats = self.inner.get_stats().map_err(to_py_err)?;
        let dict = PyDict::new(py);
        dict.set_item("model_count", stats.model_count)?;
        dict.set_item("total_versions", stats.total_versions)?;
        dict.set_item("total_size_bytes", stats.total_size_bytes)?;
        Ok(dict)
    }

    /// Change the vault passphrase (re-encrypts all models).
    ///
    /// Returns the number of models re-encrypted.
    fn change_passphrase(&mut self, new_passphrase: &[u8]) -> PyResult<usize> {
        self.inner
            .change_passphrase(new_passphrase.to_vec())
            .map_err(to_py_err)
    }

    /// Store a model from an iterable of `bytes` chunks (streaming ingest).
    #[pyo3(signature = (name, chunks, metadata, parent_version=None))]
    fn store_model_streamed(
        &mut self,
        name: &str,
        chunks: &Bound<'_, PyAny>,
        metadata: &PyModelMetadata,
        parent_version: Option<u32>,
    ) -> PyResult<PyModelVersion> {
        let mut buf = Vec::new();
        let iter = chunks.try_iter()?;
        for item in iter {
            let item = item?;
            let bytes: &[u8] = item.extract()?;
            buf.extend_from_slice(bytes);
        }
        let ver = self
            .inner
            .store_model(name, buf, metadata.inner.clone(), parent_version)
            .map_err(to_py_err)?;
        Ok(PyModelVersion { inner: ver })
    }

    /// Retrieve a model as a `ModelStream` of fixed-size chunks.
    #[pyo3(signature = (name, version=None, chunk_size=None))]
    fn get_model_streamed(
        &self,
        name: &str,
        version: Option<u32>,
        chunk_size: Option<usize>,
    ) -> PyResult<PyModelStream> {
        let cs = chunk_size.unwrap_or(8 * 1024 * 1024);
        let stream = self
            .inner
            .get_model_chunked(name, version, cs)
            .map_err(to_py_err)?;
        Ok(PyModelStream { inner: stream })
    }

    /// Get the vault configuration.
    #[getter]
    fn config(&self) -> PyVaultConfig {
        PyVaultConfig {
            inner: self.inner.get_config().clone(),
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "Vault(unlocked={}, path='{}')",
            self.inner.is_unlocked(),
            self.inner.get_config().get_vault_path(None).display()
        )
    }
}

// ── PyModelCard ──────────────────────────────────────────────────────────────

/// Model card for documentation and transparency.
///
/// Example::
///
///     from ironvault import ModelCard
///
///     card = ModelCard(
///         name="my-model", version="1.0",
///         model_type="transformer", description="A fine-tuned LLM"
///     )
///     print(card.to_markdown())
// Opt in to the FromPyObject derive: before pyo3 0.29 it was automatic for
// Clone pyclasses, and callers extract these types by value.
#[pyclass(name = "ModelCard", from_py_object)]
#[derive(Clone)]
struct PyModelCard {
    inner: ModelCard,
}

#[pymethods]
impl PyModelCard {
    /// Create a new model card.
    #[new]
    #[pyo3(signature = (
        name, version, model_type, *,
        description=None,
        developers=None,
        license=None,
        primary_use=None,
        out_of_scope=None
    ))]
    #[allow(clippy::too_many_arguments)]
    fn new(
        name: &str,
        version: &str,
        model_type: &str,
        description: Option<String>,
        developers: Option<Vec<String>>,
        license: Option<String>,
        primary_use: Option<String>,
        out_of_scope: Option<Vec<String>>,
    ) -> Self {
        let details = ModelDetails {
            name: name.to_string(),
            version: version.to_string(),
            model_type: model_type.to_string(),
            description: description.unwrap_or_default(),
            developers: developers.unwrap_or_default(),
            license,
            contact: None,
            architecture: String::new(),
            size: String::new(),
            framework: String::new(),
            format: String::new(),
            citation: None,
            repository: None,
            paper: None,
        };
        let intended = IntendedUse {
            primary_uses: primary_use.map(|s| vec![s]).unwrap_or_default(),
            primary_users: Vec::new(),
            out_of_scope_uses: out_of_scope.unwrap_or_default(),
            use_case_examples: None,
        };
        Self {
            inner: ModelCard::new(details, intended),
        }
    }

    /// Add training data information.
    #[pyo3(signature = (description, *, source=None, preprocessing=None))]
    fn set_training_data(
        &mut self,
        description: String,
        source: Option<String>,
        preprocessing: Option<String>,
    ) {
        let td = TrainingData {
            datasets: vec![description],
            sources: source.map(|s| vec![s]),
            collection_methods: None,
            preprocessing: preprocessing.map(|s| vec![s]),
            size: None,
            splits: None,
            languages: None,
            demographics: None,
        };
        {
            let card = self.inner.clone().with_training_data(td);
            self.inner = card;
        }
    }

    /// Add an evaluation metric.
    fn add_metric(&mut self, name: String, value: f64, description: String) {
        let metric = Metric {
            name,
            value,
            description: Some(description),
            threshold: None,
        };
        let eval = Evaluation {
            metrics: vec![metric],
            datasets: Vec::new(),
            benchmarks: None,
            methodology: None,
            performance_by_group: None,
        };
        {
            let card = self.inner.clone().with_evaluation(eval);
            self.inner = card;
        }
    }

    /// Add a custom metadata key-value pair.
    fn add_metadata(&mut self, key: String, value: String) {
        let card = self.inner.clone().add_metadata(key, value);
        self.inner = card;
    }

    /// Serialize to JSON string.
    fn to_json(&self) -> PyResult<String> {
        self.inner.to_json().map_err(to_py_err)
    }

    /// Serialize to YAML string.
    fn to_yaml(&self) -> PyResult<String> {
        self.inner.to_yaml().map_err(to_py_err)
    }

    /// Render as Markdown string.
    fn to_markdown(&self) -> String {
        self.inner.to_markdown()
    }

    /// Deserialize from JSON.
    #[staticmethod]
    fn from_json(json: &str) -> PyResult<Self> {
        let card = ModelCard::from_json(json).map_err(to_py_err)?;
        Ok(Self { inner: card })
    }

    /// Deserialize from YAML.
    #[staticmethod]
    fn from_yaml(yaml: &str) -> PyResult<Self> {
        let card = ModelCard::from_yaml(yaml).map_err(to_py_err)?;
        Ok(Self { inner: card })
    }

    fn __repr__(&self) -> String {
        format!(
            "ModelCard(name='{}', version='{}')",
            self.inner.model_details.name, self.inner.model_details.version
        )
    }
}

// ── PyModelStream ────────────────────────────────────────────────────────────

/// Iterator that yields fixed-size `bytes` chunks of a model.
#[pyclass(name = "ModelStream")]
struct PyModelStream {
    inner: ModelStream,
}

#[pymethods]
impl PyModelStream {
    #[getter]
    fn total_size(&self) -> usize {
        self.inner.total_size()
    }

    #[getter]
    fn remaining(&self) -> usize {
        self.inner.remaining()
    }

    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__<'py>(&mut self, py: Python<'py>) -> Option<Bound<'py, PyBytes>> {
        self.inner.next().map(|chunk| PyBytes::new(py, &chunk))
    }

    fn __repr__(&self) -> String {
        format!(
            "ModelStream(total_size={}, remaining={})",
            self.inner.total_size(),
            self.inner.remaining(),
        )
    }

    fn __len__(&self) -> usize {
        self.inner.total_size()
    }
}

// ── PyVaultBuilder ───────────────────────────────────────────────────────────

/// Builder for configuring a `Vault` with optional backends.
///
/// Example::
///
///     from ironvault import VaultBuilder, VaultConfig
///
///     vault = VaultBuilder() \
///         .config(VaultConfig()) \
///         .sqlite_versions() \
///         .build()
///     vault.unlock(b"my-passphrase")
#[pyclass(name = "VaultBuilder")]
struct PyVaultBuilder {
    config: Option<VaultConfig>,
    use_sqlite: bool,
    default_subscribers: bool,
}

#[pymethods]
impl PyVaultBuilder {
    #[new]
    fn new() -> Self {
        Self {
            config: None,
            use_sqlite: false,
            default_subscribers: true,
        }
    }

    /// Set a custom `VaultConfig`.
    fn config<'py>(mut slf: PyRefMut<'py, Self>, config: &PyVaultConfig) -> PyRefMut<'py, Self> {
        slf.config = Some(config.inner.clone());
        slf
    }

    /// Use SQLite for version storage instead of JSON files.
    fn sqlite_versions(mut slf: PyRefMut<'_, Self>) -> PyRefMut<'_, Self> {
        slf.use_sqlite = true;
        slf
    }

    /// Disable built-in audit and metrics event subscribers.
    fn no_default_subscribers(mut slf: PyRefMut<'_, Self>) -> PyRefMut<'_, Self> {
        slf.default_subscribers = false;
        slf
    }

    /// Build and return a configured `Vault`.
    fn build(&self) -> PyResult<PyVault> {
        let mut builder = crate::vault::VaultBuilder::new();
        if let Some(cfg) = &self.config {
            builder = builder.config(cfg.clone());
        }
        #[cfg(feature = "sqlite")]
        if self.use_sqlite {
            builder = builder.sqlite_versions();
        }
        #[cfg(not(feature = "sqlite"))]
        if self.use_sqlite {
            return Err(PyRuntimeError::new_err(
                "SQLite version backend requires the 'sqlite' feature",
            ));
        }
        if !self.default_subscribers {
            builder = builder.no_default_subscribers();
        }
        let vault = builder.build().map_err(to_py_err)?;
        Ok(PyVault { inner: vault })
    }

    fn __repr__(&self) -> String {
        format!(
            "VaultBuilder(sqlite={}, default_subscribers={})",
            self.use_sqlite, self.default_subscribers
        )
    }
}

// ── PyVaultError wrapper ─────────────────────────────────────────────────────

/// Standalone utility: SHA-256 hex digest of data.
#[pyfunction]
fn sha256_hex(data: &[u8]) -> String {
    crate::crypto::VaultCrypto::hash_sha256_hex(data)
}

/// Library version string.
#[pyfunction]
fn version() -> &'static str {
    crate::VERSION
}

// ── PyTagStore ───────────────────────────────────────────────────────────────

/// Tag store for model tagging and search.
#[pyclass(name = "TagStore")]
struct PyTagStore {
    inner: crate::tags::TagStore,
}

#[pymethods]
impl PyTagStore {
    #[new]
    fn new(vault_path: &str) -> PyResult<Self> {
        let path = std::path::PathBuf::from(vault_path);
        let store = crate::tags::TagStore::new(&path).map_err(to_py_err)?;
        Ok(Self { inner: store })
    }

    fn add_tags(&mut self, model: &str, tags: Vec<String>) -> PyResult<()> {
        self.inner.add_tags(model, &tags).map_err(to_py_err)
    }

    fn remove_tags(&mut self, model: &str, tags: Vec<String>) -> PyResult<()> {
        self.inner.remove_tags(model, &tags).map_err(to_py_err)
    }

    fn get_tags(&self, model: &str) -> Vec<String> {
        self.inner.get_tags(model).into_iter().collect()
    }

    fn search(
        &self,
        query: Option<String>,
        tags: Option<Vec<String>>,
        known_models: Vec<String>,
    ) -> Vec<HashMap<String, String>> {
        let sq = crate::tags::SearchQuery {
            name_pattern: query,
            tags: tags.unwrap_or_default(),
            annotations: vec![],
        };
        self.inner
            .search(&sq, &known_models)
            .into_iter()
            .map(|r| {
                let mut m = HashMap::new();
                m.insert("model".into(), r.model);
                m.insert("tags".into(), format!("{:?}", r.tags));
                m
            })
            .collect()
    }

    fn __repr__(&self) -> String {
        "TagStore(...)".to_string()
    }
}

// ── PyAclGuard ───────────────────────────────────────────────────────────────

/// Access control guard for role-based permissions.
#[pyclass(name = "AclGuard")]
struct PyAclGuard {
    inner: crate::access_control::AclGuard,
}

#[pymethods]
impl PyAclGuard {
    #[new]
    fn new(vault_path: &str) -> PyResult<Self> {
        let path = std::path::PathBuf::from(vault_path);
        let guard = crate::access_control::AclGuard::new(&path).map_err(to_py_err)?;
        Ok(Self { inner: guard })
    }

    fn grant(&mut self, principal: &str, role: &str) -> PyResult<()> {
        let r: crate::access_control::Role = role
            .parse()
            .map_err(|e: crate::error::VaultError| PyValueError::new_err(e.to_string()))?;
        self.inner.grant(principal, r).map_err(to_py_err)
    }

    fn revoke(&mut self, principal: &str) -> PyResult<bool> {
        self.inner.revoke(principal).map_err(to_py_err)
    }

    fn list(&self) -> Vec<HashMap<String, String>> {
        self.inner
            .list()
            .iter()
            .map(|e| {
                let mut m = HashMap::new();
                m.insert("principal".into(), e.principal.clone());
                m.insert("role".into(), e.role.to_string());
                m
            })
            .collect()
    }

    fn check(&self, principal: &str, role: &str) -> PyResult<bool> {
        let r: crate::access_control::Role = role
            .parse()
            .map_err(|e: crate::error::VaultError| PyValueError::new_err(e.to_string()))?;
        Ok(self
            .inner
            .resolve(principal)
            .is_some_and(|resolved| resolved >= r))
    }

    fn __repr__(&self) -> String {
        "AclGuard(...)".to_string()
    }
}

// ── PyProfileStore ───────────────────────────────────────────────────────────

/// Configuration profile manager.
#[pyclass(name = "ProfileStore")]
struct PyProfileStore {
    inner: crate::profiles::ProfileStore,
}

#[pymethods]
impl PyProfileStore {
    #[new]
    fn new(config_dir: &str) -> PyResult<Self> {
        let path = std::path::PathBuf::from(config_dir);
        let store = crate::profiles::ProfileStore::new(&path).map_err(to_py_err)?;
        Ok(Self { inner: store })
    }

    #[pyo3(signature = (name, description=None, overrides=None))]
    fn create_profile(
        &mut self,
        name: &str,
        description: Option<String>,
        overrides: Option<HashMap<String, String>>,
    ) -> PyResult<()> {
        let profile = crate::profiles::Profile {
            name: name.to_string(),
            description,
            overrides: overrides.unwrap_or_default().into_iter().collect(),
            created_at: chrono::Utc::now().to_rfc3339(),
        };
        self.inner.set(profile).map_err(to_py_err)
    }

    fn activate(&mut self, name: &str) -> PyResult<()> {
        self.inner.activate(name).map_err(to_py_err)
    }

    fn deactivate(&mut self) -> PyResult<()> {
        self.inner.deactivate().map_err(to_py_err)
    }

    fn active_name(&self) -> Option<String> {
        self.inner.active_name().map(|s| s.to_string())
    }

    fn list_profiles(&self) -> Vec<String> {
        self.inner.list().iter().map(|p| p.name.clone()).collect()
    }

    fn __repr__(&self) -> String {
        format!("ProfileStore(active={:?})", self.inner.active_name())
    }
}

// ── PyQuantProfileStore ──────────────────────────────────────────────────────

/// Quantization profile manager.
#[pyclass(name = "QuantProfileStore")]
struct PyQuantProfileStore {
    inner: crate::quantization::QuantProfileStore,
}

#[pymethods]
impl PyQuantProfileStore {
    #[new]
    fn new(vault_path: &str) -> PyResult<Self> {
        let path = std::path::PathBuf::from(vault_path);
        let store = crate::quantization::QuantProfileStore::new(&path).map_err(to_py_err)?;
        Ok(Self { inner: store })
    }

    #[pyo3(signature = (name, method, description=None))]
    fn set(&mut self, name: &str, method: &str, description: Option<String>) -> PyResult<()> {
        let m: crate::quantization::QuantMethod = method
            .parse()
            .map_err(|e: crate::error::VaultError| PyValueError::new_err(e.to_string()))?;
        let profile = crate::quantization::QuantProfile {
            name: name.to_string(),
            method: m,
            description,
            metadata: std::collections::BTreeMap::new(),
        };
        self.inner.set(profile).map_err(to_py_err)
    }

    fn remove(&mut self, name: &str) -> PyResult<bool> {
        self.inner.remove(name).map_err(to_py_err)
    }

    fn list(&self) -> Vec<HashMap<String, String>> {
        self.inner
            .list()
            .iter()
            .map(|p| {
                let mut m = HashMap::new();
                m.insert("name".into(), p.name.clone());
                m.insert("method".into(), p.method.to_string());
                if let Some(ref d) = p.description {
                    m.insert("description".into(), d.clone());
                }
                m
            })
            .collect()
    }

    #[staticmethod]
    fn estimate(size: u64, from_method: &str, to_method: &str) -> PyResult<u64> {
        let from: crate::quantization::QuantMethod = from_method
            .parse()
            .map_err(|e: crate::error::VaultError| PyValueError::new_err(e.to_string()))?;
        let to: crate::quantization::QuantMethod = to_method
            .parse()
            .map_err(|e: crate::error::VaultError| PyValueError::new_err(e.to_string()))?;
        Ok(crate::quantization::estimate_quantized_size(size, from, to))
    }

    fn __repr__(&self) -> String {
        format!("QuantProfileStore({} profiles)", self.inner.list().len())
    }
}

// ── PyEvalStore ──────────────────────────────────────────────────────────────

/// Evaluation result store.
#[pyclass(name = "EvalStore")]
struct PyEvalStore {
    inner: crate::evaluation::EvalStore,
}

#[pymethods]
impl PyEvalStore {
    #[new]
    fn new(vault_path: &str) -> PyResult<Self> {
        let path = std::path::PathBuf::from(vault_path);
        let store = crate::evaluation::EvalStore::new(&path).map_err(to_py_err)?;
        Ok(Self { inner: store })
    }

    #[pyo3(signature = (model, version, suite, metrics, higher_is_better=true))]
    fn record(
        &mut self,
        model: &str,
        version: u64,
        suite: &str,
        metrics: HashMap<String, f64>,
        higher_is_better: bool,
    ) -> PyResult<()> {
        let metric_results: Vec<crate::evaluation::MetricResult> = metrics
            .into_iter()
            .map(|(name, value)| crate::evaluation::MetricResult {
                name,
                value,
                unit: "score".to_string(),
                higher_is_better,
            })
            .collect();
        let run = crate::evaluation::EvalRun {
            suite: suite.to_string(),
            model: model.to_string(),
            version,
            metrics: metric_results,
            timestamp: chrono::Utc::now().to_rfc3339(),
            context: std::collections::BTreeMap::new(),
        };
        self.inner.record(run).map_err(to_py_err)
    }

    #[pyo3(signature = (model, version=None))]
    fn get_runs(&self, model: &str, version: Option<u64>) -> Vec<HashMap<String, String>> {
        self.inner
            .get_runs(model, version)
            .iter()
            .map(|r| {
                let mut m = HashMap::new();
                m.insert("model".into(), r.model.clone());
                m.insert("version".into(), r.version.to_string());
                m.insert("suite".into(), r.suite.clone());
                m.insert("timestamp".into(), r.timestamp.clone());
                m
            })
            .collect()
    }

    fn suites(&self) -> Vec<String> {
        self.inner.suites()
    }

    fn count(&self) -> usize {
        self.inner.count()
    }

    fn __repr__(&self) -> String {
        format!("EvalStore({} runs)", self.inner.count())
    }
}

// ── PyBackupManager ──────────────────────────────────────────────────────────

/// Backup schedule manager.
#[pyclass(name = "BackupManager")]
struct PyBackupManager {
    inner: crate::scheduler::BackupManager,
}

#[pymethods]
impl PyBackupManager {
    #[new]
    fn new(vault_path: &str) -> PyResult<Self> {
        let path = std::path::PathBuf::from(vault_path);
        let mgr = crate::scheduler::BackupManager::new(&path).map_err(to_py_err)?;
        Ok(Self { inner: mgr })
    }

    fn set_schedule(
        &mut self,
        name: &str,
        frequency: &str,
        max_backups: usize,
        output_dir: &str,
    ) -> PyResult<()> {
        let freq: crate::scheduler::BackupFrequency = frequency
            .parse()
            .map_err(|e: crate::error::VaultError| PyValueError::new_err(e.to_string()))?;
        let schedule = crate::scheduler::BackupSchedule {
            name: name.to_string(),
            frequency: freq,
            max_backups,
            output_dir: std::path::PathBuf::from(output_dir),
            enabled: true,
            created_at: chrono::Utc::now().to_rfc3339(),
        };
        self.inner.set_schedule(schedule).map_err(to_py_err)
    }

    fn remove_schedule(&mut self, name: &str) -> PyResult<bool> {
        self.inner.remove_schedule(name).map_err(to_py_err)
    }

    fn list_schedules(&self) -> Vec<HashMap<String, String>> {
        self.inner
            .list_schedules()
            .iter()
            .map(|s| {
                let mut m = HashMap::new();
                m.insert("name".into(), s.name.clone());
                m.insert("frequency".into(), s.frequency.to_string());
                m.insert("max_backups".into(), s.max_backups.to_string());
                m.insert("output_dir".into(), s.output_dir.display().to_string());
                m.insert("enabled".into(), s.enabled.to_string());
                m
            })
            .collect()
    }

    fn backup_count(&self) -> usize {
        self.inner.backup_count()
    }

    fn __repr__(&self) -> String {
        format!(
            "BackupManager({} schedules, {} backups)",
            self.inner.list_schedules().len(),
            self.inner.backup_count()
        )
    }
}

// ── PyVaultRegistry ──────────────────────────────────────────────────────────

/// Multi-vault registry.
#[pyclass(name = "VaultRegistry")]
struct PyVaultRegistry {
    inner: crate::multi_vault::VaultRegistry,
}

#[pymethods]
impl PyVaultRegistry {
    #[new]
    fn new(config_dir: &str) -> PyResult<Self> {
        let path = std::path::PathBuf::from(config_dir);
        let reg = crate::multi_vault::VaultRegistry::new(&path).map_err(to_py_err)?;
        Ok(Self { inner: reg })
    }

    #[pyo3(signature = (name, path, description=None))]
    fn register(&mut self, name: &str, path: &str, description: Option<String>) -> PyResult<()> {
        let entry = crate::multi_vault::VaultEntry {
            name: name.to_string(),
            path: std::path::PathBuf::from(path),
            description,
            registered_at: chrono::Utc::now().to_rfc3339(),
        };
        self.inner.register(entry).map_err(to_py_err)
    }

    fn unregister(&mut self, name: &str) -> PyResult<bool> {
        self.inner.unregister(name).map_err(to_py_err)
    }

    fn activate(&mut self, name: &str) -> PyResult<()> {
        self.inner.activate(name).map_err(to_py_err)
    }

    fn deactivate(&mut self) -> PyResult<()> {
        self.inner.deactivate().map_err(to_py_err)
    }

    fn active_name(&self) -> Option<String> {
        self.inner.active_name().map(|s| s.to_string())
    }

    fn list(&self) -> Vec<HashMap<String, String>> {
        self.inner
            .list()
            .iter()
            .map(|v| {
                let mut m = HashMap::new();
                m.insert("name".into(), v.name.clone());
                m.insert("path".into(), v.path.display().to_string());
                m.insert("is_active".into(), v.is_active.to_string());
                m.insert("exists".into(), v.exists.to_string());
                m
            })
            .collect()
    }

    fn count(&self) -> usize {
        self.inner.count()
    }

    fn __repr__(&self) -> String {
        format!(
            "VaultRegistry({} vaults, active={:?})",
            self.inner.count(),
            self.inner.active_name()
        )
    }
}

// ── module init ──────────────────────────────────────────────────────────────

/// The `ironvault._native` extension module.
#[pymodule]
#[pyo3(name = "_native")]
fn ironvault_native(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyModelFormat>()?;
    m.add_class::<PyModelMetadata>()?;
    m.add_class::<PyModelVersion>()?;
    m.add_class::<PyVaultConfig>()?;
    m.add_class::<PyVault>()?;
    m.add_class::<PyModelCard>()?;
    m.add_class::<PyModelStream>()?;
    m.add_class::<PyVaultBuilder>()?;
    m.add_class::<PyTagStore>()?;
    m.add_class::<PyAclGuard>()?;
    m.add_class::<PyProfileStore>()?;
    m.add_class::<PyQuantProfileStore>()?;
    m.add_class::<PyEvalStore>()?;
    m.add_class::<PyBackupManager>()?;
    m.add_class::<PyVaultRegistry>()?;
    m.add_function(wrap_pyfunction!(sha256_hex, m)?)?;
    m.add_function(wrap_pyfunction!(version, m)?)?;
    Ok(())
}

// ── tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // parse_format tests — no Python runtime needed

    #[test]
    fn parse_format_safetensors() {
        assert!(matches!(
            parse_format("safetensors").unwrap(),
            ModelFormat::Safetensors
        ));
    }

    #[test]
    fn parse_format_gguf() {
        assert!(matches!(parse_format("gguf").unwrap(), ModelFormat::GGUF));
    }

    #[test]
    fn parse_format_pytorch_aliases() {
        assert!(matches!(
            parse_format("pytorch").unwrap(),
            ModelFormat::PyTorch
        ));
        assert!(matches!(parse_format("pt").unwrap(), ModelFormat::PyTorch));
        assert!(matches!(parse_format("pth").unwrap(), ModelFormat::PyTorch));
    }

    #[test]
    fn parse_format_tensorrt_aliases() {
        assert!(matches!(
            parse_format("tensorrt").unwrap(),
            ModelFormat::TensorRT
        ));
        assert!(matches!(
            parse_format("plan").unwrap(),
            ModelFormat::TensorRT
        ));
    }

    #[test]
    fn parse_format_onnx() {
        assert!(matches!(parse_format("onnx").unwrap(), ModelFormat::ONNX));
    }

    #[test]
    fn parse_format_mlx() {
        assert!(matches!(parse_format("mlx").unwrap(), ModelFormat::MLX));
    }

    #[test]
    fn parse_format_coreml_aliases() {
        assert!(matches!(
            parse_format("coreml").unwrap(),
            ModelFormat::CoreML
        ));
        assert!(matches!(
            parse_format("mlmodel").unwrap(),
            ModelFormat::CoreML
        ));
    }

    #[test]
    fn parse_format_torchscript() {
        assert!(matches!(
            parse_format("torchscript").unwrap(),
            ModelFormat::TorchScript
        ));
    }

    #[test]
    fn parse_format_tflite() {
        assert!(matches!(
            parse_format("tflite").unwrap(),
            ModelFormat::TFLite
        ));
    }

    #[test]
    fn parse_format_tensorflow_aliases() {
        assert!(matches!(
            parse_format("tensorflow").unwrap(),
            ModelFormat::TensorFlow
        ));
        assert!(matches!(
            parse_format("tf").unwrap(),
            ModelFormat::TensorFlow
        ));
        assert!(matches!(
            parse_format("pb").unwrap(),
            ModelFormat::TensorFlow
        ));
    }

    #[test]
    fn parse_format_keras_aliases() {
        assert!(matches!(parse_format("keras").unwrap(), ModelFormat::Keras));
        assert!(matches!(parse_format("h5").unwrap(), ModelFormat::Keras));
    }

    #[test]
    fn parse_format_openvino() {
        assert!(matches!(
            parse_format("openvino").unwrap(),
            ModelFormat::OpenVINO
        ));
    }

    #[test]
    fn parse_format_tvm() {
        assert!(matches!(parse_format("tvm").unwrap(), ModelFormat::TVM));
    }

    #[test]
    fn parse_format_ncnn() {
        assert!(matches!(parse_format("ncnn").unwrap(), ModelFormat::NCNN));
    }

    #[test]
    fn parse_format_mnn() {
        assert!(matches!(parse_format("mnn").unwrap(), ModelFormat::MNN));
    }

    #[test]
    fn parse_format_rknn() {
        assert!(matches!(parse_format("rknn").unwrap(), ModelFormat::RKNN));
    }

    #[test]
    fn parse_format_caffe() {
        assert!(matches!(parse_format("caffe").unwrap(), ModelFormat::Caffe));
    }

    #[test]
    fn parse_format_mxnet() {
        assert!(matches!(parse_format("mxnet").unwrap(), ModelFormat::MXNet));
    }

    #[test]
    fn parse_format_darknet() {
        assert!(matches!(
            parse_format("darknet").unwrap(),
            ModelFormat::Darknet
        ));
    }

    #[test]
    fn parse_format_hdf5() {
        assert!(matches!(parse_format("hdf5").unwrap(), ModelFormat::HDF5));
    }

    #[test]
    fn parse_format_pickle_aliases() {
        assert!(matches!(
            parse_format("pickle").unwrap(),
            ModelFormat::Pickle
        ));
        assert!(matches!(parse_format("pkl").unwrap(), ModelFormat::Pickle));
    }

    #[test]
    fn parse_format_numpy_aliases() {
        assert!(matches!(parse_format("numpy").unwrap(), ModelFormat::NumPy));
        assert!(matches!(parse_format("npy").unwrap(), ModelFormat::NumPy));
        assert!(matches!(parse_format("npz").unwrap(), ModelFormat::NumPy));
    }

    #[test]
    fn parse_format_unknown_returns_custom() {
        let fmt = parse_format("unknown_format").unwrap();
        assert!(matches!(fmt, ModelFormat::Custom(ref s) if s == "unknown_format"));
    }

    #[test]
    fn parse_format_case_insensitive() {
        assert!(matches!(
            parse_format("SAFETENSORS").unwrap(),
            ModelFormat::Safetensors
        ));
        assert!(matches!(
            parse_format("PyTorch").unwrap(),
            ModelFormat::PyTorch
        ));
        assert!(matches!(parse_format("ONNX").unwrap(), ModelFormat::ONNX));
    }
}
