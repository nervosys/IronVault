#![no_main]
use libfuzzer_sys::fuzz_target;

use ironvault::model_card::ModelCard;

fuzz_target!(|data: &[u8]| {
    // Try to parse arbitrary bytes as JSON and YAML model cards.
    // The parser must never panic — only return Err for invalid input.

    if let Ok(text) = std::str::from_utf8(data) {
        // Attempt JSON parsing
        let _ = ModelCard::from_json(text);

        // Attempt YAML parsing
        let _ = ModelCard::from_yaml(text);
    }

    // Also try with lossy UTF-8 conversion (covers non-UTF-8 paths)
    let lossy = String::from_utf8_lossy(data);
    let _ = ModelCard::from_json(&lossy);
    let _ = ModelCard::from_yaml(&lossy);
});
