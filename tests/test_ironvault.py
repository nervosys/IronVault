"""
Python unit tests for the ironvault package.

Tests cover:
- ModelFormat detection and enumeration
- VaultConfig initialization and defaults
- Vault class subprocess interface
- FIPSCrypto (standalone, NOT interop with Rust)
- Compression utilities
- Package initialization and version
"""

import os
import tempfile
from pathlib import Path
from unittest.mock import patch, MagicMock

import pytest


# ---------------------------------------------------------------------------
# ModelFormat tests
# ---------------------------------------------------------------------------

class TestModelFormat:
    """Tests for ironvault.formats.registry.ModelFormat."""

    def test_detect_safetensors(self):
        from ironvault.formats.registry import ModelFormat
        assert ModelFormat.detect("model.safetensors") == ModelFormat.SAFETENSORS

    def test_detect_gguf(self):
        from ironvault.formats.registry import ModelFormat
        assert ModelFormat.detect("llama-7b.gguf") == ModelFormat.GGUF

    def test_detect_pytorch_pt(self):
        from ironvault.formats.registry import ModelFormat
        assert ModelFormat.detect("weights.pt") == ModelFormat.PYTORCH

    def test_detect_pytorch_pth(self):
        from ironvault.formats.registry import ModelFormat
        assert ModelFormat.detect("weights.pth") == ModelFormat.PYTORCH

    def test_detect_pytorch_bin(self):
        from ironvault.formats.registry import ModelFormat
        assert ModelFormat.detect("pytorch_model.bin") == ModelFormat.PYTORCH

    def test_detect_onnx(self):
        from ironvault.formats.registry import ModelFormat
        assert ModelFormat.detect("model.onnx") == ModelFormat.ONNX

    def test_detect_tflite(self):
        from ironvault.formats.registry import ModelFormat
        assert ModelFormat.detect("model.tflite") == ModelFormat.TFLITE

    def test_detect_coreml(self):
        from ironvault.formats.registry import ModelFormat
        assert ModelFormat.detect("model.mlmodel") == ModelFormat.COREML

    def test_detect_tensorrt_plan(self):
        from ironvault.formats.registry import ModelFormat
        assert ModelFormat.detect("engine.plan") == ModelFormat.TENSORRT

    def test_detect_openvino(self):
        from ironvault.formats.registry import ModelFormat
        assert ModelFormat.detect("model.xml") == ModelFormat.OPENVINO

    def test_detect_keras_h5(self):
        from ironvault.formats.registry import ModelFormat
        assert ModelFormat.detect("model.h5") == ModelFormat.KERAS

    def test_detect_keras_ext(self):
        from ironvault.formats.registry import ModelFormat
        assert ModelFormat.detect("model.keras") == ModelFormat.KERAS

    def test_detect_tensorflow_pb(self):
        from ironvault.formats.registry import ModelFormat
        assert ModelFormat.detect("saved_model.pb") == ModelFormat.TENSORFLOW

    def test_detect_pickle(self):
        from ironvault.formats.registry import ModelFormat
        assert ModelFormat.detect("model.pkl") == ModelFormat.PICKLE

    def test_detect_numpy(self):
        from ironvault.formats.registry import ModelFormat
        assert ModelFormat.detect("weights.npy") == ModelFormat.NUMPY

    def test_detect_hdf5(self):
        from ironvault.formats.registry import ModelFormat
        assert ModelFormat.detect("data.hdf5") == ModelFormat.HDF5

    def test_detect_mnn(self):
        from ironvault.formats.registry import ModelFormat
        assert ModelFormat.detect("model.mnn") == ModelFormat.MNN

    def test_detect_rknn(self):
        from ironvault.formats.registry import ModelFormat
        assert ModelFormat.detect("model.rknn") == ModelFormat.RKNN

    def test_detect_darknet(self):
        from ironvault.formats.registry import ModelFormat
        assert ModelFormat.detect("yolov4.weights") == ModelFormat.DARKNET

    def test_detect_caffe(self):
        from ironvault.formats.registry import ModelFormat
        assert ModelFormat.detect("model.caffemodel") == ModelFormat.CAFFE

    def test_detect_unknown_returns_custom(self):
        from ironvault.formats.registry import ModelFormat
        assert ModelFormat.detect("model.xyz") == ModelFormat.CUSTOM

    def test_detect_case_insensitive(self):
        from ironvault.formats.registry import ModelFormat
        # Path().suffix.lower() ensures case insensitivity
        assert ModelFormat.detect("MODEL.SAFETENSORS") == ModelFormat.SAFETENSORS

    def test_file_extensions_pytorch(self):
        from ironvault.formats.registry import ModelFormat
        exts = ModelFormat.PYTORCH.file_extensions
        assert ".pt" in exts
        assert ".pth" in exts

    def test_file_extensions_empty_for_custom(self):
        from ironvault.formats.registry import ModelFormat
        assert ModelFormat.CUSTOM.file_extensions == []

    def test_str_representation(self):
        from ironvault.formats.registry import ModelFormat
        assert str(ModelFormat.SAFETENSORS) == "safetensors"
        assert str(ModelFormat.PYTORCH) == "pytorch"

    def test_all_rust_variants_present(self):
        """Ensure every Rust ModelFormat variant has a Python counterpart."""
        from ironvault.formats.registry import ModelFormat
        expected = {
            "SAFETENSORS", "GGUF", "PYTORCH", "TENSORRT", "ONNX", "MLX",
            "COREML", "TORCHSCRIPT", "TFLITE", "TENSORFLOW", "KERAS",
            "OPENVINO", "TVM", "NCNN", "MNN", "RKNN", "CAFFE", "MXNET",
            "DARKNET", "HDF5", "PICKLE", "NUMPY", "CUSTOM",
        }
        actual = {m.name for m in ModelFormat}
        assert expected == actual, f"Missing: {expected - actual}, Extra: {actual - expected}"


