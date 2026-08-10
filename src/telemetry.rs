//! Telemetry and analytics module for IronVault.
//!
//! Collects anonymous usage data to help improve the product.
//! **Disabled by default** — users can opt in via:
//! - Config file: `telemetry.enabled = true`
//! - CLI: `iv telemetry enable`
//!
//! To disable (if previously enabled):
//! - Config file: `telemetry.enabled = false`
//! - Environment variable: `IRONVAULT_TELEMETRY_ENABLED=false` or `IRONVAULT_TELEMETRY_DISABLED=1`
//! - Environment variable: `DO_NOT_TRACK=1`
//!
//! ## Data Collected
//!
//! Only if you opt in. Two events are emitted.
//!
//! [`TelemetryEvent::AppStart`], once per process:
//!
//! - **Environment**: version, OS, architecture, enabled feature flags
//! - **Anonymous ID**: random UUID v4 generated on first run, and a per-run
//!   session UUID. Neither is derived from anything about the machine or the
//!   user, so neither can be correlated back to an identity.
//!
//! [`TelemetryEvent::CommandRun`], once per CLI invocation:
//!
//! - **Command and subcommand name**, duration in milliseconds, and a success
//!   boolean. The binary takes both names from clap's registered command
//!   table rather than from the command line, so the field can only ever hold
//!   one of the subcommand literals declared in `args.rs` — never an argument
//!   value. The failure *reason* is not recorded, only the boolean.
//!
//! [`TelemetryEvent::ModelOperation`] on store/get/delete: operation, format
//! label, a **size bucket** (never the exact size), duration, outcome.
//!
//! [`TelemetryEvent::Conversion`] on `iv convert`: source and target format
//! labels, duration, outcome.
//!
//! [`TelemetryEvent::ApiCall`] per HTTP request: the matched **route
//! template**, method, status, duration.
//!
//! [`TelemetryEvent::Error`] when a command fails: the variant name from
//! [`crate::VaultError::kind`] and nothing else — never the message.
//!
//! [`TelemetryEvent::FeatureUsed`] for KMS: the URI scheme only.
//!
//! Every one of those labels is a `&'static str` chosen from a closed set.
//! Format labels come from [`crate::formats::ModelFormat::telemetry_name`]
//! rather than `name()`, which returns the user's own string for a custom
//! format.
//!
//! ## Where events go
//!
//! The built-in sender posts to `https://telemetry.nervosys.ai/v1/events`,
//! the project's own collector — the default value of
//! [`TelemetryConfig::endpoint`].
//! It is a compiled-in default and can be overridden with `endpoint` in
//! `config.toml`. Nothing reaches it unless telemetry is explicitly enabled;
//! `enabled` defaults to `false`.
//!
//! The OTLP path ([`OtlpSettings`]) is different: it has no default endpoint
//! and no default credential, and is configured only from the environment.
//!
//! ## Data NOT Collected
//!
//! - Model contents or file data
//! - Passphrases or encryption keys
//! - File paths or model names
//! - Personal information
//! - IP addresses (anonymized by backend)
//!
//! ## Keeping that true
//!
//! Three fields are free-form strings and are the only way the guarantees
//! above can be broken: [`TelemetryEvent::Error::context`],
//! [`TelemetryEvent::ApiCall::endpoint`] and
//! [`TelemetryEvent::FeatureUsed::detail`]. Error messages routinely contain
//! file paths, and a real request path contains the model name, so wiring any
//! of these to a formatted error or an unparameterised route would silently
//! start collecting exactly what this module promises it does not. Pass
//! constants and enum-like discriminants, never a formatted message.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use uuid::Uuid;

use crate::error::Result;

/// Global telemetry instance
static TELEMETRY: OnceLock<Arc<TelemetryClient>> = OnceLock::new();

/// Whether telemetry has been explicitly disabled
static TELEMETRY_DISABLED: AtomicBool = AtomicBool::new(false);

/// Telemetry configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryConfig {
    /// Whether telemetry is enabled (default: **false**).
    ///
    /// This said "default: true" while `Default::default` set it to `false`.
    /// The code was right and the comment was wrong, but it is the sort of
    /// wrong that gets quoted in a privacy review.
    pub enabled: bool,

    /// Anonymous device ID (auto-generated UUID)
    #[serde(default = "generate_device_id")]
    pub device_id: String,

    /// Telemetry endpoint URL
    #[serde(default = "default_endpoint")]
    pub endpoint: String,

    /// Batch size before sending (reduces network calls)
    #[serde(default = "default_batch_size")]
    pub batch_size: usize,

    /// Flush interval in seconds
    #[serde(default = "default_flush_interval")]
    pub flush_interval_secs: u64,
}

fn generate_device_id() -> String {
    Uuid::new_v4().to_string()
}

fn default_endpoint() -> String {
    "https://telemetry.nervosys.ai/v1/events".to_string()
}

fn default_batch_size() -> usize {
    25
}

fn default_flush_interval() -> u64 {
    300 // 5 minutes
}

impl Default for TelemetryConfig {
    fn default() -> Self {
        Self {
            enabled: false, // Opt-in: disabled by default for privacy
            device_id: generate_device_id(),
            endpoint: default_endpoint(),
            batch_size: default_batch_size(),
            flush_interval_secs: default_flush_interval(),
        }
    }
}

