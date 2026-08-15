//! Convert a HuggingFace llama checkpoint to GGUF.
//!
//! ```text
//! cargo run --release --example hf_to_gguf -- <src-dir> <out.gguf> [f16|bf16|f32]
//! ```
//!
//! Quantize the result with `llama-quantize`; this project cannot encode
//! K-quants (see `gguf_quant`).

use std::path::PathBuf;

use ironvault::gguf_quant::GGMLQuantizationType;
use ironvault::hf_gguf::{convert_hf_to_gguf, HfToGgufOptions};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!(
            "usage: hf_to_gguf <hf-model-dir> <out.gguf> [f16|bf16|f32]\n\
             \n\
             The directory needs config.json, tokenizer.json, and either\n\
             model.safetensors or model.safetensors.index.json."
        );
        std::process::exit(2);
    }

    let src = PathBuf::from(&args[1]);
    let dst = PathBuf::from(&args[2]);
    let out_type = match args.get(3).map(String::as_str).unwrap_or("f16") {
        "f16" => GGMLQuantizationType::F16,
        "bf16" => GGMLQuantizationType::BF16,
        "f32" => GGMLQuantizationType::F32,
        other => {
            eprintln!("unknown output type {other:?}; use f16, bf16 or f32");
            std::process::exit(2);
        }
    };

    let started = std::time::Instant::now();
    let summary = convert_hf_to_gguf(&src, &dst, &HfToGgufOptions { out_type })?;
    let secs = started.elapsed().as_secs_f64();

    println!(
        "{} -> {}\n  {} tensors, {:.2} GiB, vocab {}, {:.1}s",
        src.display(),
        dst.display(),
        summary.tensors,
        summary.tensor_bytes as f64 / (1024.0 * 1024.0 * 1024.0),
        summary.vocab,
        secs
    );
    Ok(())
}