# ---------------------------------------------------------------------------
# VaultConfig tests
# ---------------------------------------------------------------------------

class TestVaultConfig:
    """Tests for ironvault.core.config.VaultConfig."""

    def test_config_creates_directories(self):
        from ironvault.core.config import VaultConfig
        with tempfile.TemporaryDirectory() as tmpdir:
            with patch("ironvault.core.config.user_config_dir", return_value=os.path.join(tmpdir, "config")), \
                 patch("ironvault.core.config.user_data_dir", return_value=os.path.join(tmpdir, "data")), \
                 patch("ironvault.core.config.user_cache_dir", return_value=os.path.join(tmpdir, "cache")):
                cfg = VaultConfig()
                assert cfg.config_dir.exists()
                assert cfg.data_dir.exists()
                assert cfg.cache_dir.exists()

    def test_default_config_values(self):
        from ironvault.core.config import VaultConfig
        with tempfile.TemporaryDirectory() as tmpdir:
            with patch("ironvault.core.config.user_config_dir", return_value=os.path.join(tmpdir, "config")), \
                 patch("ironvault.core.config.user_data_dir", return_value=os.path.join(tmpdir, "data")), \
                 patch("ironvault.core.config.user_cache_dir", return_value=os.path.join(tmpdir, "cache")):
                cfg = VaultConfig()
                assert cfg.crypto_algorithm == "aes-256-gcm"
                assert cfg.kdf == "pbkdf2-hmac-sha256"
                assert cfg.kdf_iterations == 600000
                assert cfg.compression_algorithm == "gzip"
                assert cfg.max_versions == 10
                assert cfg.require_passphrase is True
                assert cfg.fips_mode is True

    def test_config_override(self):
        from ironvault.core.config import VaultConfig
        with tempfile.TemporaryDirectory() as tmpdir:
            with patch("ironvault.core.config.user_config_dir", return_value=os.path.join(tmpdir, "config")), \
                 patch("ironvault.core.config.user_data_dir", return_value=os.path.join(tmpdir, "data")), \
                 patch("ironvault.core.config.user_cache_dir", return_value=os.path.join(tmpdir, "cache")):
                cfg = VaultConfig(config_override={"custom_key": "custom_value"})
                assert cfg.config["custom_key"] == "custom_value"

    def test_save_and_reload_config(self):
        from ironvault.core.config import VaultConfig
        with tempfile.TemporaryDirectory() as tmpdir:
            with patch("ironvault.core.config.user_config_dir", return_value=os.path.join(tmpdir, "config")), \
                 patch("ironvault.core.config.user_data_dir", return_value=os.path.join(tmpdir, "data")), \
                 patch("ironvault.core.config.user_cache_dir", return_value=os.path.join(tmpdir, "cache")):
                cfg1 = VaultConfig()
                cfg1.config["test_marker"] = "present"
                cfg1.save_config()

                cfg2 = VaultConfig()
                assert cfg2.config.get("test_marker") == "present"

    def test_get_vault_path_default(self):
        from ironvault.core.config import VaultConfig
        with tempfile.TemporaryDirectory() as tmpdir:
            with patch("ironvault.core.config.user_config_dir", return_value=os.path.join(tmpdir, "config")), \
                 patch("ironvault.core.config.user_data_dir", return_value=os.path.join(tmpdir, "data")), \
                 patch("ironvault.core.config.user_cache_dir", return_value=os.path.join(tmpdir, "cache")):
                cfg = VaultConfig()
                vault_path = cfg.get_vault_path()
                assert "default" in str(vault_path)

    def test_get_vault_path_named(self):
        from ironvault.core.config import VaultConfig
        with tempfile.TemporaryDirectory() as tmpdir:
            with patch("ironvault.core.config.user_config_dir", return_value=os.path.join(tmpdir, "config")), \
                 patch("ironvault.core.config.user_data_dir", return_value=os.path.join(tmpdir, "data")), \
                 patch("ironvault.core.config.user_cache_dir", return_value=os.path.join(tmpdir, "cache")):
                cfg = VaultConfig()
                vault_path = cfg.get_vault_path("production")
                assert "production" in str(vault_path)