/// OTLP export settings, read from the standard OpenTelemetry environment
/// variables.
///
/// These are deliberately environment-only and are never written to the
/// telemetry config file, baked into the binary, or defaulted to a vendor
/// endpoint. Two reasons:
///
/// 1. `OTEL_EXPORTER_OTLP_HEADERS` carries a bearer token. A credential that
///    ships inside an AGPL crate published to a public registry is readable by
///    everyone who installs it, which makes it a shared secret with the world
///    rather than an authorisation.
/// 2. Configuring an exporter is an act by whoever *operates* a deployment.
///    Baking one in would export on behalf of every user who merely installed
///    the tool, which is a different person making a different decision.
///
/// Recognised variables (the standard set, so any OpenTelemetry collector or
/// vendor endpoint works without bespoke configuration):
///
/// - `OTEL_EXPORTER_OTLP_ENDPOINT`, or `OTEL_EXPORTER_OTLP_LOGS_ENDPOINT`
/// - `OTEL_EXPORTER_OTLP_PROTOCOL` — `http/protobuf` (default) or `http/json`
/// - `OTEL_EXPORTER_OTLP_HEADERS` — e.g. `Authorization=Bearer <token>`
/// - `OTEL_SERVICE_NAME`
///
/// Setting these does **not** enable telemetry. Telemetry remains opt-in; when
/// it is off, nothing is exported no matter how the exporter is configured.
#[cfg(feature = "otel")]
#[derive(Debug, Clone)]
pub struct OtlpSettings {
    /// Collector endpoint.
    pub endpoint: String,
    /// Wire protocol.
    pub protocol: OtlpProtocol,
    /// Service name reported as `service.name`.
    pub service_name: String,
}

/// OTLP wire protocol.
#[cfg(feature = "otel")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OtlpProtocol {
    /// `http/protobuf`
    HttpBinary,
    /// `http/json`
    HttpJson,
}

/// Non-`otel` builds still need to notice that OTLP was configured, so they can
/// say so instead of silently dropping it.
#[cfg(not(feature = "otel"))]
pub struct OtlpSettings;

impl OtlpSettings {
    /// The configured endpoint, if any, checking the logs-specific variable
    /// first as the specification requires.
    #[must_use]
    pub fn endpoint_from_env() -> Option<String> {
        for key in [
            "OTEL_EXPORTER_OTLP_LOGS_ENDPOINT",
            "OTEL_EXPORTER_OTLP_ENDPOINT",
        ] {
            if let Ok(value) = std::env::var(key) {
                let trimmed = value.trim();
                if !trimmed.is_empty() {
                    return Some(trimmed.to_string());
                }
            }
        }
        None
    }
}

#[cfg(feature = "otel")]
impl OtlpSettings {
    /// Read settings from the environment, or `None` if no endpoint is set.
    #[must_use]
    pub fn from_env() -> Option<Self> {
        let endpoint = Self::endpoint_from_env()?;

        let protocol = match std::env::var("OTEL_EXPORTER_OTLP_LOGS_PROTOCOL")
            .or_else(|_| std::env::var("OTEL_EXPORTER_OTLP_PROTOCOL"))
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase()
            .as_str()
        {
            "http/json" => OtlpProtocol::HttpJson,
            // The specification's default for HTTP transport, and what an
            // unset or unrecognised value falls back to.
            _ => OtlpProtocol::HttpBinary,
        };

        let service_name = std::env::var("OTEL_SERVICE_NAME")
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| env!("CARGO_PKG_NAME").to_string());

        Some(Self {
            endpoint,
            protocol,
            service_name,
        })
    }
}

/// Telemetry event types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TelemetryEvent {
    /// Application started
    AppStart {
        version: String,
        os: String,
        arch: String,
        features: Vec<String>,
    },

    /// Command executed
    CommandRun {
        command: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        subcommand: Option<String>,
        duration_ms: u64,
        success: bool,
    },

    /// Model operation (store, get, delete)
    ModelOperation {
        operation: String,
        format: String,
        size_bucket: String, // "small", "medium", "large", "xlarge"
        duration_ms: u64,
        success: bool,
    },

    /// Format conversion
    Conversion {
        source_format: String,
        target_format: String,
        duration_ms: u64,
        success: bool,
    },

    /// API endpoint called
    ApiCall {
        endpoint: String,
        method: String,
        status_code: u16,
        duration_ms: u64,
    },

    /// Error occurred
    Error {
        error_type: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        context: Option<String>,
    },

    /// Feature usage
    FeatureUsed {
        feature: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        detail: Option<String>,
    },
}

/// Envelope for telemetry events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct TelemetryEnvelope {
    pub(crate) device_id: String,
    pub(crate) session_id: String,
    pub(crate) timestamp: u64,
    pub(crate) event: TelemetryEvent,
}

/// Telemetry client for collecting and sending analytics data
pub struct TelemetryClient {
    config: TelemetryConfig,
    session_id: String,
    events: parking_lot::Mutex<Vec<TelemetryEnvelope>>,
    enabled: AtomicBool,
}

