"""
IronVault - Universal secure vault for AI model formats

Native Rust bindings via PyO3. Falls back to pure-Python CLI wrappers
if the native extension is not available (e.g. source installs without Rust).
"""

__version__ = "5.1.1"

try:
    # Native Rust bindings (installed via maturin)
    from ironvault._native import (  # type: ignore[attr-defined]
        ModelCard,
        ModelFormat,
        ModelMetadata,
        ModelStream,
        ModelVersion,
        Vault,
        VaultBuilder,
        VaultConfig,
        sha256_hex,
        version as rust_version,
    )

    _NATIVE = True
except ImportError:
    # Fallback to pure-Python CLI wrappers
    from ironvault.core.vault import Vault  # type: ignore[assignment]
    from ironvault.core.config import VaultConfig  # type: ignore[assignment]
    from ironvault.formats.registry import ModelFormat  # type: ignore[assignment]

    ModelMetadata = None  # type: ignore[assignment,misc]
    ModelStream = None  # type: ignore[assignment,misc]
    ModelVersion = None  # type: ignore[assignment,misc]
    ModelCard = None  # type: ignore[assignment,misc]
    VaultBuilder = None  # type: ignore[assignment,misc]
    sha256_hex = None  # type: ignore[assignment]
    rust_version = None  # type: ignore[assignment]
    _NATIVE = False

__all__ = [
    "Vault",
    "VaultBuilder",
    "VaultConfig",
    "ModelFormat",
    "ModelMetadata",
    "ModelStream",
    "ModelVersion",
    "ModelCard",
    "sha256_hex",
    "version",
    "_NATIVE",
]


def version() -> str:
    """Return the native Rust library version, or the Python package version."""
    if rust_version is not None:
        return rust_version()
    return __version__