# ---------------------------------------------------------------------------
# Vault class tests (subprocess mock)
# ---------------------------------------------------------------------------

class TestVault:
    """Tests for ironvault.core.vault.Vault subprocess wrapper."""

    def test_vault_init(self):
        from ironvault.core.vault import Vault
        with tempfile.TemporaryDirectory() as tmpdir:
            with patch("ironvault.core.config.user_config_dir", return_value=os.path.join(tmpdir, "config")), \
                 patch("ironvault.core.config.user_data_dir", return_value=os.path.join(tmpdir, "data")), \
                 patch("ironvault.core.config.user_cache_dir", return_value=os.path.join(tmpdir, "cache")):
                vault = Vault(os.path.join(tmpdir, "vault"))
                assert vault.path == Path(os.path.join(tmpdir, "vault"))

    def test_vault_list_models_mock(self):
        from ironvault.core.vault import Vault
        with tempfile.TemporaryDirectory() as tmpdir:
            with patch("ironvault.core.config.user_config_dir", return_value=os.path.join(tmpdir, "config")), \
                 patch("ironvault.core.config.user_data_dir", return_value=os.path.join(tmpdir, "data")), \
                 patch("ironvault.core.config.user_cache_dir", return_value=os.path.join(tmpdir, "cache")):
                vault = Vault(os.path.join(tmpdir, "vault"))
                with patch("subprocess.run") as mock_run:
                    mock_run.return_value = MagicMock(
                        returncode=0, stdout='model_a\nmodel_b\n', stderr=""
                    )
                    models = vault.list_models()
                    assert "model_a" in models
                    assert "model_b" in models


# ---------------------------------------------------------------------------
# FIPSCrypto tests (standalone Python crypto, NOT interop with Rust)
# ---------------------------------------------------------------------------