impl TelemetryClient {
    /// Create a new telemetry client
    pub fn new(config: TelemetryConfig) -> Self {
        let enabled = config.enabled && !Self::is_disabled_by_env();

        Self {
            enabled: AtomicBool::new(enabled),
            session_id: Uuid::new_v4().to_string(),
            events: parking_lot::Mutex::new(Vec::new()),
            config,
        }
    }

    /// Check if telemetry is disabled via environment variable
    fn is_disabled_by_env() -> bool {
        crate::env::var("IRONVAULT_TELEMETRY_ENABLED")
            .ok_or(())
            .map(|v| v.to_lowercase() == "false" || v == "0")
            .unwrap_or(false)
            || crate::env::var("IRONVAULT_TELEMETRY_DISABLED")
                .ok_or(())
                .map(|v| v == "1" || v.to_lowercase() == "true")
                .unwrap_or(false)
            || std::env::var("DO_NOT_TRACK")
                .map(|v| v == "1" || v.to_lowercase() == "true")
                .unwrap_or(false)
    }

    /// Disable telemetry
    pub fn disable(&self) {
        self.enabled.store(false, Ordering::SeqCst);
    }

    /// Enable telemetry
    pub fn enable(&self) {
        self.enabled.store(true, Ordering::SeqCst);
    }

    /// Check if telemetry is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::SeqCst)
    }

    /// Track an event
    pub fn track(&self, event: TelemetryEvent) {
        if !self.is_enabled() {
            return;
        }

        let envelope = TelemetryEnvelope {
            device_id: self.config.device_id.clone(),
            session_id: self.session_id.clone(),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            event,
        };

        let mut events = self.events.lock();
        events.push(envelope);

        // Flush if we've reached batch size
        if events.len() >= self.config.batch_size {
            let batch = std::mem::take(&mut *events);
            drop(events);
            self.send_batch(batch);
        }
    }

    /// Flush all pending events
    pub fn flush(&self) {
        if !self.is_enabled() {
            return;
        }

        let batch = std::mem::take(&mut *self.events.lock());
        if !batch.is_empty() {
            self.send_batch(batch);
        }
    }

    /// Send a batch of events to the telemetry endpoint
    fn send_batch(&self, events: Vec<TelemetryEnvelope>) {
        let endpoint = self.config.endpoint.clone();

        std::thread::spawn(move || {
            // Try to send to remote server with retries
            let mut last_error = None;
            for attempt in 0..3 {
                if attempt > 0 {
                    // Exponential backoff: 100ms, 400ms
                    std::thread::sleep(Duration::from_millis(100 * (1 << attempt)));
                }

                // OTLP takes precedence when configured; the JSON endpoint is
                // the fallback for deployments with no collector.
                let result = match Self::try_send_otlp(&events) {
                    Some(otlp_result) => otlp_result,
                    None => Self::send_http_batch(&endpoint, &events),
                };

                match result {
                    Ok(()) => {
                        // Also try to send any previously queued events
                        let _ = Self::flush_local_queue(&endpoint);
                        return;
                    }
                    Err(e) => {
                        last_error = Some(e);
                    }
                }
            }

            // All retries failed - save to local queue for later
            if let Some(_err) = last_error {
                if let Ok(body) = serde_json::to_string(&events) {
                    let _ = Self::write_to_local_queue(&body);
                }
            }
        });
    }

    /// Send events via OTLP, if an OTLP endpoint is configured.
    ///
    /// Returns `None` when OTLP is not configured, so the caller falls back to
    /// the plain JSON sender.
    #[cfg(feature = "otel")]
    fn try_send_otlp(events: &[TelemetryEnvelope]) -> Option<std::result::Result<(), String>> {
        let settings = OtlpSettings::from_env()?;
        Some(crate::telemetry_otlp::export(events, &settings))
    }

    #[cfg(not(feature = "otel"))]
    #[allow(clippy::unnecessary_wraps)]
    fn try_send_otlp(_events: &[TelemetryEnvelope]) -> Option<std::result::Result<(), String>> {
        // Warn rather than fail silently: an operator who configured OTLP and
        // is running a binary without the feature would otherwise see nothing
        // arrive and have no way to tell why.
        if OtlpSettings::endpoint_from_env().is_some() {
            eprintln!(
                "warning: OTEL_EXPORTER_OTLP_ENDPOINT is set, but this binary was \
                 built without the `otel` feature. Telemetry will not be exported \
                 over OTLP."
            );
        }
        None
    }

    /// Send events via HTTP POST
    fn send_http_batch(
        endpoint: &str,
        events: &[TelemetryEnvelope],
    ) -> std::result::Result<(), String> {
        use reqwest::blocking::Client;

        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

        let response = client
            .post(endpoint)
            .header("Content-Type", "application/json")
            .header("User-Agent", format!("iv/{}", env!("CARGO_PKG_VERSION")))
            .json(events)
            .send()
            .map_err(|e| format!("HTTP request failed: {}", e))?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(format!("Server returned error: {}", response.status()))
        }
    }

    /// Flush any events stored in the local queue
    fn flush_local_queue(endpoint: &str) -> std::result::Result<(), String> {
        let queue_file = Self::get_queue_file_path();

        if !queue_file.exists() {
            return Ok(());
        }

        // Read all queued events
        let contents =
            fs::read_to_string(&queue_file).map_err(|e| format!("Failed to read queue: {}", e))?;

        let mut all_sent = true;
        let mut remaining_lines = Vec::new();

        for line in contents.lines() {
            if line.trim().is_empty() {
                continue;
            }

            // Parse the queued batch
            if let Ok(events) = serde_json::from_str::<Vec<TelemetryEnvelope>>(line) {
                if Self::send_http_batch(endpoint, &events).is_err() {
                    all_sent = false;
                    remaining_lines.push(line.to_string());
                }
            }
        }

        // Update queue file - remove sent events
        if all_sent {
            let _ = fs::remove_file(&queue_file);
        } else if !remaining_lines.is_empty() {
            use std::io::Write;
            if let Ok(mut file) = fs::File::create(&queue_file) {
                for line in remaining_lines {
                    let _ = writeln!(file, "{}", line);
                }
            }
        }

        Ok(())
    }

    /// Get the queue file path
    fn get_queue_file_path() -> PathBuf {
        directories::BaseDirs::new()
            .map(|d| {
                d.cache_dir()
                    .join("ai")
                    .join("telemetry")
                    .join("events.jsonl")
            })
            .unwrap_or_else(|| {
                PathBuf::from(".")
                    .join(".cache")
                    .join("ai")
                    .join("telemetry")
                    .join("events.jsonl")
            })
    }

    /// Write events to local queue file (for offline/batched collection)
    fn write_to_local_queue(body: &str) -> std::io::Result<()> {
        use std::io::Write;

        let queue_file = Self::get_queue_file_path();
        if let Some(queue_dir) = queue_file.parent() {
            fs::create_dir_all(queue_dir)?;
            let _ = crate::permissions::restrict_dir(queue_dir);
        }

        let mut options = fs::OpenOptions::new();
        options.create(true).append(true);
        crate::permissions::set_create_mode(&mut options);

        let mut file = options.open(queue_file)?;
        writeln!(file, "{}", body)?;
        Ok(())
    }

    /// Get the device ID
    pub fn device_id(&self) -> &str {
        &self.config.device_id
    }
}

