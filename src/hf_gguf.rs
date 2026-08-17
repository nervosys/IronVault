//! HuggingFace (safetensors) → GGUF conversion.
//!
//! IronWorks runs GGUF and nothing else, because it is the only format that
//! stores weights as quantized blocks in the layout the fused kernels consume.
//! Producing that file is IronVault's job: the vault owns models at rest, the
//! engine owns running them.
//!
//! # Scope, stated plainly
//!
//! **Llama-architecture models only, to `F16`/`BF16`/`F32`.** Any other
//! `model_type` is refused by name rather than converted approximately —
//! `convert_hf_to_gguf.py` is thousands of lines of per-architecture logic and
//! pretending to cover it here would produce files that load and generate
//! wrong text.
//!
//! **No K-quants.** Nothing in this project can encode `Q4_K`/`Q6_K` yet (see
//! `gguf_quant`), so quantize the `F16` output with `llama-quantize`. That
//! matches how llama.cpp splits the job: convert, then quantize.
//!
//! # Memory
//!
//! Streams. The safetensors header gives every shape up front, so all tensors
//! are declared before any data is read, then converted and written one at a
//! time. Peak residency is the largest single tensor, not the model.

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufWriter, Read, Seek, SeekFrom};
use std::path::Path;

use gguf_quant::{quant, GGMLQuantizationType, GGUFValue, GgufStreamWriter};

use crate::error::{Result, VaultError};

/// What the caller asks for.
#[derive(Debug, Clone)]
pub struct HfToGgufOptions {
    /// Output tensor type. `F16` is the conventional convert-then-quantize
    /// intermediate.
    pub out_type: GGMLQuantizationType,
}

impl Default for HfToGgufOptions {
    fn default() -> Self {
        Self {
            out_type: GGMLQuantizationType::F16,
        }
    }
}

/// What the conversion produced.
#[derive(Debug, Clone)]
pub struct HfToGgufSummary {
    /// Tensors written.
    pub tensors: usize,
    /// Bytes of tensor data written (excluding header and padding).
    pub tensor_bytes: u64,
    /// Vocabulary entries exported.
    pub vocab: usize,
}

// ── config.json ────────────────────────────────────────────────────

/// The llama hyperparameters GGUF needs.
struct LlamaConfig {
    hidden_size: u64,
    intermediate_size: u64,
    num_hidden_layers: u64,
    num_attention_heads: u64,
    num_key_value_heads: u64,
    max_position_embeddings: u64,
    rms_norm_eps: f32,
    rope_theta: f32,
    vocab_size: u64,
    bos_token_id: u32,
    eos_token_id: u32,
}

fn json_of(path: &Path) -> Result<serde_json::Value> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| VaultError::ConversionError(format!("{}: {e}", path.display())))?;
    serde_json::from_str(&text)
        .map_err(|e| VaultError::ConversionError(format!("{}: {e}", path.display())))
}

fn read_llama_config(dir: &Path) -> Result<LlamaConfig> {
    let v = json_of(&dir.join("config.json"))?;

    let model_type = v["model_type"].as_str().unwrap_or("<absent>");
    if model_type != "llama" {
        return Err(VaultError::ConversionError(format!(
            "model_type is {model_type:?}, and this converter handles \"llama\" only.\n\
             Converting other architectures correctly means per-architecture tensor \
             and hyperparameter mapping; guessing produces a GGUF that loads and \
             generates wrong text. Use llama.cpp convert_hf_to_gguf.py for {model_type:?}."
        )));
    }

    let num = |k: &str| -> Result<u64> {
        v[k].as_u64()
            .ok_or_else(|| VaultError::ConversionError(format!("config.json: missing {k}")))
    };
    let heads = num("num_attention_heads")?;

    Ok(LlamaConfig {
        hidden_size: num("hidden_size")?,
        intermediate_size: num("intermediate_size")?,
        num_hidden_layers: num("num_hidden_layers")?,
        num_attention_heads: heads,
        // Absent means MHA, i.e. as many KV heads as query heads.
        num_key_value_heads: v["num_key_value_heads"].as_u64().unwrap_or(heads),
        max_position_embeddings: num("max_position_embeddings")?,
        rms_norm_eps: v["rms_norm_eps"].as_f64().unwrap_or(1e-5) as f32,
        rope_theta: v["rope_theta"].as_f64().unwrap_or(10000.0) as f32,
        vocab_size: num("vocab_size")?,
        bos_token_id: v["bos_token_id"].as_u64().unwrap_or(1) as u32,
        eos_token_id: v["eos_token_id"].as_u64().unwrap_or(2) as u32,
    })
}