class TestFIPSCrypto:
    """Tests for ironvault.crypto.fips.FIPSCrypto."""

    def test_key_generation(self):
        from ironvault.crypto.fips import FIPSCrypto
        crypto = FIPSCrypto()
        key, salt = crypto.generate_key(b"test-passphrase")
        assert len(key) == FIPSCrypto.KEY_SIZE
        assert len(salt) == FIPSCrypto.SALT_SIZE

    def test_key_deterministic_with_salt(self):
        from ironvault.crypto.fips import FIPSCrypto
        crypto = FIPSCrypto()
        key1, salt = crypto.generate_key(b"passphrase")
        key2, _ = crypto.generate_key(b"passphrase", salt=salt)
        assert key1 == key2

    def test_different_passwords_different_keys(self):
        from ironvault.crypto.fips import FIPSCrypto
        crypto = FIPSCrypto()
        key1, salt = crypto.generate_key(b"password-one")
        key2, _ = crypto.generate_key(b"password-two", salt=salt)
        assert key1 != key2

    def test_encrypt_decrypt_roundtrip(self):
        from ironvault.crypto.fips import FIPSCrypto
        crypto = FIPSCrypto()
        key, _ = crypto.generate_key(b"roundtrip-test")
        plaintext = b"Hello, IronVault!"
        ciphertext = crypto.encrypt(plaintext, key)
        assert ciphertext != plaintext
        decrypted = crypto.decrypt(ciphertext, key)
        assert decrypted == plaintext

    def test_encrypt_produces_different_ciphertexts(self):
        from ironvault.crypto.fips import FIPSCrypto
        crypto = FIPSCrypto()
        key, _ = crypto.generate_key(b"nonce-test")
        plaintext = b"Same data, different nonces"
        ct1 = crypto.encrypt(plaintext, key)
        ct2 = crypto.encrypt(plaintext, key)
        # Different nonces → different ciphertexts
        assert ct1 != ct2

    def test_decrypt_wrong_key_fails(self):
        from ironvault.crypto.fips import FIPSCrypto
        crypto = FIPSCrypto()
        key1, _ = crypto.generate_key(b"correct-password")
        key2, _ = crypto.generate_key(b"wrong-password")
        plaintext = b"Secret model weights"
        ciphertext = crypto.encrypt(plaintext, key1)
        with pytest.raises(Exception):
            crypto.decrypt(ciphertext, key2)

    def test_encrypt_empty_data(self):
        from ironvault.crypto.fips import FIPSCrypto
        crypto = FIPSCrypto()
        key, _ = crypto.generate_key(b"empty-test")
        ciphertext = crypto.encrypt(b"", key)
        decrypted = crypto.decrypt(ciphertext, key)
        assert decrypted == b""

    def test_encrypt_large_data(self):
        from ironvault.crypto.fips import FIPSCrypto
        crypto = FIPSCrypto()
        key, _ = crypto.generate_key(b"large-test")
        plaintext = os.urandom(1024 * 1024)  # 1 MB
        ciphertext = crypto.encrypt(plaintext, key)
        decrypted = crypto.decrypt(ciphertext, key)
        assert decrypted == plaintext


# ---------------------------------------------------------------------------
# Compression tests
# ---------------------------------------------------------------------------