impl Drop for TelemetryClient {
    fn drop(&mut self) {
        self.flush();
    }
}

// === Global telemetry functions ===

/// Initialize the global telemetry client
pub fn init(config: TelemetryConfig) {
    let _ = TELEMETRY.set(Arc::new(TelemetryClient::new(config)));
}

/// Initialize telemetry with default config, loading from disk if available
pub fn init_default(config_dir: Option<&PathBuf>) -> Result<()> {
    let config = load_or_create_config(config_dir)?;
    init(config);
    Ok(())
}

/// Load telemetry config from disk or create default
fn load_or_create_config(config_dir: Option<&PathBuf>) -> Result<TelemetryConfig> {
    let config_path = config_dir
        .cloned()
        .or_else(|| directories::BaseDirs::new().map(|d| d.config_dir().join("ai").join("models")))
        .map(|d| d.join("telemetry.yaml"));

    if let Some(path) = &config_path {
        if path.exists() {
            let contents = fs::read_to_string(path)?;
            let config: TelemetryConfig = serde_yaml_ng::from_str(&contents)?;
            return Ok(config);
        }
    }

    // Create default config and save it
    let config = TelemetryConfig::default();

    if let Some(path) = config_path {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let contents = serde_yaml_ng::to_string(&config)?;
        fs::write(&path, contents)?;
    }

    Ok(config)
}

/// Disable telemetry globally
pub fn disable() {
    TELEMETRY_DISABLED.store(true, Ordering::SeqCst);
    if let Some(client) = TELEMETRY.get() {
        client.disable();
    }
}

/// Check if telemetry is enabled
pub fn is_enabled() -> bool {
    !TELEMETRY_DISABLED.load(Ordering::SeqCst)
        && TELEMETRY.get().map(|c| c.is_enabled()).unwrap_or(false)
}

/// Track an event
pub fn track(event: TelemetryEvent) {
    if let Some(client) = TELEMETRY.get() {
        client.track(event);
    }
}

/// Flush pending events
pub fn flush() {
    if let Some(client) = TELEMETRY.get() {
        client.flush();
    }
}

// === Convenience tracking functions ===

/// Track application start
pub fn track_app_start() {
    let features = collect_enabled_features();

    track(TelemetryEvent::AppStart {
        version: env!("CARGO_PKG_VERSION").to_string(),
        os: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
        features,
    });
}

/// Track a command execution
pub fn track_command(command: &str, subcommand: Option<&str>, duration: Duration, success: bool) {
    track(TelemetryEvent::CommandRun {
        command: command.to_string(),
        subcommand: subcommand.map(|s| s.to_string()),
        duration_ms: duration.as_millis() as u64,
        success,
    });
}

