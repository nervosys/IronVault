//! Compression utilities for efficient model storage

use flate2::write::{GzDecoder, GzEncoder};
use flate2::Compression as FlateCompression;
use std::io::Write;

use crate::error::{Result, VaultError};

/// Compression algorithm
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressionAlgorithm {
    /// No compression
    None,
    /// Gzip compression (fast, good ratio)
    Gzip,
    /// LZMA compression (slow, best ratio for large models)
    Lzma,
}

/// Compression level
#[derive(Debug, Clone, Copy)]
pub enum CompressionLevel {
    /// No compression
    None = 0,
    /// Fast compression
    Fast = 1,
    /// Balanced compression (default)
    Balanced = 6,
    /// Maximum compression
    Maximum = 9,
}

impl From<CompressionLevel> for FlateCompression {
    fn from(level: CompressionLevel) -> Self {
        match level {
            CompressionLevel::None => FlateCompression::none(),
            CompressionLevel::Fast => FlateCompression::fast(),
            CompressionLevel::Balanced => FlateCompression::new(6),
            CompressionLevel::Maximum => FlateCompression::best(),
        }
    }
}

/// Compress data using specified algorithm and level
pub fn compress(
    data: &[u8],
    algorithm: CompressionAlgorithm,
    level: CompressionLevel,
) -> Result<Vec<u8>> {
    match algorithm {
        CompressionAlgorithm::None => Ok(data.to_vec()),
        CompressionAlgorithm::Gzip => compress_gzip(data, level),
        CompressionAlgorithm::Lzma => compress_lzma(data, level),
    }
}

/// Decompress data using specified algorithm
pub fn decompress(data: &[u8], algorithm: CompressionAlgorithm) -> Result<Vec<u8>> {
    match algorithm {
        CompressionAlgorithm::None => Ok(data.to_vec()),
        CompressionAlgorithm::Gzip => decompress_gzip(data),
        CompressionAlgorithm::Lzma => decompress_lzma(data),
    }
}

/// Compress data using Gzip
fn compress_gzip(data: &[u8], level: CompressionLevel) -> Result<Vec<u8>> {
    let mut encoder = GzEncoder::new(Vec::new(), level.into());
    encoder
        .write_all(data)
        .map_err(|e| VaultError::CompressionError(format!("Gzip compression failed: {}", e)))?;
    encoder
        .finish()
        .map_err(|e| VaultError::CompressionError(format!("Gzip compression failed: {}", e)))
}

/// Decompress Gzip data
fn decompress_gzip(data: &[u8]) -> Result<Vec<u8>> {
    let mut decoder = GzDecoder::new(Vec::new());
    decoder
        .write_all(data)
        .map_err(|e| VaultError::CompressionError(format!("Gzip decompression failed: {}", e)))?;
    decoder
        .finish()
        .map_err(|e| VaultError::CompressionError(format!("Gzip decompression failed: {}", e)))
}

/// Compress data using LZMA
fn compress_lzma(data: &[u8], _level: CompressionLevel) -> Result<Vec<u8>> {
    let mut output = Vec::new();
    lzma_rs::lzma_compress(&mut std::io::Cursor::new(data), &mut output)
        .map_err(|e| VaultError::CompressionError(format!("LZMA compression failed: {}", e)))?;
    Ok(output)
}

/// Decompress LZMA data
fn decompress_lzma(data: &[u8]) -> Result<Vec<u8>> {
    let mut output = Vec::new();
    lzma_rs::lzma_decompress(&mut std::io::Cursor::new(data), &mut output)
        .map_err(|e| VaultError::CompressionError(format!("LZMA decompression failed: {}", e)))?;
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gzip_compress_decompress() {
        let data = b"Hello, IronVault! This is test data for compression.".repeat(100);
        let compressed = compress(
            &data,
            CompressionAlgorithm::Gzip,
            CompressionLevel::Balanced,
        )
        .unwrap();
        let decompressed = decompress(&compressed, CompressionAlgorithm::Gzip).unwrap();

        assert_eq!(data.to_vec(), decompressed);
        assert!(compressed.len() < data.len());
    }

    #[test]
    fn test_lzma_compress_decompress() {
        let data = b"Hello, IronVault! This is test data for compression.".repeat(100);
        let compressed = compress(
            &data,
            CompressionAlgorithm::Lzma,
            CompressionLevel::Balanced,
        )
        .unwrap();
        let decompressed = decompress(&compressed, CompressionAlgorithm::Lzma).unwrap();

        assert_eq!(data.to_vec(), decompressed);
        assert!(compressed.len() < data.len());
    }

    #[test]
    fn test_none_compress_decompress() {
        // Covers line 51 — CompressionAlgorithm::None path
        let data = b"uncompressed data".to_vec();
        let compressed =
            compress(&data, CompressionAlgorithm::None, CompressionLevel::Fast).unwrap();
        assert_eq!(compressed, data);
        let decompressed = decompress(&compressed, CompressionAlgorithm::None).unwrap();
        assert_eq!(decompressed, data);
    }

    #[test]
    fn test_compression_levels() {
        let data = b"Test data repeated for compression ".repeat(50);
        // Test Fast level
        let fast = compress(&data, CompressionAlgorithm::Gzip, CompressionLevel::Fast).unwrap();
        let decompressed = decompress(&fast, CompressionAlgorithm::Gzip).unwrap();
        assert_eq!(decompressed, data.to_vec());

        // Test Maximum level
        let max = compress(&data, CompressionAlgorithm::Gzip, CompressionLevel::Maximum).unwrap();
        let decompressed = decompress(&max, CompressionAlgorithm::Gzip).unwrap();
        assert_eq!(decompressed, data.to_vec());
        // Maximum should generally compress better or equal to Fast
        assert!(max.len() <= fast.len() + 10);
    }

    #[test]
    fn test_lzma_with_different_levels() {
        let data = b"LZMA test data with some content.".repeat(20);
        for level in [
            CompressionLevel::Fast,
            CompressionLevel::Balanced,
            CompressionLevel::Maximum,
        ] {
            let compressed = compress(&data, CompressionAlgorithm::Lzma, level).unwrap();
            let decompressed = decompress(&compressed, CompressionAlgorithm::Lzma).unwrap();
            assert_eq!(decompressed, data.to_vec());
        }
    }

    #[test]
    fn test_gzip_decompress_invalid_data() {
        // Covers error path in decompress_gzip
        let invalid = vec![0xFF, 0xFE, 0xFD, 0xFC];
        let result = decompress(&invalid, CompressionAlgorithm::Gzip);
        assert!(result.is_err());
    }

    #[test]
    fn test_lzma_decompress_invalid_data() {
        // Covers error path in decompress_lzma
        let invalid = vec![0xFF, 0xFE, 0xFD, 0xFC];
        let result = decompress(&invalid, CompressionAlgorithm::Lzma);
        assert!(result.is_err());
    }

    #[test]
    fn test_gzip_compress_level_none() {
        // Covers CompressionLevel::None -> FlateCompression::none() conversion
        let data = b"test data for level none compression".to_vec();
        let compressed =
            compress(&data, CompressionAlgorithm::Gzip, CompressionLevel::None).unwrap();
        let decompressed = decompress(&compressed, CompressionAlgorithm::Gzip).unwrap();
        assert_eq!(decompressed, data);
    }
}
