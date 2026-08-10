//! Minimal GGUF header reader.
//!
//! GGUF v2/v3 layout:
//! `[magic "GGUF"][version u32][tensor_count u64][metadata_count u64]`
//! `[metadata KV pairs][tensor infos][padding][tensor data]`
//!
//! Only the header is read — tensor data is never touched. The metadata block
//! is variable-length, so reaching the tensor infos means walking every KV
//! pair, skipping values by their type tag.
//!
//! Every read is bounds-checked. A truncated, corrupt, or hostile file yields
//! `None` or a partial result rather than panicking, because both callers
//! (`iv diff`, license scanning) run against files the vault does not control.

/// `gguf_metadata_value_type` tags.
const UINT8: u32 = 0;
const INT8: u32 = 1;
const UINT16: u32 = 2;
const INT16: u32 = 3;
const UINT32: u32 = 4;
const INT32: u32 = 5;
const FLOAT32: u32 = 6;
const BOOL: u32 = 7;
const STRING: u32 = 8;
const ARRAY: u32 = 9;
const UINT64: u32 = 10;
const INT64: u32 = 11;
const FLOAT64: u32 = 12;

/// Upper bound on a tensor's declared dimension count. ggml itself caps
/// tensors at 4; the headroom avoids rejecting a future revision outright.
const MAX_DIMS: u32 = 8;

/// A tensor descriptor read from a GGUF header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TensorDesc {
    pub name: String,
    pub shape: Vec<usize>,
    pub dtype: &'static str,
    pub param_count: u64,
}

/// Does this buffer start with the GGUF magic?
#[must_use]
pub fn is_gguf(data: &[u8]) -> bool {
    data.len() >= 4 && &data[0..4] == b"GGUF"
}

/// Human-readable name for a `ggml_type` discriminant.
#[must_use]
pub fn ggml_type_name(ggml_type: u32) -> &'static str {
    match ggml_type {
        0 => "F32",
        1 => "F16",
        2 => "Q4_0",
        3 => "Q4_1",
        6 => "Q5_0",
        7 => "Q5_1",
        8 => "Q8_0",
        9 => "Q8_1",
        10 => "Q2_K",
        11 => "Q3_K",
        12 => "Q4_K",
        13 => "Q5_K",
        14 => "Q6_K",
        15 => "Q8_K",
        16 => "IQ2_XXS",
        17 => "IQ2_XS",
        18 => "IQ3_XXS",
        19 => "IQ1_S",
        20 => "IQ4_NL",
        21 => "IQ3_S",
        22 => "IQ2_S",
        23 => "IQ4_XS",
        24 => "I8",
        25 => "I16",
        26 => "I32",
        27 => "I64",
        28 => "F64",
        29 => "IQ1_M",
        30 => "BF16",
        // 4 and 5 were Q4_2/Q4_3, removed from ggml.
        _ => "UNKNOWN",
    }
}

/// Read every tensor descriptor in the header.
///
/// Returns whatever was read before the first malformed field, so a truncated
/// file degrades to a partial list rather than an error.
#[must_use]
pub fn tensors(data: &[u8]) -> Vec<TensorDesc> {
    let mut out = Vec::new();

    let Some((mut r, tensor_count, metadata_count)) = Reader::open(data) else {
        return out;
    };
    if r.skip_metadata(metadata_count).is_none() {
        return out;
    }

    for _ in 0..tensor_count {
        let Some(name) = r.string() else { return out };
        let Some(n_dims) = r.u32() else { return out };
        if n_dims > MAX_DIMS {
            return out;
        }

        let mut shape = Vec::with_capacity(n_dims as usize);
        for _ in 0..n_dims {
            let Some(dim) = r.u64().and_then(|d| usize::try_from(d).ok()) else {
                return out;
            };
            shape.push(dim);
        }

        let Some(ggml_type) = r.u32() else { return out };
        // Data offset — not needed for header-level comparison.
        if r.u64().is_none() {
            return out;
        }

        let param_count = shape
            .iter()
            .try_fold(1u64, |acc, &d| acc.checked_mul(d as u64))
            .unwrap_or(u64::MAX);

        out.push(TensorDesc {
            name,
            shape,
            dtype: ggml_type_name(ggml_type),
            param_count,
        });
    }

    out
}