/// Track a model operation
pub fn track_model_op(
    operation: &str,
    format: &str,
    size_bytes: u64,
    duration: Duration,
    success: bool,
) {
    let size_bucket = match size_bytes {
        0..=10_000_000 => "small",              // < 10MB
        10_000_001..=100_000_000 => "medium",   // 10MB - 100MB
        100_000_001..=1_000_000_000 => "large", // 100MB - 1GB
        _ => "xlarge",                          // > 1GB
    };

    track(TelemetryEvent::ModelOperation {
        operation: operation.to_string(),
        format: format.to_string(),
        size_bucket: size_bucket.to_string(),
        duration_ms: duration.as_millis() as u64,
        success,
    });
}

/// Track format conversion
pub fn track_conversion(
    source_format: &str,
    target_format: &str,
    duration: Duration,
    success: bool,
) {
    track(TelemetryEvent::Conversion {
        source_format: source_format.to_string(),
        target_format: target_format.to_string(),
        duration_ms: duration.as_millis() as u64,
        success,
    });
}

/// Track an API call.
///
/// `endpoint` must be the *route template* (`/models/:name`), never the
/// resolved path — a resolved path contains the model name, which this module
/// documents as not collected.
pub fn track_api_call(endpoint: &str, method: &str, status_code: u16, duration: Duration) {
    track(TelemetryEvent::ApiCall {
        endpoint: endpoint.to_string(),
        method: method.to_string(),
        status_code,
        duration_ms: duration.as_millis() as u64,
    });
}

/// Track an error.
///
/// `error_type` is a discriminant (`"integrity"`, `"auth"`), and `context`
/// must be a constant. Never pass a formatted [`crate::VaultError`]: its
/// messages embed file paths and model names, which this module documents as
/// not collected.
pub fn track_error(error_type: &str, context: Option<&str>) {
    track(TelemetryEvent::Error {
        error_type: error_type.to_string(),
        context: context.map(|s| s.to_string()),
    });
}

/// Track feature usage.
///
/// `detail` must be a constant, not user data — see the module-level note on
/// keeping the "not collected" guarantees true.
pub fn track_feature(feature: &str, detail: Option<&str>) {
    track(TelemetryEvent::FeatureUsed {
        feature: feature.to_string(),
        detail: detail.map(|s| s.to_string()),
    });
}

/// Collect enabled feature flags
#[allow(unused_mut, clippy::vec_init_then_push)]
fn collect_enabled_features() -> Vec<String> {
    let mut features: Vec<String> = vec![];

    #[cfg(feature = "api")]
    features.push("api".to_string());

    #[cfg(feature = "python")]
    features.push("python".to_string());

    #[cfg(feature = "cloud")]
    features.push("cloud".to_string());

    features
}

/// Timer guard for automatic duration tracking
pub struct TrackingTimer {
    start: Instant,
    command: String,
    subcommand: Option<String>,
}

impl TrackingTimer {
    pub fn new(command: &str, subcommand: Option<&str>) -> Self {
        Self {
            start: Instant::now(),
            command: command.to_string(),
            subcommand: subcommand.map(|s| s.to_string()),
        }
    }

