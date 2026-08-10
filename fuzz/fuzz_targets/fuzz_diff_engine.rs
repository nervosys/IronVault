#![no_main]
use libfuzzer_sys::fuzz_target;

use ironvault::diff::ModelDiffer;

fuzz_target!(|data: &[u8]| {
    // Split the input in half to create two "model" files to diff.
    // The differ must never panic regardless of input content.
    let mid = data.len() / 2;
    let (left, right) = data.split_at(mid);

    // Test with a common format extension
    let _ = ModelDiffer::diff_bytes(left, right, "left", "right", "safetensors", "safetensors");

    // Also test with mismatched formats
    let _ = ModelDiffer::diff_bytes(left, right, "left", "right", "gguf", "pt");

    // Test identical inputs
    let _ = ModelDiffer::diff_bytes(data, data, "same", "same", "bin", "bin");
});
