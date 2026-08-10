#![no_main]
use libfuzzer_sys::fuzz_target;

use ironvault::conversion::ConversionPipeline;
use ironvault::formats::ModelFormat;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        // Fuzz format string parsing — should never panic
        let _ = ModelFormat::from_extension(s);

        // Fuzz conversion path lookup with arbitrary format pairs
        let parts: Vec<&str> = s.splitn(2, '\n').collect();
        if parts.len() == 2 {
            let src = ModelFormat::from_extension(parts[0]);
            let tgt = ModelFormat::from_extension(parts[1]);
            let pipeline = ConversionPipeline::with_builtins();
            let _ = pipeline.find_path(&src, &tgt);
        }
    }
});
