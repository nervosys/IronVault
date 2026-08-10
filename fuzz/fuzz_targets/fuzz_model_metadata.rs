#![no_main]
use libfuzzer_sys::fuzz_target;

use ironvault::formats::{ModelFormat, ModelMetadata};

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        // Fuzz ModelMetadata construction with arbitrary strings
        // Split input into name and format extension
        let parts: Vec<&str> = s.splitn(2, '\n').collect();
        let name = parts.first().copied().unwrap_or("");
        let ext = parts.get(1).copied().unwrap_or("safetensors");

        let format = ModelFormat::from_extension(ext);
        let metadata = ModelMetadata::new(name.to_string(), format);

        // Exercise builder methods with fuzzed data
        let _ = metadata
            .with_description(name.to_string())
            .with_framework(ext.to_string())
            .with_task(name.to_string());
    }
});