// ── safetensors ────────────────────────────────────────────────────

/// One tensor as safetensors describes it.
#[derive(Debug, Clone)]
struct StEntry {
    dtype: String,
    shape: Vec<u64>,
    /// Byte range within the shard, relative to the end of the header.
    begin: u64,
    end: u64,
    shard: std::path::PathBuf,
}

/// Read every shard header in `dir`, without reading any tensor data.
fn read_safetensors_index(dir: &Path) -> Result<HashMap<String, StEntry>> {
    let index = dir.join("model.safetensors.index.json");
    let shards: Vec<std::path::PathBuf> = if index.is_file() {
        let v = json_of(&index)?;
        let map = v["weight_map"]
            .as_object()
            .ok_or_else(|| VaultError::ConversionError("index.json has no weight_map".into()))?;
        let mut names: Vec<String> = map
            .values()
            .filter_map(|s| s.as_str().map(String::from))
            .collect();
        names.sort();
        names.dedup();
        names.into_iter().map(|n| dir.join(n)).collect()
    } else {
        let single = dir.join("model.safetensors");
        if !single.is_file() {
            return Err(VaultError::ConversionError(format!(
                "no model.safetensors or model.safetensors.index.json in {}",
                dir.display()
            )));
        }
        vec![single]
    };

    let mut out = HashMap::new();
    for shard in shards {
        let mut f = File::open(&shard)
            .map_err(|e| VaultError::ConversionError(format!("{}: {e}", shard.display())))?;
        let mut len_bytes = [0u8; 8];
        f.read_exact(&mut len_bytes)
            .map_err(|e| VaultError::ConversionError(format!("{}: {e}", shard.display())))?;
        let header_len = u64::from_le_bytes(len_bytes);
        // Same cap the existing SafeTensors reader uses: a crafted header must
        // not be able to make us allocate arbitrarily.
        if header_len > 100 * 1024 * 1024 {
            return Err(VaultError::ConversionError(format!(
                "{}: safetensors header is {header_len} bytes",
                shard.display()
            )));
        }
        let mut header = vec![0u8; header_len as usize];
        f.read_exact(&mut header)
            .map_err(|e| VaultError::ConversionError(format!("{}: {e}", shard.display())))?;
        let v: serde_json::Value = serde_json::from_slice(&header)
            .map_err(|e| VaultError::ConversionError(format!("{}: {e}", shard.display())))?;

        let obj = v.as_object().ok_or_else(|| {
            VaultError::ConversionError(format!("{}: header is not an object", shard.display()))
        })?;
        for (name, meta) in obj {
            if name == "__metadata__" {
                continue;
            }
            let offsets = meta["data_offsets"]
                .as_array()
                .ok_or_else(|| VaultError::ConversionError(format!("{name}: no data_offsets")))?;
            out.insert(
                name.clone(),
                StEntry {
                    dtype: meta["dtype"].as_str().unwrap_or("").to_string(),
                    shape: meta["shape"]
                        .as_array()
                        .map(|a| a.iter().filter_map(serde_json::Value::as_u64).collect())
                        .unwrap_or_default(),
                    begin: offsets[0].as_u64().unwrap_or(0),
                    end: offsets[1].as_u64().unwrap_or(0),
                    // Header length is needed to find the data; store it folded
                    // into the shard path lookup below.
                    shard: shard.clone(),
                },
            );
        }
        // Record where this shard's data section starts.
        out.entry(format!("__data_start__{}", shard.display()))
            .or_insert(StEntry {
                dtype: String::new(),
                shape: vec![],
                begin: 8 + header_len,
                end: 8 + header_len,
                shard: shard.clone(),
            });
    }
    Ok(out)
}

