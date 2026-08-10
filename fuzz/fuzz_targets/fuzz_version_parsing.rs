#![no_main]
use libfuzzer_sys::fuzz_target;

use ironvault::version::ModelVersion;

fuzz_target!(|data: &[u8]| {
    // Fuzz JSON deserialization of ModelVersion.
    // Should never panic regardless of input.
    if let Ok(json_str) = std::str::from_utf8(data) {
        let _ = serde_json::from_str::<ModelVersion>(json_str);
    }
});
