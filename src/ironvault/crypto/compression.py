"""
Compression utilities for efficient model storage.
"""

import gzip
import lzma
import zlib
from enum import Enum
from typing import Protocol


class CompressionLevel(Enum):
    """Compression level options."""
    NONE = 0
    FAST = 1
    BALANCED = 6
    MAXIMUM = 9


class Compressor(Protocol):
    """Protocol for compression implementations."""
    
    def compress(self, data: bytes, level: int) -> bytes:
        """Compress data."""
        ...
    
    def decompress(self, data: bytes) -> bytes:
        """Decompress data."""
        ...


class GzipCompressor:
    """Gzip compression (fast, good compression ratio)."""
    
    def compress(self, data: bytes, level: int = 6) -> bytes:
        """Compress using gzip."""
        return gzip.compress(data, compresslevel=level)
    
    def decompress(self, data: bytes) -> bytes:
        """Decompress gzip data."""
        return gzip.decompress(data)


class LZMACompressor:
    """LZMA compression (slower, better compression for large models)."""
    
    def compress(self, data: bytes, level: int = 6) -> bytes:
        """Compress using LZMA."""
        return lzma.compress(data, preset=level)
    
    def decompress(self, data: bytes) -> bytes:
        """Decompress LZMA data."""
        return lzma.decompress(data)


class ZlibCompressor:
    """Zlib compression (balanced speed/ratio)."""
    
    def compress(self, data: bytes, level: int = 6) -> bytes:
        """Compress using zlib."""
        return zlib.compress(data, level=level)
    
    def decompress(self, data: bytes) -> bytes:
        """Decompress zlib data."""
        return zlib.decompress(data)


def get_compressor(algorithm: str = "gzip") -> Compressor:
    """
    Get compressor implementation.
    
    Args:
        algorithm: Compression algorithm ('gzip', 'lzma', 'zlib')
    
    Returns:
        Compressor instance
    """
    compressors = {
        "gzip": GzipCompressor(),
        "lzma": LZMACompressor(),
        "zlib": ZlibCompressor(),
    }
    
    if algorithm not in compressors:
        raise ValueError(f"Unknown compression algorithm: {algorithm}")
    
    return compressors[algorithm]