/// Read one tensor and widen it to `f32`.
fn read_tensor_f32(entry: &StEntry, data_start: u64) -> Result<Vec<f32>> {
    let mut f = File::open(&entry.shard)
        .map_err(|e| VaultError::ConversionError(format!("{}: {e}", entry.shard.display())))?;
    f.seek(SeekFrom::Start(data_start + entry.begin))
        .map_err(|e| VaultError::ConversionError(format!("{e}")))?;
    let mut raw = vec![0u8; (entry.end - entry.begin) as usize];
    f.read_exact(&mut raw)
        .map_err(|e| VaultError::ConversionError(format!("{e}")))?;

    match entry.dtype.as_str() {
        "F32" => Ok(raw
            .chunks_exact(4)
            .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect()),
        "F16" => Ok(raw
            .chunks_exact(2)
            .map(|c| quant::f16_to_f32([c[0], c[1]]))
            .collect()),
        "BF16" => Ok(raw
            .chunks_exact(2)
            .map(|c| quant::bf16_to_f32([c[0], c[1]]))
            .collect()),
        other => Err(VaultError::ConversionError(format!(
            "unsupported safetensors dtype {other:?} (F32, F16 and BF16 are handled)"
        ))),
    }
}

// ── tensor naming and the RoPE permutation ─────────────────────────

/// Map a HuggingFace llama tensor name to its GGUF name.
///
/// Returns `None` for tensors GGUF does not carry (e.g. rotary caches), which
/// are skipped rather than treated as an error.
fn gguf_name(hf: &str) -> Option<String> {
    let direct = match hf {
        "model.embed_tokens.weight" => Some("token_embd.weight"),
        "model.norm.weight" => Some("output_norm.weight"),
        "lm_head.weight" => Some("output.weight"),
        _ => None,
    };
    if let Some(d) = direct {
        return Some(d.to_string());
    }

    let rest = hf.strip_prefix("model.layers.")?;
    let (idx, tail) = rest.split_once('.')?;
    let suffix = match tail {
        "input_layernorm.weight" => "attn_norm.weight",
        "post_attention_layernorm.weight" => "ffn_norm.weight",
        "self_attn.q_proj.weight" => "attn_q.weight",
        "self_attn.k_proj.weight" => "attn_k.weight",
        "self_attn.v_proj.weight" => "attn_v.weight",
        "self_attn.o_proj.weight" => "attn_output.weight",
        "mlp.gate_proj.weight" => "ffn_gate.weight",
        "mlp.up_proj.weight" => "ffn_up.weight",
        "mlp.down_proj.weight" => "ffn_down.weight",
        _ => return None,
    };
    Some(format!("blk.{idx}.{suffix}"))
}

/// Reorder Q/K projection rows from HuggingFace's RoPE layout to GGML's.
///
/// 🚨 **This is the step most likely to be silently wrong.** HuggingFace splits
/// each head's rotary dimensions into two contiguous halves; GGML interleaves
/// them. llama.cpp's converter applies exactly this permutation to `q_proj` and
/// `k_proj` (and nothing else), so a GGUF written without it loads fine, runs
/// fine, and produces confidently wrong text — the rotation is applied to the
/// wrong dimension pairs.
///
/// For head `h` and half-index `b`, with `hd` rows per head:
///
/// ```text
/// dst[h*hd + 2b]     = src[h*hd + b]
/// dst[h*hd + 2b + 1] = src[h*hd + hd/2 + b]
/// ```
///
/// `n_head` is the query head count for `q_proj` and the **key/value** head
/// count for `k_proj` — using the wrong one is the same class of bug.
fn permute_rope_rows(data: &[f32], rows: usize, cols: usize, n_head: usize) -> Vec<f32> {
    let hd = rows / n_head;
    let half = hd / 2;
    let mut out = vec![0.0f32; data.len()];
    for h in 0..n_head {
        for b in 0..half {
            let dst_even = h * hd + 2 * b;
            let dst_odd = dst_even + 1;
            let src_even = h * hd + b;
            let src_odd = h * hd + half + b;
            out[dst_even * cols..(dst_even + 1) * cols]
                .copy_from_slice(&data[src_even * cols..(src_even + 1) * cols]);
            out[dst_odd * cols..(dst_odd + 1) * cols]
                .copy_from_slice(&data[src_odd * cols..(src_odd + 1) * cols]);
        }
    }
    out
}

// ── tokenizer ──────────────────────────────────────────────────────

struct Vocab {
    tokens: Vec<String>,
    scores: Vec<f32>,
    types: Vec<i32>,
    merges: Vec<String>,
    /// Value for `tokenizer.ggml.model`: "llama" (SentencePiece) or "gpt2" (BPE).
    /// This decides which tokenizer an engine builds, so it is not cosmetic.
    model: &'static str,
}

