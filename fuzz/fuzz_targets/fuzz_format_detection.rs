#![no_main]
use libfuzzer_sys::fuzz_target;

use ironvault::formats::ModelFormat;

fuzz_target!(|data: &[u8]| {
    // Fuzz format detection from arbitrary extensions
    if let Ok(ext) = std::str::from_utf8(data) {
        // from_extension should never panic regardless of input
        let _ = ModelFormat::from_extension(ext);
    }
});
