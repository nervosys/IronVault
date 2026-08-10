#![no_main]
use libfuzzer_sys::fuzz_target;

use ironvault::scanning::PickleScanner;

fuzz_target!(|data: &[u8]| {
    // Fuzz the pickle scanner with arbitrary byte sequences.
    // The scanner must never panic regardless of input.
    let report = PickleScanner::scan_bytes(data, "fuzz-input");

    // Sanity checks on the report structure
    assert!(!report.file_path.is_empty());
    assert_eq!(report.file_size, data.len() as u64);

    // If safe, there should be no critical/warning findings
    if report.safe {
        for f in &report.findings {
            assert!(
                f.severity != ironvault::scanning::Severity::Critical
                    && f.severity != ironvault::scanning::Severity::Warning,
                "report marked safe but has critical/warning finding"
            );
        }
    }
});