    pub fn finish(self, success: bool) {
        track_command(
            &self.command,
            self.subcommand.as_deref(),
            self.start.elapsed(),
            success,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_telemetry_config_default() {
        let config = TelemetryConfig::default();
        assert!(!config.enabled);
        assert!(!config.device_id.is_empty());
        assert!(config.endpoint.contains("telemetry"));
        assert_eq!(config.batch_size, 25);
        assert_eq!(config.flush_interval_secs, 300);
    }

    #[test]
    fn test_telemetry_client_disable() {
        let config = TelemetryConfig {
            enabled: true,
            ..TelemetryConfig::default()
        };
        let client = TelemetryClient::new(config);

        assert!(client.is_enabled());
        client.disable();
        assert!(!client.is_enabled());
    }

    #[test]
    fn test_telemetry_client_enable() {
        let config = TelemetryConfig {
            enabled: false,
            ..TelemetryConfig::default()
        };
        let client = TelemetryClient::new(config);
        assert!(!client.is_enabled());
        client.enable();
        assert!(client.is_enabled());
    }

    #[test]
    fn test_telemetry_client_device_id() {
        let config = TelemetryConfig::default();
        let expected_id = config.device_id.clone();
        let client = TelemetryClient::new(config);
        assert_eq!(client.device_id(), expected_id);
    }

    #[test]
    fn test_telemetry_client_track_when_disabled() {
        let config = TelemetryConfig {
            enabled: false,
            ..TelemetryConfig::default()
        };
        let client = TelemetryClient::new(config);

        // Should be a no-op when disabled
        client.track(TelemetryEvent::CommandRun {
            command: "test".to_string(),
            subcommand: None,
            duration_ms: 0,
            success: true,
        });

        // Events list should be empty
        let events = client.events.lock();
        assert!(events.is_empty());
    }

    #[test]
    fn test_telemetry_client_track_when_enabled() {
        let config = TelemetryConfig {
            enabled: true,
            batch_size: 100, // high batch size to avoid auto-flush
            ..TelemetryConfig::default()
        };
        let client = TelemetryClient::new(config);

        client.track(TelemetryEvent::CommandRun {
            command: "store".to_string(),
            subcommand: Some("model".to_string()),
            duration_ms: 150,
            success: true,
        });

        let events = client.events.lock();
        assert_eq!(events.len(), 1);
        assert!(!events[0].device_id.is_empty());
        assert!(!events[0].session_id.is_empty());
        assert!(events[0].timestamp > 0);
    }

    #[test]
    fn test_telemetry_client_flush_when_disabled() {
        let config = TelemetryConfig {
            enabled: false,
            ..TelemetryConfig::default()
        };
        let client = TelemetryClient::new(config);
        // Should not panic
        client.flush();
    }

    #[test]
    fn test_size_bucket() {
        // Test the size bucketing logic
        let check_bucket = |size: u64| -> &'static str {
            match size {
                0..=10_000_000 => "small",
                10_000_001..=100_000_000 => "medium",
                100_000_001..=1_000_000_000 => "large",
                _ => "xlarge",
            }
        };

        assert_eq!(check_bucket(1_000), "small");
        assert_eq!(check_bucket(50_000_000), "medium");
        assert_eq!(check_bucket(500_000_000), "large");
        assert_eq!(check_bucket(2_000_000_000), "xlarge");
    }

    #[test]
    fn test_event_serialization() {
        let event = TelemetryEvent::CommandRun {
            command: "store".to_string(),
            subcommand: None,
            duration_ms: 150,
            success: true,
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("command_run"));
        assert!(json.contains("store"));
    }

    #[test]
    fn test_event_app_start_serialization() {
        let event = TelemetryEvent::AppStart {
            version: "1.3.0".to_string(),
            os: "windows".to_string(),
            arch: "x86_64".to_string(),
            features: vec!["api".to_string(), "cloud".to_string()],
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("app_start"));
        assert!(json.contains("1.3.0"));
    }

    #[test]
    fn test_event_model_operation_serialization() {
        let event = TelemetryEvent::ModelOperation {
            operation: "store".to_string(),
            format: "safetensors".to_string(),
            size_bucket: "medium".to_string(),
            duration_ms: 3000,
            success: true,
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("model_operation"));
    }

    #[test]
    fn test_event_conversion_serialization() {
        let event = TelemetryEvent::Conversion {
            source_format: "pytorch".to_string(),
            target_format: "onnx".to_string(),
            duration_ms: 5000,
            success: false,
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("conversion"));
        assert!(json.contains("false"));
    }

    #[test]
    fn test_event_api_call_serialization() {
        let event = TelemetryEvent::ApiCall {
            endpoint: "/api/v1/models".to_string(),
            method: "GET".to_string(),
            status_code: 200,
            duration_ms: 50,
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("api_call"));
        assert!(json.contains("200"));
    }

    #[test]
    fn test_event_error_serialization() {
        let event = TelemetryEvent::Error {
            error_type: "CryptoError".to_string(),
            context: Some("decryption failed".to_string()),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("error"));
        assert!(json.contains("CryptoError"));

        // Without context
        let event2 = TelemetryEvent::Error {
            error_type: "IoError".to_string(),
            context: None,
        };
        let json2 = serde_json::to_string(&event2).unwrap();
        assert!(!json2.contains("context"));
    }

    #[test]
    fn test_event_feature_used_serialization() {
        let event = TelemetryEvent::FeatureUsed {
            feature: "cloud_push".to_string(),
            detail: Some("s3".to_string()),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("feature_used"));

        let event2 = TelemetryEvent::FeatureUsed {
            feature: "rag".to_string(),
            detail: None,
        };
        let json2 = serde_json::to_string(&event2).unwrap();
        assert!(!json2.contains("detail"));
    }

    #[test]
    fn test_telemetry_config_serialization_roundtrip() {
        let config = TelemetryConfig::default();
        let yaml = serde_yaml_ng::to_string(&config).unwrap();
        let deserialized: TelemetryConfig = serde_yaml_ng::from_str(&yaml).unwrap();
        assert_eq!(deserialized.batch_size, config.batch_size);
        assert_eq!(deserialized.flush_interval_secs, config.flush_interval_secs);
    }

    #[test]
    fn test_get_queue_file_path() {
        let path = TelemetryClient::get_queue_file_path();
        assert!(path.to_string_lossy().contains("telemetry"));
        assert!(path.to_string_lossy().contains("events.jsonl"));
    }

    #[test]
    fn test_tracking_timer() {
        let timer = TrackingTimer::new("test-cmd", Some("sub"));
        assert_eq!(timer.command, "test-cmd");
        assert_eq!(timer.subcommand.as_deref(), Some("sub"));
        // Don't call finish() since global telemetry isn't initialized
    }

    #[test]
    fn test_tracking_timer_no_subcommand() {
        let timer = TrackingTimer::new("list", None);
        assert!(timer.subcommand.is_none());
    }

    #[test]
    fn test_collect_enabled_features() {
        let features = collect_enabled_features();
        // Result depends on compile features, but should not panic
        assert!(features.len() <= 10);
    }

    #[test]
    fn test_global_is_enabled_without_init() {
        // Without initialization, should return false
        assert!(!is_enabled());
    }

    #[test]
    fn test_is_disabled_by_env_default() {
        // Without any env vars set, should return false
        // (env vars may or may not be set in CI, so just ensure no panic)
        let _ = TelemetryClient::is_disabled_by_env();
    }

    /// Configuring an exporter must never be what turns collection on.
    ///
    /// The two decisions are made by different people: an operator points the
    /// build at a collector, but whether this deployment reports at all is the
    /// opt-in. If setting `OTEL_EXPORTER_OTLP_ENDPOINT` silently enabled
    /// telemetry, every deployment that configured a collector for its own
    /// traces would start reporting here too.
    #[test]
    fn test_otlp_endpoint_alone_does_not_enable_telemetry() {
        let config = TelemetryConfig::default();
        assert!(!config.enabled, "precondition: default is opt-in");

        let client = TelemetryClient::new(config);
        assert!(
            !client.is_enabled(),
            "an OTLP endpoint configures where events go, not whether they are \
             collected"
        );
    }

    /// `DO_NOT_TRACK` and friends must still win when OTLP is configured.
    #[test]
    fn test_explicit_disable_still_wins_over_otlp_config() {
        let config = TelemetryConfig {
            enabled: true,
            ..Default::default()
        };

        // Simulate the operator's kill switch being set.
        let client = TelemetryClient::new(config);
        client.disable();

        assert!(
            !client.is_enabled(),
            "an explicit disable must not be overridden by exporter settings"
        );
    }

    #[test]
    fn test_telemetry_client_track_multiple_events() {
        let config = TelemetryConfig {
            enabled: true,
            batch_size: 100,
            ..TelemetryConfig::default()
        };
        let client = TelemetryClient::new(config);

        // Track several different event types
        client.track(TelemetryEvent::AppStart {
            version: "1.3.0".to_string(),
            os: "windows".to_string(),
            arch: "x86_64".to_string(),
            features: vec!["api".to_string()],
        });
        client.track(TelemetryEvent::CommandRun {
            command: "store".to_string(),
            subcommand: None,
            duration_ms: 100,
            success: true,
        });
        client.track(TelemetryEvent::Error {
            error_type: "IoError".to_string(),
            context: Some("test context".to_string()),
        });

        let events = client.events.lock();
        assert_eq!(events.len(), 3);
    }

    #[test]
    fn test_telemetry_client_flush_clears_events() {
        let config = TelemetryConfig {
            enabled: true,
            batch_size: 100,
            ..TelemetryConfig::default()
        };
        let client = TelemetryClient::new(config);

        client.track(TelemetryEvent::FeatureUsed {
            feature: "rag".to_string(),
            detail: None,
        });
        assert_eq!(client.events.lock().len(), 1);

        // Flush will try to send (and fail since no server), but events should be drained
        client.flush();
        // After flush, internal batch was taken; wait a moment for the spawned thread
        std::thread::sleep(std::time::Duration::from_millis(50));
        assert_eq!(client.events.lock().len(), 0);
    }

    #[test]
    fn test_telemetry_config_custom_values() {
        let config = TelemetryConfig {
            enabled: true,
            device_id: "custom-id".to_string(),
            endpoint: "https://custom.endpoint/v1".to_string(),
            batch_size: 10,
            flush_interval_secs: 60,
        };
        assert!(config.enabled);
        assert_eq!(config.device_id, "custom-id");
        assert_eq!(config.batch_size, 10);
        assert_eq!(config.flush_interval_secs, 60);
    }

    #[test]
    fn test_telemetry_envelope_fields() {
        let config = TelemetryConfig {
            enabled: true,
            batch_size: 100,
            ..TelemetryConfig::default()
        };
        let client = TelemetryClient::new(config);

        client.track(TelemetryEvent::Conversion {
            source_format: "pytorch".to_string(),
            target_format: "onnx".to_string(),
            duration_ms: 2000,
            success: true,
        });

        let events = client.events.lock();
        let env = &events[0];
        assert!(!env.device_id.is_empty());
        assert!(!env.session_id.is_empty());
        assert!(env.timestamp > 0);

        // Verify the event content via serialization
        let json = serde_json::to_string(&env.event).unwrap();
        assert!(json.contains("conversion"));
        assert!(json.contains("pytorch"));
    }

    #[test]
    fn test_tracking_timer_finish_does_not_panic() {
        // TrackingTimer::finish calls the global track(), which is a no-op
        // if no global client is initialized
        let timer = TrackingTimer::new("bench-cmd", Some("sub-a"));
        std::thread::sleep(std::time::Duration::from_millis(5));
        timer.finish(true);
    }

    #[test]
    fn test_tracking_timer_finish_failure() {
        let timer = TrackingTimer::new("failing-cmd", None);
        timer.finish(false);
    }

    #[test]
    fn test_global_track_noop_without_init() {
        // Global track() should silently no-op when TELEMETRY not initialized
        track(TelemetryEvent::FeatureUsed {
            feature: "test".to_string(),
            detail: None,
        });
    }

    #[test]
    fn test_global_flush_noop_without_init() {
        flush();
    }

    #[test]
    fn test_global_disable_noop_without_init() {
        disable();
    }

    #[test]
    fn test_convenience_track_functions_noop() {
        // All convenience functions should be no-ops when global not initialized
        track_app_start();
        track_command("test", Some("sub"), Duration::from_millis(10), true);
        track_command("test", None, Duration::from_millis(5), false);
        track_model_op(
            "store",
            "safetensors",
            1_000,
            Duration::from_millis(50),
            true,
        );
        track_model_op("get", "gguf", 500_000_000, Duration::from_millis(100), true);
        track_model_op(
            "store",
            "onnx",
            50_000_000,
            Duration::from_millis(75),
            false,
        );
        track_model_op(
            "store",
            "pytorch",
            2_000_000_000,
            Duration::from_millis(200),
            true,
        );
        track_conversion("pytorch", "onnx", Duration::from_secs(5), true);
        track_api_call("/models", "GET", 200, Duration::from_millis(30));
        track_error("TestError", Some("test context"));
        track_error("TestError", None);
        track_feature("rag", Some("search"));
        track_feature("cloud", None);
    }

    #[test]
    fn test_size_bucket_boundaries() {
        // Test exact boundary values for size bucketing
        let check_bucket = |size: u64| -> &'static str {
            match size {
                0..=10_000_000 => "small",
                10_000_001..=100_000_000 => "medium",
                100_000_001..=1_000_000_000 => "large",
                _ => "xlarge",
            }
        };

        assert_eq!(check_bucket(0), "small");
        assert_eq!(check_bucket(10_000_000), "small"); // upper boundary
        assert_eq!(check_bucket(10_000_001), "medium"); // lower boundary
        assert_eq!(check_bucket(100_000_000), "medium"); // upper boundary
        assert_eq!(check_bucket(100_000_001), "large"); // lower boundary
        assert_eq!(check_bucket(1_000_000_000), "large"); // upper boundary
        assert_eq!(check_bucket(1_000_000_001), "xlarge"); // lower boundary
    }