/// Read a little-endian `u32` varint from `data` at `pos`, returning the new
/// position. Minimal protobuf, enough for a SentencePiece model.
fn varint(data: &[u8], mut pos: usize) -> Option<(u64, usize)> {
    let mut value = 0u64;
    let mut shift = 0u32;
    while pos < data.len() {
        let byte = data[pos];
        pos += 1;
        value |= u64::from(byte & 0x7F) << shift;
        if byte & 0x80 == 0 {
            return Some((value, pos));
        }
        shift += 7;
        if shift >= 64 {
            return None;
        }
    }
    None
}

/// Parse `tokenizer.model` — a SentencePiece `ModelProto`.
///
/// 🚨 **This file is why the conversion is correct.** A llama GGUF declares
/// `tokenizer.ggml.model = "llama"`, and the SentencePiece path picks pieces by
/// **score**, so the scores are not decoration: without them the segmentation
/// differs from the model's own tokenizer and every generation is wrong.
///
/// An earlier version of this converter read `tokenizer.json` instead and
/// emitted the BPE form (`"gpt2"`). That tokenized `"The capital of France is"`
/// as 11 GPT-2 byte-level tokens against SentencePiece's 6 `▁`-prefixed ones,
/// and the model produced `"elle Maria Maria Maria..."`. The file loaded, the
/// engine ran, and the output was fluent nonsense.
///
/// Wire format: the top-level message has repeated field 1, each a
/// `SentencePiece { piece = 1 (string), score = 2 (float), type = 3 (varint) }`.
/// SentencePiece's type enum matches GGUF's `llama_token_type` one-for-one
/// (NORMAL 1, UNKNOWN 2, CONTROL 3, USER_DEFINED 4, UNUSED 5, BYTE 6), so the
/// values pass through unchanged.
fn parse_sentencepiece(data: &[u8]) -> Result<(Vec<String>, Vec<f32>, Vec<i32>)> {
    let mut tokens = Vec::new();
    let mut scores = Vec::new();
    let mut types = Vec::new();

    let mut pos = 0usize;
    while pos < data.len() {
        let (tag, next) = varint(data, pos)
            .ok_or_else(|| VaultError::ConversionError("tokenizer.model: bad varint".into()))?;
        pos = next;
        let field = tag >> 3;
        let wire = tag & 7;

        if field != 1 || wire != 2 {
            // Skip anything that is not a piece: trainer_spec, normalizer_spec…
            pos = skip_field(data, pos, wire)?;
            continue;
        }

        let (len, next) = varint(data, pos)
            .ok_or_else(|| VaultError::ConversionError("tokenizer.model: bad length".into()))?;
        pos = next;
        let end = pos
            .checked_add(len as usize)
            .filter(|e| *e <= data.len())
            .ok_or_else(|| VaultError::ConversionError("tokenizer.model: piece overruns".into()))?;

        let (mut piece, mut score, mut ty) = (String::new(), 0.0f32, 1i32);
        let mut cursor = pos;
        while cursor < end {
            let (piece_tag, after_tag) = varint(data, cursor).ok_or_else(|| {
                VaultError::ConversionError("tokenizer.model: bad piece varint".into())
            })?;
            cursor = after_tag;
            match (piece_tag >> 3, piece_tag & 7) {
                (1, 2) => {
                    let (str_len, str_start) = varint(data, cursor).ok_or_else(|| {
                        VaultError::ConversionError("tokenizer.model: bad piece len".into())
                    })?;
                    let str_end = str_start + str_len as usize;
                    if str_end > end {
                        return Err(VaultError::ConversionError(
                            "tokenizer.model: piece string overruns".into(),
                        ));
                    }
                    piece = String::from_utf8_lossy(&data[str_start..str_end]).into_owned();
                    cursor = str_end;
                }
                (2, 5) => {
                    if cursor + 4 > end {
                        return Err(VaultError::ConversionError(
                            "tokenizer.model: score overruns".into(),
                        ));
                    }
                    score = f32::from_le_bytes([
                        data[cursor],
                        data[cursor + 1],
                        data[cursor + 2],
                        data[cursor + 3],
                    ]);
                    cursor += 4;
                }
                (3, 0) => {
                    let (raw_ty, after_ty) = varint(data, cursor).ok_or_else(|| {
                        VaultError::ConversionError("tokenizer.model: bad type".into())
                    })?;
                    ty = raw_ty as i32;
                    cursor = after_ty;
                }
                (_, wire) => cursor = skip_field(data, cursor, wire)?,
            }
        }

        tokens.push(piece);
        scores.push(score);
        types.push(ty);
        pos = end;
    }

    if tokens.is_empty() {
        return Err(VaultError::ConversionError(
            "tokenizer.model contained no pieces".into(),
        ));
    }
    Ok((tokens, scores, types))
}

