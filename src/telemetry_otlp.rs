//! OTLP export for telemetry events (`otel` feature).
//!
//! Maps [`TelemetryEvent`](crate::telemetry::TelemetryEvent) onto OpenTelemetry
//! log records and ships them to a collector over OTLP/HTTP.
//!
//! ## What is exported
//!
//! Only the fields already present on the event types, which are bucketed and
//! discriminant-shaped by construction — no file paths, model names, or model
//! contents. See the privacy notes on [`crate::telemetry`]; this module is a
//! transport and does not widen what is collected.
//!
//! ## Configuration
//!
//! Entirely from the standard `OTEL_*` environment variables — see
//! [`OtlpSettings`](crate::telemetry::OtlpSettings). Nothing here is baked into the binary, and in particular
//! the bearer token in `OTEL_EXPORTER_OTLP_HEADERS` is read from the
//! environment of the process the operator started, never from the source
//! tree.

use opentelemetry::logs::{AnyValue, LogRecord as _, Logger as _, LoggerProvider as _, Severity};
use opentelemetry::KeyValue;
use opentelemetry_otlp::{Protocol, WithExportConfig};
use opentelemetry_sdk::logs::SdkLoggerProvider;
use opentelemetry_sdk::Resource;

use crate::telemetry::{OtlpProtocol, OtlpSettings, TelemetryEnvelope, TelemetryEvent};

/// Build a provider and export one batch, then flush.
///
/// A provider is built per batch rather than cached in a `static`. Batches are
/// infrequent (they are gated on a flush interval and a batch size), and a
/// long-lived global provider would hold a connection open for the life of a
/// CLI process that emits two events and exits.
pub(crate) fn export(
    events: &[TelemetryEnvelope],
    settings: &OtlpSettings,
) -> std::result::Result<(), String> {
    if events.is_empty() {
        return Ok(());
    }

    let protocol = match settings.protocol {
        OtlpProtocol::HttpBinary => Protocol::HttpBinary,
        OtlpProtocol::HttpJson => Protocol::HttpJson,
    };

    // The exporter builder reads OTEL_EXPORTER_OTLP_HEADERS itself, so the
    // bearer token never passes through this crate's own types and cannot end
    // up in a Debug print or an error message.
    let exporter = opentelemetry_otlp::LogExporter::builder()
        .with_http()
        .with_endpoint(&settings.endpoint)
        .with_protocol(protocol)
        .build()
        .map_err(|e| format!("Failed to build OTLP exporter: {e}"))?;

    let resource = Resource::builder()
        .with_service_name(settings.service_name.clone())
        .with_attribute(KeyValue::new("service.version", env!("CARGO_PKG_VERSION")))
        .build();

    // Simple rather than batch: this crate already batches, and the simple
    // processor exports on the calling thread, which is the detached thread
    // `send_batch` spawned. A batch processor would need a runtime that a CLI
    // process does not have.
    let provider = SdkLoggerProvider::builder()
        .with_simple_exporter(exporter)
        .with_resource(resource)
        .build();

    let logger = provider.logger("ironvault");

    for envelope in events {
        let mut record = logger.create_log_record();
        record.set_severity_number(severity_for(&envelope.event));
        record.set_severity_text(severity_text(&envelope.event));
        record.set_event_name(event_name(&envelope.event));

        record.add_attribute("device.id", envelope.device_id.clone());
        record.add_attribute("session.id", envelope.session_id.clone());
        record.add_attribute(
            "timestamp.unix",
            i64::try_from(envelope.timestamp).unwrap_or(-1),
        );

        for (key, value) in attributes(&envelope.event) {
            record.add_attribute(key, value);
        }

        logger.emit(record);
    }

    provider
        .force_flush()
        .map_err(|e| format!("OTLP flush failed: {e}"))?;

    Ok(())
}

fn event_name(event: &TelemetryEvent) -> &'static str {
    match event {
        TelemetryEvent::AppStart { .. } => "app_start",
        TelemetryEvent::CommandRun { .. } => "command_run",
        TelemetryEvent::ModelOperation { .. } => "model_operation",
        TelemetryEvent::Conversion { .. } => "conversion",
        TelemetryEvent::ApiCall { .. } => "api_call",
        TelemetryEvent::Error { .. } => "error",
        TelemetryEvent::FeatureUsed { .. } => "feature_used",
    }
}

fn severity_for(event: &TelemetryEvent) -> Severity {
    match event {
        TelemetryEvent::Error { .. } => Severity::Error,
        TelemetryEvent::CommandRun { success: false, .. }
        | TelemetryEvent::ModelOperation { success: false, .. }
        | TelemetryEvent::Conversion { success: false, .. } => Severity::Warn,
        _ => Severity::Info,
    }
}

fn severity_text(event: &TelemetryEvent) -> &'static str {
    match severity_for(event) {
        Severity::Error => "ERROR",
        Severity::Warn => "WARN",
        _ => "INFO",
    }
}

