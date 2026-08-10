//! Example: Benchmark metadata storage and retrieval
//!
//! Run with: cargo run --example benchmark_demo

use ironvault::benchmark::{BenchmarkRecord, BenchmarkStore};
use std::collections::BTreeMap;

fn main() -> ironvault::Result<()> {
    println!("=== IronVault Benchmark Example ===\n");

    // 1. Create a benchmark record
    println!("1. Creating benchmark record...");
    let mut record = BenchmarkRecord::new("llama2-finetuned", 1);
    println!("   Model: {}", record.model_name);
    println!("   Version: {}\n", record.version);

    // 2. Add benchmark results
    println!("2. Adding benchmark results...");
    record.add_result("mmlu", 72.5, "percent", true);
    record.add_result("humaneval", 48.2, "percent", true);
    record.add_result("perplexity", 6.3, "ppl", false);
    record.add_result("inference_latency", 42.0, "ms", false);
    println!("   ✓ Added 4 benchmark results\n");

    // 3. Add a detailed result with metadata
    println!("3. Adding detailed result...");
    let mut meta = BTreeMap::new();
    meta.insert("few_shot".to_string(), "5".to_string());
    meta.insert("temperature".to_string(), "0.0".to_string());
    record.add_detailed_result("gsm8k", 65.0, "percent", true, Some("GSM8K"), meta);
    println!("   ✓ Added GSM8K with dataset and metadata\n");

    // 4. Query results
    println!("4. Querying results...");
    if let Some(mmlu) = record.get_result("mmlu") {
        println!(
            "   MMLU: {} {} (higher is better: {})",
            mmlu.score, mmlu.unit, mmlu.higher_is_better
        );
    }
    if let Some(ppl) = record.get_result("perplexity") {
        println!(
            "   Perplexity: {} {} (higher is better: {})",
            ppl.score, ppl.unit, ppl.higher_is_better
        );
    }
    println!();

    // 5. Display formatted output
    println!("5. Formatted display:");
    println!("{}", record.display());

    // 6. Use BenchmarkStore for persistence
    println!("6. Persistence with BenchmarkStore...");
    let temp_dir = std::env::temp_dir().join("aim_bench_demo");
    let store = BenchmarkStore::new(&temp_dir)?;
    store.save(&record)?;
    println!("   ✓ Saved to: {}", temp_dir.display());

    let loaded = store.list_for_model("llama2-finetuned")?;
    println!(
        "   ✓ Found {} record(s) for llama2-finetuned\n",
        loaded.len()
    );

    // Cleanup
    let _ = std::fs::remove_dir_all(&temp_dir);

    // 7. CLI commands
    println!("7. CLI usage:");
    println!(
        "   iv benchmark add my-model --version 1 --benchmark mmlu --score 72.5 --unit percent"
    );
    println!("   iv benchmark show my-model --version 1 --format json\n");

    println!("=== Benchmark example complete ===");
    Ok(())
}