class TestCompression:
    """Tests for ironvault.crypto.compression module."""

    def test_gzip_roundtrip(self):
        from ironvault.crypto.compression import GzipCompressor
        c = GzipCompressor()
        data = b"IronVault compression test" * 100
        compressed = c.compress(data)
        assert c.decompress(compressed) == data

    def test_zlib_roundtrip(self):
        from ironvault.crypto.compression import ZlibCompressor
        c = ZlibCompressor()
        data = b"Zlib compression test data" * 100
        compressed = c.compress(data)
        assert c.decompress(compressed) == data

    def test_lzma_roundtrip(self):
        from ironvault.crypto.compression import LZMACompressor
        c = LZMACompressor()
        data = b"LZMA compression test data" * 100
        compressed = c.compress(data)
        assert c.decompress(compressed) == data

    def test_compression_reduces_size(self):
        from ironvault.crypto.compression import GzipCompressor
        c = GzipCompressor()
        data = b"A" * 10_000
        compressed = c.compress(data)
        assert len(compressed) < len(data)

    def test_empty_data_roundtrip(self):
        from ironvault.crypto.compression import GzipCompressor
        c = GzipCompressor()
        compressed = c.compress(b"")
        assert c.decompress(compressed) == b""

    def test_get_compressor_gzip(self):
        from ironvault.crypto.compression import get_compressor, GzipCompressor
        c = get_compressor("gzip")
        assert isinstance(c, GzipCompressor)

    def test_get_compressor_lzma(self):
        from ironvault.crypto.compression import get_compressor, LZMACompressor
        c = get_compressor("lzma")
        assert isinstance(c, LZMACompressor)

    def test_get_compressor_zlib(self):
        from ironvault.crypto.compression import get_compressor, ZlibCompressor
        c = get_compressor("zlib")
        assert isinstance(c, ZlibCompressor)

    def test_get_compressor_unknown_raises(self):
        from ironvault.crypto.compression import get_compressor
        with pytest.raises(ValueError):
            get_compressor("brotli")

    def test_compression_levels(self):
        from ironvault.crypto.compression import GzipCompressor
        c = GzipCompressor()
        data = b"Test data for compression levels" * 500
        fast = c.compress(data, level=1)
        maximum = c.compress(data, level=9)
        # Both should decompress correctly
        assert c.decompress(fast) == data
        assert c.decompress(maximum) == data
        # Maximum compression should be at least as good (usually better)
        assert len(maximum) <= len(fast)


# ---------------------------------------------------------------------------
# Package initialization tests
# ---------------------------------------------------------------------------

class TestPackageInit:
    """Tests for ironvault package initialization."""

    def test_version_is_set(self):
        """The package version must be a release version, not a hardcoded literal.

        Asserting a specific string here is what let the package drift to 1.3.0
        while the test still expected 1.2.1 — check the shape instead.
        """
        import re
        import ironvault
        assert re.fullmatch(r"\d+\.\d+\.\d+", ironvault.__version__), (
            f"unexpected version format: {ironvault.__version__!r}"
        )

    def test_version_matches_crate(self):
        """The Python package and the Rust crate ship as one release."""
        import re
        from pathlib import Path
        import ironvault

        cargo = Path(__file__).resolve().parent.parent / "Cargo.toml"
        crate_version = re.search(
            r'^version = "([^"]+)"', cargo.read_text(encoding="utf-8"), re.MULTILINE
        ).group(1)
        assert ironvault.__version__ == crate_version

    def test_pyproject_version_matches_crate(self):
        """The wheel's metadata version is a third place the number lives.

        `__init__.py` was checked against Cargo.toml but `pyproject.toml` was
        not, so a release could bump two of the three and ship a wheel whose
        metadata disagreed with the package it contained.
        """
        import re
        from pathlib import Path

        root = Path(__file__).resolve().parent.parent
        crate_version = re.search(
            r'^version = "([^"]+)"',
            (root / "Cargo.toml").read_text(encoding="utf-8"),
            re.MULTILINE,
        ).group(1)
        pyproject_version = re.search(
            r'^version = "([^"]+)"',
            (root / "pyproject.toml").read_text(encoding="utf-8"),
            re.MULTILINE,
        ).group(1)

        assert pyproject_version == crate_version

    def test_native_flag_exists(self):
        import ironvault
        assert isinstance(ironvault._NATIVE, bool)

    def test_vault_is_importable(self):
        from ironvault import Vault
        assert Vault is not None

    def test_vault_config_is_importable(self):
        from ironvault import VaultConfig
        assert VaultConfig is not None

    def test_model_format_is_importable(self):
        from ironvault import ModelFormat
        assert ModelFormat is not None


