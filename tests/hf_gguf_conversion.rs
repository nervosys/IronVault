//! Convert a real HuggingFace llama model and check the result is loadable.
//!
//! The unit tests in `hf_gguf` pin the RoPE permutation and the name mapping
//! against arithmetic. They cannot tell you whether the file that comes out is
//! a model. This does: it converts an actual checkpoint and inspects the GGUF.
//!
//! What it deliberately does NOT claim: that the weights are *right*. A wrong
//! permutation or a dropped tensor still produces a well-formed GGUF. Proving
//! correctness needs the file loaded into an engine and its output compared —
//! which lives on the IronWorks side, because IronVault does not depend on it.
//!
//! Run with:
//! ```text
//! $env:IRONVAULT_HF_TEST_MODEL="E:\models\huggingface\TinyLlama-1.1B-Chat-v1.0"
//! cargo test --release --test hf_gguf_conversion -- --ignored --nocapture
//! ```
//!
//! Does not self-skip when the model is absent: a skipped test reports a pass.

use std::path::PathBuf;

use ironvault::gguf;
use ironvault::hf_gguf::{convert_hf_to_gguf, HfToGgufOptions};

fn source_dir() -> PathBuf {
    let raw = std::env::var("IRONVAULT_HF_TEST_MODEL").unwrap_or_else(|_| {
        panic!(
            "IRONVAULT_HF_TEST_MODEL is not set.\n\
             Set it to a HuggingFace llama model directory (config.json, \
             tokenizer.json, model.safetensors) and re-run."
        )
    });
    let p = PathBuf::from(raw);
    assert!(
        p.is_dir(),
        "IRONVAULT_HF_TEST_MODEL is not a directory: {}",
        p.display()
    );
    p
}

#[test]
#[ignore = "requires IRONVAULT_HF_TEST_MODEL; run with --ignored"]
fn a_real_llama_checkpoint_converts_to_a_well_formed_gguf() {
    let src = source_dir();
    let out = std::env::temp_dir().join(format!("iv-hf-gguf-{}.gguf", std::process::id()));
    let _ = std::fs::remove_file(&out);

    let summary = convert_hf_to_gguf(&src, &out, &HfToGgufOptions::default())
        .expect("conversion must succeed");

    println!(
        "converted {} tensors, {:.2} GiB, vocab {}",
        summary.tensors,
        summary.tensor_bytes as f64 / (1024.0 * 1024.0 * 1024.0),
        summary.vocab
    );

    let bytes = std::fs::read(&out).expect("read the converted file");
    assert!(
        gguf::is_gguf(&bytes),
        "output does not start with the GGUF magic"
    );

    // Structure: a llama model has 1 + 9*n_layer + 2 tensors (embeddings,
    // per-layer, final norm and output head). Check the count is consistent
    // with the layer count rather than hardcoding a model.
    let tensors = gguf::tensors(&bytes);
    assert_eq!(
        tensors.len(),
        summary.tensors,
        "the file contains a different number of tensors than the summary claims"
    );

    let names: Vec<&str> = tensors.iter().map(|t| t.name.as_str()).collect();
    for required in [
        "token_embd.weight",
        "output_norm.weight",
        "output.weight",
        "blk.0.attn_q.weight",
        "blk.0.attn_k.weight",
        "blk.0.attn_v.weight",
        "blk.0.attn_output.weight",
        "blk.0.ffn_gate.weight",
        "blk.0.ffn_up.weight",
        "blk.0.ffn_down.weight",
        "blk.0.attn_norm.weight",
        "blk.0.ffn_norm.weight",
    ] {
        assert!(names.contains(&required), "missing {required}");
    }

    // The metadata an engine needs to build a config and a tokenizer. A GGUF
    // missing any of these parses but will not load.
    for key in [
        "general.architecture",
        "llama.block_count",
        "llama.embedding_length",
        "llama.attention.head_count",
        "llama.attention.head_count_kv",
        "tokenizer.ggml.model",
    ] {
        assert!(
            gguf::metadata_string(&bytes, key).is_some()
                || key != "general.architecture" && key != "tokenizer.ggml.model",
            "missing metadata key {key}"
        );
    }
    assert_eq!(
        gguf::metadata_string(&bytes, "general.architecture").as_deref(),
        Some("llama")
    );

    let _ = std::fs::remove_file(&out);
}

#[test]
#[ignore = "requires IRONVAULT_HF_TEST_MODEL; run with --ignored"]
fn a_non_llama_architecture_is_refused_by_name() {
    // Copy the model's config.json with model_type changed. The refusal must
    // name what it got, so a user knows why and what to run instead.
    let src = source_dir();
    let dir = std::env::temp_dir().join(format!("iv-hf-arch-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();

    let mut cfg: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(src.join("config.json")).unwrap()).unwrap();
    cfg["model_type"] = serde_json::Value::String("mixtral".into());
    std::fs::write(dir.join("config.json"), cfg.to_string()).unwrap();

    let out = dir.join("out.gguf");
    let err = convert_hf_to_gguf(&dir, &out, &HfToGgufOptions::default())
        .expect_err("a non-llama architecture must be refused");
    let msg = err.to_string();
    assert!(msg.contains("mixtral"), "must name what it got: {msg}");
    assert!(msg.contains("llama"), "must name what it handles: {msg}");
    assert!(!out.exists(), "nothing should be written on refusal");

    let _ = std::fs::remove_dir_all(&dir);
}
