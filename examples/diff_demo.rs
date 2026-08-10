//! Example: Model version diffing at the tensor level
//!
//! Run with: cargo run --example diff_demo

use ironvault::diff::ModelDiffer;
use std::path::Path;

fn main() -> ironvault::Result<()> {
    println!("=== IronVault Diff Example ===\n");

    // 1. Diff two files (if they exist)
    println!("1. Diffing model files...");
    let left = Path::new("model_v1.safetensors");
    let right = Path::new("model_v2.safetensors");

    if left.exists() && right.exists() {
        let diff = ModelDiffer::diff_files(left, right, "v1", "v2")?;
        println!("{}", diff.display());
        println!("   Added tensors: {}", diff.added_tensors.len());
        println!("   Removed tensors: {}", diff.removed_tensors.len());
        println!("   Changed tensors: {}", diff.changed_tensors.len());
        println!("   Unchanged: {}", diff.unchanged_count);
        println!("   Size delta: {} bytes", diff.size_delta);
        println!("   Param delta: {}", diff.param_delta);
    } else {
        println!("   (No model files found — showing CLI usage instead)");
        println!();
        println!("   Compare two files:");
        println!("     iv diff model_v1.safetensors model_v2.safetensors");
        println!();
        println!("   Compare vault versions:");
        println!("     iv diff mymodel@v1 mymodel@v2");
        println!();
        println!("   JSON output:");
        println!("     iv diff left.gguf right.gguf --format json");
    }
    println!();

    // 2. Supported formats
    println!("2. Supported diff formats:");
    println!("   SafeTensors — Full tensor-level diff (shapes, dtypes, params)");
    println!("   GGUF        — Full tensor-level diff from metadata headers");
    println!("   Other       — File size comparison only\n");

    // 3. Diff output fields
    println!("3. Diff output includes:");
    println!("   • Added/removed/changed tensors with shape and dtype details");
    println!("   • Parameter count deltas per tensor");
    println!("   • Overall size change percentage");
    println!("   • Summary statistics\n");

    println!("=== Diff example complete ===");
    Ok(())
}