# ---------------------------------------------------------------------------
# Vault path and property tests
# ---------------------------------------------------------------------------

class TestVaultProperties:
    """Tests for Vault class properties and initialization."""

    def test_vault_path_property(self):
        from ironvault.core.vault import Vault
        with tempfile.TemporaryDirectory() as tmpdir:
            with patch("ironvault.core.config.user_config_dir", return_value=os.path.join(tmpdir, "config")), \
                 patch("ironvault.core.config.user_data_dir", return_value=os.path.join(tmpdir, "data")), \
                 patch("ironvault.core.config.user_cache_dir", return_value=os.path.join(tmpdir, "cache")):
                vault_dir = os.path.join(tmpdir, "test_vault")
                vault = Vault(vault_dir)
                assert vault.path == Path(vault_dir)
                assert vault.path.exists()

    def test_vault_creates_directory(self):
        from ironvault.core.vault import Vault
        with tempfile.TemporaryDirectory() as tmpdir:
            with patch("ironvault.core.config.user_config_dir", return_value=os.path.join(tmpdir, "config")), \
                 patch("ironvault.core.config.user_data_dir", return_value=os.path.join(tmpdir, "data")), \
                 patch("ironvault.core.config.user_cache_dir", return_value=os.path.join(tmpdir, "cache")):
                vault_dir = os.path.join(tmpdir, "nested", "vault", "dir")
                vault = Vault(vault_dir)
                assert Path(vault_dir).exists()

    def test_vault_store_calls_aim(self):
        from ironvault.core.vault import Vault
        with tempfile.TemporaryDirectory() as tmpdir:
            with patch("ironvault.core.config.user_config_dir", return_value=os.path.join(tmpdir, "config")), \
                 patch("ironvault.core.config.user_data_dir", return_value=os.path.join(tmpdir, "data")), \
                 patch("ironvault.core.config.user_cache_dir", return_value=os.path.join(tmpdir, "cache")):
                vault = Vault(os.path.join(tmpdir, "vault"))
                with patch("subprocess.run") as mock_run:
                    mock_run.return_value = MagicMock(returncode=0, stdout="", stderr="")
                    vault.store("test-model", "/path/to/model.pt",
                                passphrase="secret", description="A test model")
                    mock_run.assert_called_once()
                    args = mock_run.call_args[0][0]
                    assert "store" in args
                    assert "test-model" in args
                    assert "--description" in args

    def test_vault_aim_not_found_raises(self):
        from ironvault.core.vault import Vault
        with tempfile.TemporaryDirectory() as tmpdir:
            with patch("ironvault.core.config.user_config_dir", return_value=os.path.join(tmpdir, "config")), \
                 patch("ironvault.core.config.user_data_dir", return_value=os.path.join(tmpdir, "data")), \
                 patch("ironvault.core.config.user_cache_dir", return_value=os.path.join(tmpdir, "cache")):
                vault = Vault(os.path.join(tmpdir, "vault"))
                with patch("subprocess.run", side_effect=FileNotFoundError):
                    with pytest.raises(FileNotFoundError, match="iv"):
                        vault.list_models()

    def test_vault_aim_error_raises_runtime(self):
        from ironvault.core.vault import Vault
        with tempfile.TemporaryDirectory() as tmpdir:
            with patch("ironvault.core.config.user_config_dir", return_value=os.path.join(tmpdir, "config")), \
                 patch("ironvault.core.config.user_data_dir", return_value=os.path.join(tmpdir, "data")), \
                 patch("ironvault.core.config.user_cache_dir", return_value=os.path.join(tmpdir, "cache")):
                vault = Vault(os.path.join(tmpdir, "vault"))
                with patch("subprocess.run") as mock_run:
                    mock_run.return_value = MagicMock(
                        returncode=1, stdout="", stderr="error: vault not found"
                    )
                    with pytest.raises(RuntimeError, match="iv command failed"):
                        vault.list_models()