/// Advance past a protobuf field of the given wire type.
fn skip_field(data: &[u8], pos: usize, wire: u64) -> Result<usize> {
    let bad = || VaultError::ConversionError("tokenizer.model: malformed field".into());
    Ok(match wire {
        0 => varint(data, pos).ok_or_else(bad)?.1,
        1 => pos + 8,
        2 => {
            let (l, n) = varint(data, pos).ok_or_else(bad)?;
            n + l as usize
        }
        5 => pos + 4,
        _ => return Err(bad()),
    })
}

/// Read the BPE merge list out of `tokenizer.json`.
///
/// Both encodings are accepted: older files store a merge as the string
/// `"a b"`, newer ones as the pair `["a", "b"]`.
fn read_merges(dir: &Path) -> Result<Vec<String>> {
    let path = dir.join("tokenizer.json");
    if !path.is_file() {
        return Ok(Vec::new());
    }
    let v = json_of(&path)?;
    Ok(v["model"]["merges"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|m| match m {
                    serde_json::Value::Array(pair) if pair.len() == 2 => Some(format!(
                        "{} {}",
                        pair[0].as_str().unwrap_or(""),
                        pair[1].as_str().unwrap_or("")
                    )),
                    serde_json::Value::String(s) => Some(s.clone()),
                    _ => None,
                })
                .collect()
        })
        .unwrap_or_default())
}

/// Export the tokenizer into the arrays GGUF carries.
///
/// Prefers `tokenizer.model` (SentencePiece) because llama GGUFs declare the
/// SentencePiece path and it needs real scores — see [`parse_sentencepiece`].
/// `tokenizer.json` supplies only the merges, which the SentencePiece path does
/// not use but which cost nothing to carry.
fn read_vocab(dir: &Path) -> Result<Vocab> {
    let spm_path = dir.join("tokenizer.model");
    if spm_path.is_file() {
        let raw = std::fs::read(&spm_path)
            .map_err(|e| VaultError::ConversionError(format!("{}: {e}", spm_path.display())))?;
        let (tokens, scores, types) = parse_sentencepiece(&raw)?;

        // 🚨 Carry the merges too, from tokenizer.json.
        //
        // A llama GGUF is conventionally read as "SentencePiece", but engines
        // implement that as *BPE over `▁`-normalised text* rather than a
        // score-based Viterbi search — IronWorks does. Ship scores without
        // merges and there is nothing to combine characters with: measured,
        // "The capital of France is" came out as
        // [T, h, e, ▁, c, a, p, i, ...] instead of [▁The, ▁capital, ...].
        //
        // So both are required, and neither alone is a working tokenizer.
        let merges = read_merges(dir)?;
        if merges.is_empty() {
            return Err(VaultError::ConversionError(format!(
                "{} has tokenizer.model but no usable merges in tokenizer.json.\n\
                 A llama vocabulary needs both: the scores identify the pieces, the \
                 merges are what an engine actually combines characters with. \
                 Without merges the model tokenizes one character at a time.",
                dir.display()
            )));
        }

        return Ok(Vocab {
            tokens,
            scores,
            types,
            merges,
            model: "llama",
        });
    }

    Err(VaultError::ConversionError(format!(
        "{} has no tokenizer.model.\n\
         A llama GGUF declares the SentencePiece tokenizer, which selects pieces \
         by score, and the scores exist only in that file. Converting from \
         tokenizer.json alone produces a model that loads and tokenizes \
         differently from the real one — measured: 11 tokens where SentencePiece \
         gives 6, and fluent nonsense out.",
        dir.display()
    )))
}