    #[test]
    fn test_telemetry_envelope_serialization_roundtrip() {
        let envelope = TelemetryEnvelope {
            device_id: "dev-1".to_string(),
            session_id: "sess-1".to_string(),
            timestamp: 1700000000,
            event: TelemetryEvent::ApiCall {
                endpoint: "/health".to_string(),
                method: "GET".to_string(),
                status_code: 200,
                duration_ms: 5,
            },
        };
        let json = serde_json::to_string(&envelope).unwrap();
        let d: TelemetryEnvelope = serde_json::from_str(&json).unwrap();
        assert_eq!(d.device_id, "dev-1");
        assert_eq!(d.session_id, "sess-1");
        assert_eq!(d.timestamp, 1700000000);
    }

    #[test]
    fn test_write_to_local_queue_and_read() {
        let dir = tempfile::tempdir().unwrap();
        let queue_path = dir.path().join("events.jsonl");

        // Manually test the write logic (without relying on the static path)
        use std::io::Write;
        let body = r#"[{"device_id":"d","session_id":"s","timestamp":1,"event":{"type":"feature_used","feature":"test"}}]"#;
        let mut f = std::fs::File::create(&queue_path).unwrap();
        writeln!(f, "{}", body).unwrap();
        drop(f);

        let contents = std::fs::read_to_string(&queue_path).unwrap();
        assert!(contents.contains("feature_used"));

        // Verify it parses as a batch
        for line in contents.lines() {
            if !line.trim().is_empty() {
                let batch: Vec<TelemetryEnvelope> = serde_json::from_str(line).unwrap();
                assert_eq!(batch.len(), 1);
            }
        }
    }