/// Look up a string-valued metadata key (e.g. `general.license`).
///
/// Returns `None` when the key is absent, when its value is not a string, or
/// when the header is malformed. It never guesses: the value returned is the
/// one stored under exactly this key, not something found near it.
#[must_use]
pub fn metadata_string(data: &[u8], key: &str) -> Option<String> {
    let (mut r, _tensor_count, metadata_count) = Reader::open(data)?;

    for _ in 0..metadata_count {
        let found = r.string()?;
        let value_type = r.u32()?;

        if found == key {
            return if value_type == STRING {
                r.string()
            } else {
                None
            };
        }

        r.skip_value(value_type, 0)?;
    }

    None
}

/// Bounds-checked little-endian cursor over a GGUF header.
struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    /// Validate the magic and version, then read the two counts, leaving the
    /// cursor at the first metadata key.
    fn open(data: &'a [u8]) -> Option<(Self, u64, u64)> {
        if !is_gguf(data) {
            return None;
        }
        let mut r = Self { data, pos: 4 };

        let version = r.u32()?;
        if !(2..=3).contains(&version) {
            // v1 encoded lengths as u32; nothing in the wild still uses it.
            return None;
        }

        let tensor_count = r.u64()?;
        let metadata_count = r.u64()?;
        Some((r, tensor_count, metadata_count))
    }

    fn take(&mut self, n: usize) -> Option<&'a [u8]> {
        let end = self.pos.checked_add(n)?;
        let slice = self.data.get(self.pos..end)?;
        self.pos = end;
        Some(slice)
    }

    fn u32(&mut self) -> Option<u32> {
        Some(u32::from_le_bytes(self.take(4)?.try_into().ok()?))
    }

    fn u64(&mut self) -> Option<u64> {
        Some(u64::from_le_bytes(self.take(8)?.try_into().ok()?))
    }

    /// GGUF strings are a u64 byte length followed by non-NUL-terminated UTF-8.
    fn string(&mut self) -> Option<String> {
        let len = usize::try_from(self.u64()?).ok()?;
        let bytes = self.take(len)?;
        Some(String::from_utf8_lossy(bytes).into_owned())
    }

    fn skip_metadata(&mut self, count: u64) -> Option<()> {
        for _ in 0..count {
            self.string()?;
            let value_type = self.u32()?;
            self.skip_value(value_type, 0)?;
        }
        Some(())
    }

    /// Advance past one metadata value of the given type without decoding it.
    fn skip_value(&mut self, value_type: u32, depth: u32) -> Option<()> {
        // Nested arrays are not legal GGUF, but a malformed file must not be
        // able to drive this into unbounded recursion.
        if depth > 4 {
            return None;
        }
        match value_type {
            UINT8 | INT8 | BOOL => self.take(1).map(|_| ()),
            UINT16 | INT16 => self.take(2).map(|_| ()),
            UINT32 | INT32 | FLOAT32 => self.take(4).map(|_| ()),
            UINT64 | INT64 | FLOAT64 => self.take(8).map(|_| ()),
            STRING => self.string().map(|_| ()),
            ARRAY => {
                let elem_type = self.u32()?;
                let count = self.u64()?;
                for _ in 0..count {
                    self.skip_value(elem_type, depth + 1)?;
                }
                Some(())
            }
            // An unknown tag has an unknown width, so the rest of the header is
            // no longer navigable.
            _ => None,
        }
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
pub(crate) mod test_support {
    /// A metadata value to embed in a synthetic header.
    pub enum Meta<'a> {
        Str(&'a str),
        StrArray(&'a [&'a str]),
        U32(u32),
    }

    pub fn push_string(out: &mut Vec<u8>, s: &str) {
        out.extend_from_slice(&(s.len() as u64).to_le_bytes());
        out.extend_from_slice(s.as_bytes());
    }

    /// Build a spec-shaped GGUF v3 header.
    ///
    /// `tensors` entries are `(name, dims, ggml_type)`.
    pub fn build(meta: &[(&str, Meta<'_>)], tensors: &[(&str, &[u64], u32)]) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(b"GGUF");
        data.extend_from_slice(&3u32.to_le_bytes());
        data.extend_from_slice(&(tensors.len() as u64).to_le_bytes());
        data.extend_from_slice(&(meta.len() as u64).to_le_bytes());

        for (key, value) in meta {
            push_string(&mut data, key);
            match value {
                Meta::Str(s) => {
                    data.extend_from_slice(&super::STRING.to_le_bytes());
                    push_string(&mut data, s);
                }
                Meta::StrArray(items) => {
                    data.extend_from_slice(&super::ARRAY.to_le_bytes());
                    data.extend_from_slice(&super::STRING.to_le_bytes());
                    data.extend_from_slice(&(items.len() as u64).to_le_bytes());
                    for item in *items {
                        push_string(&mut data, item);
                    }
                }
                Meta::U32(n) => {
                    data.extend_from_slice(&super::UINT32.to_le_bytes());
                    data.extend_from_slice(&n.to_le_bytes());
                }
            }
        }

        for (name, dims, ggml_type) in tensors {
            push_string(&mut data, name);
            data.extend_from_slice(&(dims.len() as u32).to_le_bytes());
            for d in *dims {
                data.extend_from_slice(&d.to_le_bytes());
            }
            data.extend_from_slice(&ggml_type.to_le_bytes());
            data.extend_from_slice(&0u64.to_le_bytes());
        }

        data
    }
}

#[cfg(test)]
mod tests {
    use super::test_support::{build, Meta};
    use super::*;

    fn sample() -> Vec<u8> {
        build(
            &[
                ("general.architecture", Meta::Str("llama")),
                ("general.license", Meta::Str("apache-2.0")),
                ("general.file_type", Meta::U32(15)),
                ("tokenizer.ggml.tokens", Meta::StrArray(&["a", "bb", "ccc"])),
            ],
            &[
                ("blk.0.attn_q.weight", &[4096, 4096], 12),
                ("output_norm.weight", &[4096], 0),
            ],
        )
    }

    #[test]
    fn reads_tensor_descriptors_past_the_metadata_block() {
        let t = tensors(&sample());
        assert_eq!(t.len(), 2);
        assert_eq!(t[0].name, "blk.0.attn_q.weight");
        assert_eq!(t[0].shape, vec![4096, 4096]);
        assert_eq!(t[0].dtype, "Q4_K");
        assert_eq!(t[0].param_count, 4096 * 4096);
        assert_eq!(t[1].dtype, "F32");
    }

    #[test]
    fn reads_a_string_metadata_value_by_key() {
        let data = sample();
        assert_eq!(
            metadata_string(&data, "general.license").as_deref(),
            Some("apache-2.0")
        );
        assert_eq!(
            metadata_string(&data, "general.architecture").as_deref(),
            Some("llama")
        );
    }

    #[test]
    fn absent_or_non_string_keys_yield_none() {
        let data = sample();
        assert!(metadata_string(&data, "general.nonexistent").is_none());
        // Present, but a u32 rather than a string.
        assert!(metadata_string(&data, "general.file_type").is_none());
    }

    /// The value must come from the requested key, never from a neighbouring
    /// one — the failure mode of the substring scan this module replaced.
    #[test]
    fn does_not_bleed_values_across_keys() {
        let data = build(
            &[
                ("general.license", Meta::Str("llama3")),
                (
                    "general.description",
                    Meta::Str("Distributed under the mit license, apache-2.0 compatible"),
                ),
            ],
            &[],
        );
        assert_eq!(
            metadata_string(&data, "general.license").as_deref(),
            Some("llama3")
        );
    }

    #[test]
    fn rejects_bad_magic_and_unsupported_versions() {
        let mut bad = sample();
        bad[0] = b'X';
        assert!(tensors(&bad).is_empty());
        assert!(metadata_string(&bad, "general.license").is_none());

        let mut v1 = sample();
        v1[4..8].copy_from_slice(&1u32.to_le_bytes());
        assert!(tensors(&v1).is_empty());
        assert!(metadata_string(&v1, "general.license").is_none());
    }

    #[test]
    fn truncation_never_panics() {
        let full = sample();
        for cut in 0..full.len() {
            let partial = &full[..cut];
            let t = tensors(partial);
            assert!(t.len() <= 2);
            let _ = metadata_string(partial, "general.license");
        }
    }

    /// A declared string length far beyond the buffer must fail the read rather
    /// than attempt a huge allocation.
    #[test]
    fn absurd_declared_length_is_rejected() {
        let mut data = Vec::new();
        data.extend_from_slice(b"GGUF");
        data.extend_from_slice(&3u32.to_le_bytes());
        data.extend_from_slice(&1u64.to_le_bytes());
        data.extend_from_slice(&1u64.to_le_bytes());
        data.extend_from_slice(&u64::MAX.to_le_bytes()); // key length
        data.extend_from_slice(b"short");

        assert!(tensors(&data).is_empty());
        assert!(metadata_string(&data, "general.license").is_none());
    }
}