/// Values are typed rather than stringified so a collector can aggregate
/// durations without parsing.
fn attributes(event: &TelemetryEvent) -> Vec<(&'static str, AnyValue)> {
    use AnyValue as Value;

    match event {
        TelemetryEvent::AppStart {
            version,
            os,
            arch,
            features,
        } => vec![
            ("app.version", Value::from(version.clone())),
            ("os.type", Value::from(os.clone())),
            ("host.arch", Value::from(arch.clone())),
            ("app.features", Value::from(features.join(","))),
        ],
        TelemetryEvent::CommandRun {
            command,
            subcommand,
            duration_ms,
            success,
        } => {
            let mut attrs = vec![
                ("command.name", Value::from(command.clone())),
                ("duration.ms", Value::from(*duration_ms as i64)),
                ("outcome.success", Value::from(*success)),
            ];
            if let Some(sub) = subcommand {
                attrs.push(("command.subcommand", Value::from(sub.clone())));
            }
            attrs
        }
        TelemetryEvent::ModelOperation {
            operation,
            format,
            size_bucket,
            duration_ms,
            success,
        } => vec![
            ("model.operation", Value::from(operation.clone())),
            ("model.format", Value::from(format.clone())),
            // Bucketed at the call site; an exact size is closer to
            // identifying a specific model than a bucket is.
            ("model.size_bucket", Value::from(size_bucket.clone())),
            ("duration.ms", Value::from(*duration_ms as i64)),
            ("outcome.success", Value::from(*success)),
        ],
        TelemetryEvent::Conversion {
            source_format,
            target_format,
            duration_ms,
            success,
        } => vec![
            (
                "conversion.source_format",
                Value::from(source_format.clone()),
            ),
            (
                "conversion.target_format",
                Value::from(target_format.clone()),
            ),
            ("duration.ms", Value::from(*duration_ms as i64)),
            ("outcome.success", Value::from(*success)),
        ],
        TelemetryEvent::ApiCall {
            endpoint,
            method,
            status_code,
            duration_ms,
        } => vec![
            // Route template, not a resolved path — see `track_api_call`.
            ("http.route", Value::from(endpoint.clone())),
            ("http.method", Value::from(method.clone())),
            ("http.status_code", Value::from(i64::from(*status_code))),
            ("duration.ms", Value::from(*duration_ms as i64)),
        ],
        TelemetryEvent::Error {
            error_type,
            context,
        } => {
            let mut attrs = vec![("error.type", Value::from(error_type.clone()))];
            if let Some(ctx) = context {
                attrs.push(("error.context", Value::from(ctx.clone())));
            }
            attrs
        }
        TelemetryEvent::FeatureUsed { feature, detail } => {
            let mut attrs = vec![("feature.name", Value::from(feature.clone()))];
            if let Some(detail) = detail {
                attrs.push(("feature.detail", Value::from(detail.clone())));
            }
            attrs
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_names_are_distinct() {
        let events = sample_events();
        let mut names: Vec<&str> = events.iter().map(event_name).collect();
        let total = names.len();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), total, "each variant needs its own event name");
    }

    #[test]
    fn test_failures_are_not_reported_as_info() {
        let failed = TelemetryEvent::CommandRun {
            command: "store".into(),
            subcommand: None,
            duration_ms: 5,
            success: false,
        };
        assert_eq!(severity_for(&failed), Severity::Warn);

        let ok = TelemetryEvent::CommandRun {
            command: "store".into(),
            subcommand: None,
            duration_ms: 5,
            success: true,
        };
        assert_eq!(severity_for(&ok), Severity::Info);
    }

    #[test]
    fn test_every_variant_produces_attributes() {
        for event in sample_events() {
            assert!(
                !attributes(&event).is_empty(),
                "{} produced no attributes",
                event_name(&event)
            );
        }
    }

    /// The exporter must never invent an attribute carrying a path or a model
    /// name. This pins the emitted key set so adding one is a deliberate edit
    /// to this list rather than an accident.
    #[test]
    fn test_attribute_keys_are_the_approved_set() {
        let approved = [
            "app.version",
            "os.type",
            "host.arch",
            "app.features",
            "command.name",
            "command.subcommand",
            "duration.ms",
            "outcome.success",
            "model.operation",
            "model.format",
            "model.size_bucket",
            "conversion.source_format",
            "conversion.target_format",
            "http.route",
            "http.method",
            "http.status_code",
            "error.type",
            "error.context",
            "feature.name",
            "feature.detail",
        ];

        for event in sample_events() {
            for (key, _) in attributes(&event) {
                assert!(
                    approved.contains(&key),
                    "unapproved telemetry attribute {key:?} — if this is \
                     intentional, confirm it cannot carry a file path or model \
                     name and add it to the approved list"
                );
            }
        }
    }

    fn sample_events() -> Vec<TelemetryEvent> {
        vec![
            TelemetryEvent::AppStart {
                version: "4.0.0".into(),
                os: "linux".into(),
                arch: "x86_64".into(),
                features: vec!["api".into()],
            },
            TelemetryEvent::CommandRun {
                command: "store".into(),
                subcommand: Some("model".into()),
                duration_ms: 12,
                success: true,
            },
            TelemetryEvent::ModelOperation {
                operation: "store".into(),
                format: "safetensors".into(),
                size_bucket: "large".into(),
                duration_ms: 30,
                success: true,
            },
            TelemetryEvent::Conversion {
                source_format: "pytorch".into(),
                target_format: "safetensors".into(),
                duration_ms: 40,
                success: true,
            },
            TelemetryEvent::ApiCall {
                endpoint: "/models/:name".into(),
                method: "GET".into(),
                status_code: 200,
                duration_ms: 3,
            },
            TelemetryEvent::Error {
                error_type: "integrity".into(),
                context: Some("checksum_mismatch".into()),
            },
            TelemetryEvent::FeatureUsed {
                feature: "signing".into(),
                detail: None,
            },
        ]
    }
}