#[allow(dead_code)]
fn read_vocab_from_tokenizer_json(dir: &Path) -> Result<Vocab> {
    let v = json_of(&dir.join("tokenizer.json"))?;

    let model = &v["model"];
    let ty = model["type"].as_str().unwrap_or("");
    if ty != "BPE" {
        return Err(VaultError::ConversionError(format!(
            "tokenizer.json model.type is {ty:?}; this converter handles \"BPE\""
        )));
    }

    let vocab_map = model["vocab"].as_object().ok_or_else(|| {
        VaultError::ConversionError("tokenizer.json: model.vocab is not an object".into())
    })?;

    let mut tokens = vec![String::new(); vocab_map.len()];
    for (tok, id) in vocab_map {
        let id = id.as_u64().unwrap_or(0) as usize;
        if id < tokens.len() {
            tokens[id] = tok.clone();
        }
    }

    // Token types. GGUF: 1 = normal, 2 = unknown, 3 = control, 6 = byte —
    // matching llama.cpp's llama_token_type. Byte tokens are the `<0xNN>`
    // fallbacks; control tokens are the specials declared in added_tokens.
    let mut types = vec![1i32; tokens.len()];
    for (i, t) in tokens.iter().enumerate() {
        if t.len() == 6 && t.starts_with("<0x") && t.ends_with('>') {
            types[i] = 6;
        }
    }
    if let Some(added) = v["added_tokens"].as_array() {
        for a in added {
            if a["special"].as_bool().unwrap_or(false) {
                if let Some(id) = a["id"].as_u64() {
                    if (id as usize) < types.len() {
                        types[id as usize] = 3;
                    }
                }
            }
        }
    }

    // Scores are unused by the BPE path (merge priority drives it), but the
    // array is written so a consumer expecting one finds it well-formed.
    let scores = vec![0.0f32; tokens.len()];

    let merges = model["merges"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|m| match m {
                    // Newer tokenizers store merges as ["a", "b"].
                    serde_json::Value::Array(pair) if pair.len() == 2 => Some(format!(
                        "{} {}",
                        pair[0].as_str().unwrap_or(""),
                        pair[1].as_str().unwrap_or("")
                    )),
                    serde_json::Value::String(s) => Some(s.clone()),
                    _ => None,
                })
                .collect()
        })
        .unwrap_or_default();

    Ok(Vocab {
        tokens,
        scores,
        types,
        merges,
        model: "gpt2",
    })
}

// ── the conversion ─────────────────────────────────────────────────