# ---------------------------------------------------------------------------
# Version control tests
# ---------------------------------------------------------------------------

class TestVersionControl:
    """Tests for version control system."""

    def test_model_version_dataclass(self):
        from ironvault.version.control import ModelVersion
        ver = ModelVersion(
            version=1,
            checkpoint_id="m1-v1-abc",
            timestamp="2025-01-01T00:00:00Z",
            parent_version=None,
            format="safetensors",
            size_bytes=1024,
            compressed_size_bytes=512,
            checksum_sha256="deadbeef",
            metadata={},
            file_path="/tmp/model.enc",
        )
        assert ver.version == 1
        assert ver.format == "safetensors"
        assert ver.parent_version is None

    def test_model_version_to_dict(self):
        from ironvault.version.control import ModelVersion
        ver = ModelVersion(1, "id", "ts", None, "onnx", 100, 50, "abc", {}, "/f")
        d = ver.to_dict()
        assert d["version"] == 1
        assert d["format"] == "onnx"
        assert isinstance(d, dict)

    def test_model_version_from_dict(self):
        from ironvault.version.control import ModelVersion
        data = {
            "version": 2, "checkpoint_id": "cp2", "timestamp": "t",
            "parent_version": 1, "format": "pytorch", "size_bytes": 200,
            "compressed_size_bytes": 100, "checksum_sha256": "ff",
            "metadata": {"key": "val"}, "file_path": "/x",
        }
        ver = ModelVersion.from_dict(data)
        assert ver.version == 2
        assert ver.parent_version == 1
        assert ver.metadata["key"] == "val"

    def test_version_control_add_and_list(self):
        from ironvault.version.control import VersionControl
        with tempfile.TemporaryDirectory() as tmpdir:
            vc = VersionControl(Path(tmpdir))
            v1 = vc.add_version("model", "/f", "safetensors", 100, 50, "abc")
            assert v1.version == 1
            v2 = vc.add_version("model", "/f2", "onnx", 200, 100, "def")
            assert v2.version == 2
            versions = vc.list_versions("model")
            assert len(versions) == 2
            assert versions[0].version == 1
            assert versions[1].version == 2

    def test_version_control_get_latest(self):
        from ironvault.version.control import VersionControl
        with tempfile.TemporaryDirectory() as tmpdir:
            vc = VersionControl(Path(tmpdir))
            vc.add_version("m", "/f", "pt", 1, 1, "a")
            vc.add_version("m", "/f", "pt", 2, 2, "b")
            latest = vc.get_version("m")
            assert latest.version == 2

    def test_version_control_get_specific(self):
        from ironvault.version.control import VersionControl
        with tempfile.TemporaryDirectory() as tmpdir:
            vc = VersionControl(Path(tmpdir))
            vc.add_version("m", "/f", "pt", 1, 1, "a")
            vc.add_version("m", "/f", "pt", 2, 2, "b")
            v1 = vc.get_version("m", 1)
            assert v1.version == 1

    def test_version_control_get_nonexistent(self):
        from ironvault.version.control import VersionControl
        with tempfile.TemporaryDirectory() as tmpdir:
            vc = VersionControl(Path(tmpdir))
            assert vc.get_version("nonexistent") is None
            assert vc.get_version("nonexistent", 1) is None

    def test_version_control_delete(self):
        from ironvault.version.control import VersionControl
        with tempfile.TemporaryDirectory() as tmpdir:
            vc = VersionControl(Path(tmpdir))
            vc.add_version("m", "/f", "pt", 1, 1, "a")
            assert vc.delete_version("m", 1) is True
            assert vc.delete_version("m", 1) is False
            assert vc.delete_version("x", 1) is False

    def test_version_control_lineage(self):
        from ironvault.version.control import VersionControl
        with tempfile.TemporaryDirectory() as tmpdir:
            vc = VersionControl(Path(tmpdir))
            vc.add_version("m", "/f", "pt", 1, 1, "a")
            vc.add_version("m", "/f", "pt", 2, 2, "b", parent_version=1)
            lineage = vc.get_lineage("m", 2)
            assert len(lineage) == 2
            assert lineage[0].version == 1
            assert lineage[1].version == 2

    def test_version_control_persistence(self):
        from ironvault.version.control import VersionControl
        with tempfile.TemporaryDirectory() as tmpdir:
            vc1 = VersionControl(Path(tmpdir))
            vc1.add_version("m", "/f", "pt", 100, 50, "abc")
            # Re-load from disk
            vc2 = VersionControl(Path(tmpdir))
            versions = vc2.list_versions("m")
            assert len(versions) == 1
            assert versions[0].checksum_sha256 == "abc"

    def test_version_control_list_empty_model(self):
        from ironvault.version.control import VersionControl
        with tempfile.TemporaryDirectory() as tmpdir:
            vc = VersionControl(Path(tmpdir))
            assert vc.list_versions("none") == []

    def test_version_control_cleanup(self):
        from ironvault.version.control import VersionControl
        with tempfile.TemporaryDirectory() as tmpdir:
            vc = VersionControl(Path(tmpdir))
            for i in range(5):
                vc.add_version("m", f"/f{i}", "pt", 1, 1, "a")
            removed = vc.cleanup_old_versions("m", keep_count=2)
            assert len(removed) == 3
            assert len(vc.list_versions("m")) == 2