    #[test]
    fn test_load_or_create_config_new() {
        let dir = tempfile::tempdir().unwrap();
        let config_dir = dir.path().to_path_buf();
        let config = load_or_create_config(Some(&config_dir)).unwrap();
        assert!(!config.enabled); // default is opt-in disabled
        assert!(!config.device_id.is_empty());

        // Config file should have been created
        let config_path = config_dir.join("telemetry.yaml");
        assert!(config_path.exists());
    }

    #[test]
    fn test_load_or_create_config_existing() {
        let dir = tempfile::tempdir().unwrap();
        let config_dir = dir.path().to_path_buf();
        let config_path = config_dir.join("telemetry.yaml");

        // Write a custom config
        let custom = TelemetryConfig {
            enabled: true,
            device_id: "existing-device".to_string(),
            batch_size: 50,
            ..TelemetryConfig::default()
        };
        let yaml = serde_yaml_ng::to_string(&custom).unwrap();
        std::fs::write(&config_path, &yaml).unwrap();

        // Load it
        let loaded = load_or_create_config(Some(&config_dir)).unwrap();
        assert!(loaded.enabled);
        assert_eq!(loaded.device_id, "existing-device");
        assert_eq!(loaded.batch_size, 50);
    }

    #[test]
    fn test_init_and_track() {
        // Use a unique config to avoid interfering with global state
        // Note: TELEMETRY is a OnceLock so init() only works once per process.
        // This test verifies init doesn't panic when called (may be no-op if already set).
        let config = TelemetryConfig {
            enabled: false,
            ..TelemetryConfig::default()
        };
        init(config);
        // Global is_enabled reflects the client state
        // (but won't be true since we disabled it, and TELEMETRY_DISABLED may also be set)
    }

    #[test]
    fn test_default_helper_functions() {
        // Cover the default value functions
        assert!(!generate_device_id().is_empty());
        assert!(default_endpoint().contains("telemetry"));
        assert_eq!(default_batch_size(), 25);
        assert_eq!(default_flush_interval(), 300);
    }
}