/// Convert a HuggingFace llama model directory into a GGUF file.
///
/// # Errors
///
/// Returns [`VaultError::ConversionError`] naming the specific problem: a
/// non-llama `model_type`, a missing config or tokenizer, an unsupported
/// safetensors dtype, or a tensor the mapping does not know.
pub fn convert_hf_to_gguf(
    src_dir: &Path,
    dst: &Path,
    options: &HfToGgufOptions,
) -> Result<HfToGgufSummary> {
    match options.out_type {
        GGMLQuantizationType::F16 | GGMLQuantizationType::BF16 | GGMLQuantizationType::F32 => {}
        other => {
            return Err(VaultError::ConversionError(format!(
                "cannot write {other:?}: no encoder for it exists in this project. \
                 Convert to F16 and run llama-quantize."
            )))
        }
    }

    let cfg = read_llama_config(src_dir)?;
    let vocab = read_vocab(src_dir)?;
    let index = read_safetensors_index(src_dir)?;

    // Where each shard's tensor data begins, keyed by shard path.
    let data_starts: HashMap<String, u64> = index
        .iter()
        .filter(|(k, _)| k.starts_with("__data_start__"))
        .map(|(_, e)| (e.shard.display().to_string(), e.begin))
        .collect();

    // Everything real, in a stable order: GGUF readers do not require one, but
    // a deterministic file is worth having.
    let mut entries: Vec<(String, String, StEntry)> = index
        .iter()
        .filter(|(k, _)| !k.starts_with("__data_start__"))
        .filter_map(|(hf, e)| gguf_name(hf).map(|g| (g, hf.clone(), e.clone())))
        .collect();
    entries.sort_by(|a, b| a.0.cmp(&b.0));

    if entries.is_empty() {
        return Err(VaultError::ConversionError(
            "no recognised llama tensors in the safetensors index".into(),
        ));
    }

    let mut w = GgufStreamWriter::new();
    w.add_metadata("general.architecture", GGUFValue::String("llama".into()));
    w.add_metadata(
        "general.name",
        GGUFValue::String(
            src_dir
                .file_name()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_else(|| "converted".into()),
        ),
    );
    w.add_metadata(
        "llama.context_length",
        GGUFValue::UInt32(cfg.max_position_embeddings as u32),
    );
    w.add_metadata(
        "llama.embedding_length",
        GGUFValue::UInt32(cfg.hidden_size as u32),
    );
    w.add_metadata(
        "llama.block_count",
        GGUFValue::UInt32(cfg.num_hidden_layers as u32),
    );
    w.add_metadata(
        "llama.feed_forward_length",
        GGUFValue::UInt32(cfg.intermediate_size as u32),
    );
    w.add_metadata(
        "llama.attention.head_count",
        GGUFValue::UInt32(cfg.num_attention_heads as u32),
    );
    w.add_metadata(
        "llama.attention.head_count_kv",
        GGUFValue::UInt32(cfg.num_key_value_heads as u32),
    );
    w.add_metadata(
        "llama.attention.layer_norm_rms_epsilon",
        GGUFValue::Float32(cfg.rms_norm_eps),
    );
    w.add_metadata("llama.rope.freq_base", GGUFValue::Float32(cfg.rope_theta));
    w.add_metadata("llama.vocab_size", GGUFValue::UInt32(cfg.vocab_size as u32));

    w.add_metadata(
        "tokenizer.ggml.model",
        GGUFValue::String(vocab.model.into()),
    );
    w.add_metadata(
        "tokenizer.ggml.tokens",
        GGUFValue::Array(
            vocab
                .tokens
                .iter()
                .cloned()
                .map(GGUFValue::String)
                .collect(),
        ),
    );
    w.add_metadata(
        "tokenizer.ggml.scores",
        GGUFValue::Array(
            vocab
                .scores
                .iter()
                .copied()
                .map(GGUFValue::Float32)
                .collect(),
        ),
    );
    w.add_metadata(
        "tokenizer.ggml.token_type",
        GGUFValue::Array(vocab.types.iter().copied().map(GGUFValue::Int32).collect()),
    );
    if !vocab.merges.is_empty() {
        w.add_metadata(
            "tokenizer.ggml.merges",
            GGUFValue::Array(
                vocab
                    .merges
                    .iter()
                    .cloned()
                    .map(GGUFValue::String)
                    .collect(),
            ),
        );
    }
    if vocab.model == "llama" {
        // SentencePiece prepends a "▁" to the start of the text (its
        // `add_dummy_prefix`), so "The" tokenizes as "▁The", not "The".
        //
        // ⚠️ IronWorks does not currently read this key, and its SentencePiece
        // path does not add the prefix — measured: it produced token 1576
        // ("The") where HuggingFace gives 450 ("▁The"), for every prompt that
        // does not already begin with a space. That affects every SPM model it
        // loads, not only converted ones. The key is written so the fix is a
        // reader change and not another conversion.
        w.add_metadata("tokenizer.ggml.add_space_prefix", GGUFValue::Bool(true));
    }
    w.add_metadata(
        "tokenizer.ggml.bos_token_id",
        GGUFValue::UInt32(cfg.bos_token_id),
    );
    w.add_metadata(
        "tokenizer.ggml.eos_token_id",
        GGUFValue::UInt32(cfg.eos_token_id),
    );

    // Declare every tensor before writing any. GGUF puts offsets in the header,
    // and the shapes are all known from the safetensors index.
    //
    // GGUF dimensions are the reverse of safetensors': HF stores a linear layer
    // as [out, in] row-major, GGML as ne = [in, out] with ne[0] contiguous. The
    // bytes are identical; only the declared shape flips.
    for (gname, _, e) in &entries {
        let dims: Vec<u64> = e.shape.iter().rev().copied().collect();
        // Norms stay F32: they are tiny, and quantising them costs accuracy for
        // no space. This is what llama.cpp does too.
        let ty = if gname.ends_with("_norm.weight") {
            GGMLQuantizationType::F32
        } else {
            options.out_type
        };
        w.declare_tensor(gname.clone(), &dims, ty)
            .map_err(|e| VaultError::ConversionError(format!("{gname}: {e}")))?;
    }

    let file = File::create(dst)
        .map_err(|e| VaultError::ConversionError(format!("{}: {e}", dst.display())))?;
    let mut body = w
        .begin(BufWriter::new(file))
        .map_err(|e| VaultError::ConversionError(format!("{}: {e}", dst.display())))?;

    let mut tensor_bytes = 0u64;
    for (gname, hf, e) in &entries {
        let start = *data_starts
            .get(&e.shard.display().to_string())
            .ok_or_else(|| VaultError::ConversionError(format!("{hf}: shard start unknown")))?;
        let mut values = read_tensor_f32(e, start)?;

        // Only q_proj and k_proj are permuted, and with different head counts.
        if gname.ends_with("attn_q.weight") || gname.ends_with("attn_k.weight") {
            let rows = e.shape[0] as usize;
            let cols = e.shape.get(1).copied().unwrap_or(1) as usize;
            let heads = if gname.ends_with("attn_q.weight") {
                cfg.num_attention_heads as usize
            } else {
                cfg.num_key_value_heads as usize
            };
            if !rows.is_multiple_of(heads * 2) {
                return Err(VaultError::ConversionError(format!(
                    "{gname}: {rows} rows is not divisible by 2x{heads} heads"
                )));
            }
            values = permute_rope_rows(&values, rows, cols, heads);
        }

        let ty = if gname.ends_with("_norm.weight") {
            GGMLQuantizationType::F32
        } else {
            options.out_type
        };
        let bytes = quant::encode(ty, &values)
            .map_err(|e| VaultError::ConversionError(format!("{gname}: {e}")))?;
        tensor_bytes += bytes.len() as u64;
        body.write_tensor(&bytes)
            .map_err(|e| VaultError::ConversionError(format!("{gname}: {e}")))?;
    }

    body.finish()
        .map_err(|e| VaultError::ConversionError(format!("{}: {e}", dst.display())))?;

    Ok(HfToGgufSummary {
        tensors: entries.len(),
        tensor_bytes,
        vocab: vocab.tokens.len(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The permutation is the step that fails silently, so pin it against the
    /// index arithmetic llama.cpp uses rather than only checking the length.
    #[test]
    fn rope_permutation_interleaves_the_two_halves() {
        // One head, 4 rows, 1 column: rows are [a, b, c, d] where [a,b] is the
        // first half and [c,d] the second. GGML wants [a, c, b, d].
        let src = vec![0.0, 1.0, 2.0, 3.0];
        let got = permute_rope_rows(&src, 4, 1, 1);
        assert_eq!(got, vec![0.0, 2.0, 1.0, 3.0]);
    }

    #[test]
    fn rope_permutation_is_per_head() {
        // Two heads of 4 rows. Each head permutes independently; rows never
        // cross a head boundary.
        let src: Vec<f32> = (0..8).map(|i| i as f32).collect();
        let got = permute_rope_rows(&src, 8, 1, 2);
        assert_eq!(got, vec![0.0, 2.0, 1.0, 3.0, 4.0, 6.0, 5.0, 7.0]);
    }

    #[test]
    fn rope_permutation_moves_whole_rows() {
        // 1 head, 4 rows of 2 columns: whole rows move, never individual values.
        let src = vec![0.0, 0.5, 1.0, 1.5, 2.0, 2.5, 3.0, 3.5];
        let got = permute_rope_rows(&src, 4, 2, 1);
        assert_eq!(got, vec![0.0, 0.5, 2.0, 2.5, 1.0, 1.5, 3.0, 3.5]);
    }

    #[test]
    fn the_permutation_is_a_bijection() {
        // Every source row appears exactly once. A permutation that dropped or
        // duplicated a row would still produce a loadable file.
        let rows = 128;
        let src: Vec<f32> = (0..rows).map(|i| i as f32).collect();
        let got = permute_rope_rows(&src, rows, 1, 8);
        let mut sorted = got.clone();
        sorted.sort_by(f32::total_cmp);
        assert_eq!(sorted, src);
    }

    #[test]
    fn tensor_names_map_to_ggml_convention() {
        assert_eq!(
            gguf_name("model.embed_tokens.weight").unwrap(),
            "token_embd.weight"
        );
        assert_eq!(gguf_name("lm_head.weight").unwrap(), "output.weight");
        assert_eq!(
            gguf_name("model.norm.weight").unwrap(),
            "output_norm.weight"
        );
        assert_eq!(
            gguf_name("model.layers.7.self_attn.q_proj.weight").unwrap(),
            "blk.7.attn_q.weight"
        );
        assert_eq!(
            gguf_name("model.layers.0.mlp.down_proj.weight").unwrap(),
            "blk.0.ffn_down.weight"
        );
        // Unknown tensors are skipped, not guessed at.
        assert!(gguf_name("model.layers.0.self_attn.rotary_emb.inv_freq").is_none());
    }
}