# ---------------------------------------------------------------------------
# FIPS crypto extended tests 
# ---------------------------------------------------------------------------

class TestFIPSCryptoExtended:
    """Extended tests for FIPS crypto module."""

    def test_generate_passphrase_default_length(self):
        from ironvault.crypto.fips import FIPSCrypto
        pp = FIPSCrypto.generate_passphrase()
        assert len(pp) == 64  # 32 bytes = 64 hex chars

    def test_generate_passphrase_custom_length(self):
        from ironvault.crypto.fips import FIPSCrypto
        pp = FIPSCrypto.generate_passphrase(16)
        assert len(pp) == 32

    def test_secure_compare_equal(self):
        from ironvault.crypto.fips import FIPSCrypto
        assert FIPSCrypto.secure_compare(b"abc", b"abc") is True

    def test_secure_compare_different(self):
        from ironvault.crypto.fips import FIPSCrypto
        assert FIPSCrypto.secure_compare(b"abc", b"xyz") is False

    def test_secure_compare_different_length(self):
        from ironvault.crypto.fips import FIPSCrypto
        assert FIPSCrypto.secure_compare(b"ab", b"abc") is False

    def test_encrypt_bad_key_size(self):
        from ironvault.crypto.fips import FIPSCrypto
        crypto = FIPSCrypto()
        with pytest.raises(ValueError, match="Key must be"):
            crypto.encrypt(b"data", b"short_key")

    def test_decrypt_bad_key_size(self):
        from ironvault.crypto.fips import FIPSCrypto
        crypto = FIPSCrypto()
        with pytest.raises(ValueError, match="Key must be"):
            crypto.decrypt(b"\x00" * 28, b"short_key")

    def test_decrypt_tampered_data(self):
        from ironvault.crypto.fips import FIPSCrypto
        from cryptography.exceptions import InvalidTag
        crypto = FIPSCrypto()
        key, salt = crypto.generate_key(b"testpass")
        encrypted = crypto.encrypt(b"secret data", key)
        # Tamper with ciphertext
        tampered = bytearray(encrypted)
        tampered[-1] ^= 0xFF
        with pytest.raises(InvalidTag):
            crypto.decrypt(bytes(tampered), key)

    def test_key_derivation_with_explicit_salt(self):
        from ironvault.crypto.fips import FIPSCrypto
        crypto = FIPSCrypto()
        salt = b"\x00" * 32
        key1, _ = crypto.generate_key(b"pass", salt)
        key2, _ = crypto.generate_key(b"pass", salt)
        assert key1 == key2  # Same salt + pass = same key
