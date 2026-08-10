"""
IronVault - Python interface to the IronVault.

This module provides a Python-friendly API wrapping the Rust `iv` CLI.
For production use, this would use PyO3/maturin FFI bindings to the Rust library.
Currently delegates to the compiled Rust binary for all cryptographic operations.
"""

import json
import os
import subprocess
from pathlib import Path
from typing import Any, Dict, List, Optional

from ironvault.core.config import VaultConfig


class Vault:
    """
    Secure IronVault with FIPS 140-3 encryption.

    Wraps the Rust `iv` CLI binary. All cryptographic operations
    (AES-256-GCM, Argon2id KDF) are performed by the Rust implementation.

    Example:
        >>> vault = Vault("/path/to/vault")
        >>> vault.store("my_model", "/path/to/model.safetensors")
        >>> vault.list_models()
        ['my_model']
    """

    def __init__(
        self,
        vault_path: Optional[str] = None,
        config: Optional[VaultConfig] = None,
    ) -> None:
        """
        Initialize a vault interface.

        Args:
            vault_path: Path to the vault directory. Defaults to XDG data dir.
            config: Optional VaultConfig for directory customization.
        """
        self._config = config or VaultConfig()
        self._vault_path = Path(vault_path) if vault_path else self._config.data_dir / "vault"
        self._vault_path.mkdir(parents=True, exist_ok=True)

    @property
    def path(self) -> Path:
        """Return the vault directory path."""
        return self._vault_path

    def _run_aim(self, args: List[str], passphrase: Optional[str] = None) -> subprocess.CompletedProcess:
        """
        Run the `iv` CLI binary with given arguments.

        Args:
            args: CLI arguments to pass.
            passphrase: Optional passphrase (set via IRONVAULT_PASSPHRASE env var).

        Returns:
            CompletedProcess result.

        Raises:
            FileNotFoundError: If `iv` binary is not on PATH.
            RuntimeError: If command exits with non-zero status.
        """
        env = os.environ.copy()
        env["IRONVAULT_VAULT_PATH"] = str(self._vault_path)
        if passphrase:
            env["IRONVAULT_PASSPHRASE"] = passphrase

        try:
            result = subprocess.run(
                ["iv"] + args,
                capture_output=True,
                text=True,
                env=env,
                timeout=300,
            )
        except FileNotFoundError:
            raise FileNotFoundError(
                "The 'iv' binary was not found. Build with: cargo build --release"
            )

        if result.returncode != 0:
            raise RuntimeError(f"iv command failed: {result.stderr.strip()}")

        return result

    def store(
        self,
        name: str,
        model_path: str,
        passphrase: Optional[str] = None,
        description: Optional[str] = None,
    ) -> None:
        """
        Store a model in the vault with encryption.

        Args:
            name: Name/identifier for the model.
            model_path: Path to the model file.
            passphrase: Encryption passphrase.
            description: Optional model description.
        """
        args = ["store", name, model_path]
        if description:
            args.extend(["--description", description])
        self._run_aim(args, passphrase=passphrase)

    def retrieve(
        self,
        name: str,
        output_path: str,
        passphrase: Optional[str] = None,
        version: Optional[int] = None,
    ) -> None:
        """
        Retrieve and decrypt a model from the vault.

        Args:
            name: Model name/identifier.
            output_path: Where to write the decrypted model.
            passphrase: Decryption passphrase.
            version: Optional specific version to retrieve.
        """
        args = ["retrieve", name, output_path]
        if version is not None:
            args.extend(["--version", str(version)])
        self._run_aim(args, passphrase=passphrase)

    def list_models(self) -> List[str]:
        """
        List all models in the vault.

        Returns:
            List of model names.
        """
        result = self._run_aim(["list"])
        lines = result.stdout.strip().split("\n")
        # Filter out header/decoration lines
        models = [line.strip() for line in lines if line.strip() and not line.startswith("─")]
        return models

    def delete(self, name: str) -> None:
        """
        Delete a model from the vault.

        Args:
            name: Model name to delete.
        """
        self._run_aim(["delete", name])

    def info(self, name: str) -> Dict[str, Any]:
        """
        Get information about a stored model.

        Args:
            name: Model name.

        Returns:
            Dictionary with model metadata.
        """
        result = self._run_aim(["info", name])
        # Parse the output as key-value pairs
        info: Dict[str, Any] = {}
        for line in result.stdout.strip().split("\n"):
            if ":" in line:
                key, _, value = line.partition(":")
                info[key.strip()] = value.strip()
        return info

    def verify(self, name: str, passphrase: Optional[str] = None) -> bool:
        """
        Verify integrity of a stored model.

        Args:
            name: Model name.
            passphrase: Decryption passphrase.

        Returns:
            True if integrity check passes.
        """
        try:
            self._run_aim(["verify", name], passphrase=passphrase)
            return True
        except RuntimeError:
            return False
